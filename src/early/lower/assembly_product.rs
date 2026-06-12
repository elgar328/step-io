//! Assembly/product-domain `lower` fns (the product 2-hop cluster: product,
//! formation ± specified source, `product_definition` ± associated documents).
//! See the [module docs](super) for the lowering contract.
//!
//! Besides the arena work these record the typed `Early*Id` → `ProductId`
//! correspondence that consumers resolve in one probe (replacing the raw
//! `formation_to_product` / `pdef_to_product` 2-hop step-id maps — kept
//! dual-written until every consumer migrates).

use crate::early::model::{
    EarlyProduct, EarlyProductDefinitionFormationId, EarlyProductDefinitionId,
    EarlyProductDefinitionShapeId, EarlySource,
};
use crate::ir::assembly::{
    Product, ProductDefinition, ProductDefinitionFormation, ProductDefinitionFormationData,
    ProductDefinitionFormationWithSpecifiedSource,
};
use crate::ir::error::ConvertError;
use crate::ir::id::{Placement3dId, ProductId};
use crate::reader::ReaderContext;

/// The Part21 enum token for an L1 `source` value — what the legacy
/// `read_enum` handed the L2 `String` field.
fn source_token(s: EarlySource) -> &'static str {
    match s {
        EarlySource::Made => "MADE",
        EarlySource::Bought => "BOUGHT",
        EarlySource::NotKnown => "NOT_KNOWN",
    }
}

/// Lower one `PRODUCT`: push the `Product` shell (geometry/category filled by
/// later sub-passes) and capture the first `frame_of_reference` ref for the
/// `resolve_product_contexts` post-pass.
pub(crate) fn lower_product(ctx: &mut ReaderContext, entity_id: u64, early: EarlyProduct) {
    // Legacy handler collapsed an empty description to `None` (`$` and `""`
    // both); L1 is faithfully optional, so only the empty-string collapse
    // remains here.
    let description = early.description.filter(|d| !d.is_empty());
    // Corpus is consistently a single-element SET; the post-pass resolves it.
    let pc_step_ref = early.frame_of_reference.first().copied();

    // Every Product needs a non-optional reference frame. SDR conversion
    // overwrites this with the shape representation's actual refFrame when
    // available; the "no AXIS2 at all" degenerate case is resolved in the
    // `ensure_product_ref_frames` post-pass (dispatch-order independent).
    let product = Product {
        id: early.id,
        name: early.name,
        description,
        geometry: None,
        instances: Vec::new(),
        shape_ref_frame: Placement3dId(0),
        outer_sr_frame: None,
        category: None,
        formation_with_source: false,
        geometry_context: None,
        product_context: None,
        pdef_context: None,
        representation_id: None,
        outer_representation_id: None,
        associated_documents: Vec::new(),
        formation: None,
        pdef: None,
    };
    let pid = ctx.assembly_products.push(product);
    ctx.id_cache.insert(entity_id, pid);
    if let Some(r) = pc_step_ref {
        ctx.product_pc_step_refs.insert(pid, r);
    }
}

/// Lower one `PRODUCT_DEFINITION_FORMATION` (or its `_WITH_SPECIFIED_SOURCE`
/// subtype when `make_or_buy` is `Some`): store the formation in the carrier
/// arena and link it onto the product. If the product hasn't been read
/// (malformed input), keep the chain map but skip the arena entry — the writer
/// then synthesises, matching the legacy leniency.
pub(crate) fn lower_product_definition_formation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    id: String,
    description: Option<String>,
    of_product: u64,
    make_or_buy: Option<EarlySource>,
) {
    let Some(pid) = ctx.id_cache.get::<ProductId>(of_product) else {
        return;
    };
    let data = ProductDefinitionFormationData {
        id,
        // Legacy read_string_or_unset collapsed `$` to "" (L2 keeps a String).
        description: description.unwrap_or_default(),
        of_product: pid,
    };
    let formation = if let Some(source) = make_or_buy {
        ctx.assembly_products[pid].formation_with_source = true;
        ProductDefinitionFormation::WithSpecifiedSource(
            ProductDefinitionFormationWithSpecifiedSource {
                inherited: data,
                make_or_buy: source_token(source).to_owned(),
            },
        )
    } else {
        ProductDefinitionFormation::Itself(data)
    };
    let fid = ctx.product_definition_formations.push(formation);
    ctx.id_cache.insert(entity_id, fid);
    ctx.assembly_products[pid].formation = Some(fid);

    // Typed one-probe correspondence: formation file id → ProductId (what the
    // 2-hop `formation_to_product` chain ultimately resolved to).
    let early_id: EarlyProductDefinitionFormationId = ctx.early.record_lowered(pid);
    ctx.id_cache.insert(entity_id, early_id);
}

/// Lower one `PRODUCT_DEFINITION` (or its `_WITH_ASSOCIATED_DOCUMENTS` subtype
/// when `documentation_ids` is `Some`): materialize the canonical arena entry,
/// link it onto the product, and resolve the document refs (unresolved ones
/// are surfaced as warnings and skipped — legacy leniency).
pub(crate) fn lower_product_definition(
    ctx: &mut ReaderContext,
    entity_id: u64,
    id: String,
    description: Option<String>,
    formation_ref: u64,
    frame_of_reference: u64,
    documentation_ids: Option<Vec<u64>>,
) -> Result<(), ConvertError> {
    // One typed probe replaces the raw formation_to_product hop. `None` covers
    // both a missing formation and one whose product did not resolve (the
    // legacy chain then failed at the consumer instead).
    let Some(pid) = ctx.product_of_formation(formation_ref) else {
        return Err(ConvertError::MissingReference {
            from: entity_id,
            to: formation_ref,
            field_name: "formation",
        });
    };

    // Resolve the WAD document refs up front (the canonical entry carries
    // them; unresolved subtypes are surfaced and skipped).
    let mut docs = Vec::new();
    if let Some(doc_refs) = documentation_ids {
        docs.reserve(doc_refs.len());
        for r in doc_refs {
            if let Some(did) = ctx.id_cache.get::<crate::ir::DocumentId>(r) {
                docs.push(did);
            } else {
                ctx.warnings.push(ConvertError::MissingReference {
                    from: entity_id,
                    to: r,
                    field_name: "documentation_ids",
                });
            }
        }
    }

    // `formation` resolves inline (the formation handler ran first under topo
    // dispatch); `context` is filled later by `resolve_product_contexts` (the
    // context id maps populate after this pass — same deferral as
    // `Product.pdef_context`).
    let formation = ctx
        .id_cache
        .get::<crate::ir::id::ProductDefinitionFormationId>(formation_ref);
    let pd_id = ctx.product_definitions.push(ProductDefinition {
        id,
        // Legacy read_string_or_unset collapsed `$` to "" (L2 keeps a String).
        description: description.unwrap_or_default(),
        formation,
        context: None,
        documentation_ids: docs.clone(),
    });
    ctx.id_cache.insert(entity_id, pd_id);
    ctx.assembly_products[pid].pdef = Some(pd_id);
    ctx.product_pdc_step_refs.insert(pid, frame_of_reference);
    // Product view keeps `associated_documents` (the writer's plain-vs-WAD
    // discriminator) only when at least one doc resolved — an empty list
    // keeps the plain PRODUCT_DEFINITION output, matching the legacy reader.
    if !docs.is_empty() {
        ctx.assembly_products[pid].associated_documents = docs;
    }
    // Typed one-probe correspondence: pdef file id → ProductId (what the
    // 2-hop `pdef_to_product` chain ultimately resolved to).
    let early_id: EarlyProductDefinitionId = ctx.early.record_lowered(pid);
    ctx.id_cache.insert(entity_id, early_id);
    Ok(())
}

/// Lower one product-bearing `PRODUCT_DEFINITION_SHAPE` (the handler already
/// classified the `definition` target as a PDEF; the NAUO-bearing branch stays
/// in the handler). The L1 `name`/`description` are intentionally dropped —
/// the writer synthesises empty strings, the legacy behavior (faithful
/// round-trip of these is a separate, deliberate decision).
pub(crate) fn lower_product_definition_shape(
    ctx: &mut ReaderContext,
    entity_id: u64,
    pdef_ref: u64,
) {
    // Raw step-id chain map: dual-written until every consumer migrates to
    // the typed `EarlyProductDefinitionShapeId` probe.
    ctx.pdef_shape_to_pdef.insert(entity_id, pdef_ref);
    // Mirror the product-targeted PDS into the schema-faithful
    // `property_definitions` arena so the writer's arena-driven emit sees it.
    // The NAUO-targeted PDS is materialised later, in
    // `materialize_nauo_owned_pds`, once the ACU arena exists.
    if let Some(product_id) = ctx.product_of_pdef(pdef_ref) {
        let pd_id = ctx
            .properties
            .get_or_insert_with(crate::ir::property::PropertyPool::default)
            .property_definitions
            .push(
                crate::ir::property::PropertyDefinition::ProductDefinitionShape(
                    crate::ir::property::ProductDefinitionShape {
                        inherited: crate::ir::property::PropertyDefinitionData {
                            name: String::new(),
                            description: String::new(),
                            definition:
                                crate::ir::property::CharacterizedDefinition::ProductDefinition(
                                    product_id,
                                ),
                        },
                    },
                ),
            );
        ctx.id_cache.insert(entity_id, pd_id);
        // Typed one-probe correspondence: PDS file id → ProductId (what the
        // former `pdef_shape_to_pdef` → `product_of_pdef` chain resolved to).
        let early_id: EarlyProductDefinitionShapeId = ctx.early.record_lowered(product_id);
        ctx.id_cache.insert(entity_id, early_id);
    }
}
