//! Assembly/product-domain `lower` fns (the product 2-hop cluster: product,
//! formation ± specified source, `product_definition` ± associated documents).
//! See the [module docs](super) for the lowering contract.
//!
//! Besides the arena work these record the typed `Early*Id` → `ProductId`
//! correspondence that consumers resolve in one probe (this replaced the raw
//! `formation_to_product` / `pdef_to_product` / `pdef_shape_to_pdef` step-id
//! chain maps).

use crate::early::model::{
    EarlyDesignContext, EarlyMakeFromUsageOption, EarlyMechanicalContext,
    EarlyNextAssemblyUsageOccurrence, EarlyProduct, EarlyProductCategory,
    EarlyProductCategoryRelationship, EarlyProductContext, EarlyProductDefinitionContext,
    EarlyProductDefinitionContextAssociation, EarlyProductDefinitionContextRole,
    EarlyProductDefinitionFormationId, EarlyProductDefinitionId,
    EarlyProductDefinitionRelationship, EarlyProductDefinitionShapeId,
    EarlyProductRelatedProductCategory, EarlySource,
};
use crate::ir::ApplicationContextId;
use crate::ir::assembly::{
    Product, ProductCategory, ProductCategoryRelationship, ProductContext, ProductContextData,
    ProductDefinition, ProductDefinitionContext, ProductDefinitionContextAssociation,
    ProductDefinitionContextData, ProductDefinitionContextRole, ProductDefinitionFormation,
    ProductDefinitionFormationData, ProductDefinitionFormationWithSpecifiedSource,
    ProductRelatedProductCategoryData,
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
        formation_with_source: false,
        geometry_context: None,
        product_context: None,
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

    // `formation` and `context` are both schema-required and resolve inline:
    // topo dispatch lowers the formation / PRODUCT_DEFINITION_CONTEXT handlers
    // (direct refs of this PD) first, so their typed ids are already cached. An
    // unresolved ref means a dangling/dropped target — drop the PD (cascade).
    let Some(formation) = ctx
        .id_cache
        .get::<crate::ir::id::ProductDefinitionFormationId>(formation_ref)
    else {
        return Err(ConvertError::MissingReference {
            from: entity_id,
            to: formation_ref,
            field_name: "formation",
        });
    };
    let Some(context) = ctx
        .id_cache
        .get::<crate::ir::id::ProductDefinitionContextId>(frame_of_reference)
    else {
        return Err(ConvertError::MissingReference {
            from: entity_id,
            to: frame_of_reference,
            field_name: "frame_of_reference",
        });
    };
    let pd_id = ctx.product_definitions.push(ProductDefinition {
        id,
        // Legacy read_string_or_unset collapsed `$` to "" (L2 keeps a String).
        description: description.unwrap_or_default(),
        formation,
        context,
        documentation_ids: docs.clone(),
    });
    ctx.id_cache.insert(entity_id, pd_id);
    ctx.assembly_products[pid].pdef = Some(pd_id);
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

/// Lower one `NEXT_ASSEMBLY_USAGE_OCCURRENCE`: resolve the parent / child
/// PDEF endpoints and defer the instance to the `resolve_nauo_instances`
/// post-pass (the transform comes from the CDSR handler, which topological
/// dispatch places *after* the NAUO; a NAUO with no transform leaves neither
/// an `Instance` nor an arena entry). No `record_lowered`: the canonical
/// `assembly_component_usage` arena entry only exists after the post-pass.
pub(crate) fn lower_next_assembly_usage_occurrence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyNextAssemblyUsageOccurrence,
) -> Result<(), ConvertError> {
    let relating_pdef = early.relating_product_definition;
    let related_pdef = early.related_product_definition;
    let parent_pid = ctx.resolve_product_by_pdef(entity_id, relating_pdef, "relating_pdef")?;
    let child_pid = ctx.resolve_product_by_pdef(entity_id, related_pdef, "related_pdef")?;
    // Canonical NAUO endpoints are PRODUCT_DEFINITION refs — resolve them to
    // `product_definitions` arena ids (the `Instance` view keeps the
    // ProductId child above). Missing = drop, mirroring resolve_product_by_pdef.
    let (Some(relating_pd), Some(related_pd)) = (
        ctx.id_cache
            .get::<crate::ir::id::ProductDefinitionId>(relating_pdef),
        ctx.id_cache
            .get::<crate::ir::id::ProductDefinitionId>(related_pdef),
    ) else {
        return Err(ConvertError::MissingReference {
            from: entity_id,
            to: relating_pdef,
            field_name: "relating/related_product_definition",
        });
    };

    ctx.pending_nauo_instances
        .push(crate::reader::PendingNauoInstance {
            parent: parent_pid,
            child: child_pid,
            relating_pd,
            related_pd,
            occurrence_id: early.id,
            occurrence_name: early.name,
            // Legacy read_string_or_unset collapsed `$` to "" (the canonical
            // arena entry keeps a String).
            description: early.description.unwrap_or_default(),
            reference_designator: early.reference_designator,
            nauo_id: entity_id,
        });
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

/// Lower one `PRODUCT_CATEGORY` (`Itself` carrier — the legacy read
/// collapsed an empty description to `None`, reproduced by the filter).
pub(crate) fn lower_product_category(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyProductCategory,
) {
    let pc_id = ctx
        .product_categories
        .push(crate::ir::assembly::ProductCategory::Itself(
            crate::ir::assembly::ProductCategoryData {
                name: early.name,
                description: early.description.filter(|d| !d.is_empty()),
            },
        ));
    ctx.pc_arena_map.insert(entity_id, pc_id);
}

/// Lower one `PRODUCT_RELATED_PRODUCT_CATEGORY` into the shared
/// `product_categories` arena. Empty `products` (non-standard SET[1:?]) is a
/// `NsCase::EmptyPrrpc` drop that records `empty_prrpc_refs` so a referencing
/// `PRODUCT_CATEGORY_RELATIONSHIP` cascades. Unknown product refs are filtered
/// (verbatim port of the legacy read). `description` empty→None mirrors the
/// `optional_text` reader / the `Itself` sibling.
pub(crate) fn lower_product_related_product_category(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyProductRelatedProductCategory,
) {
    if early.products.is_empty() {
        ctx.ns_push(
            crate::reader::NsCase::EmptyPrrpc,
            "PRODUCT_RELATED_PRODUCT_CATEGORY".into(),
            1,
            "dropped (empty products, non-standard SET[1:?])".into(),
        );
        ctx.empty_prrpc_refs.insert(entity_id);
        return;
    }
    let mut products = Vec::with_capacity(early.products.len());
    for prod_ref in &early.products {
        if let Some(pid) = ctx.id_cache.get::<ProductId>(*prod_ref) {
            products.push(pid);
        }
    }
    let pc_id = ctx
        .product_categories
        .push(ProductCategory::ProductRelatedProductCategory(
            ProductRelatedProductCategoryData {
                name: early.name,
                description: early.description.filter(|d| !d.is_empty()),
                products,
            },
        ));
    ctx.prpc_arena_map.insert(entity_id, pc_id);
}

/// Lower one `PRODUCT_CATEGORY_RELATIONSHIP`. A `sub_category` that is a
/// dropped empty PRRPC cascades (`NsCase::EmptyPrrpcCascade` drop). `category`
/// resolves through `pc_arena_map` then `prpc_arena_map` (a PRPC is a valid
/// `product_category`); `sub_category` through `prpc_arena_map`. Either
/// unresolved → `MissingReference` drop. `name`/`description` are kept faithful
/// (L1-strict — the legacy writer discarded them to `" "`).
pub(crate) fn lower_product_category_relationship(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyProductCategoryRelationship,
) {
    if ctx.empty_prrpc_refs.contains(&early.sub_category) {
        ctx.ns_push(
            crate::reader::NsCase::EmptyPrrpcCascade,
            "PRODUCT_CATEGORY_RELATIONSHIP".into(),
            1,
            "dropped (relates empty PRRPC)".into(),
        );
        return;
    }
    let (Some(&category), Some(&sub_category)) = (
        ctx.pc_arena_map
            .get(&early.category)
            .or_else(|| ctx.prpc_arena_map.get(&early.category)),
        ctx.prpc_arena_map.get(&early.sub_category),
    ) else {
        ctx.warnings.push(ConvertError::MissingReference {
            from: entity_id,
            to: if ctx.prpc_arena_map.contains_key(&early.sub_category) {
                early.category
            } else {
                early.sub_category
            },
            field_name: "category",
        });
        return;
    };
    ctx.product_category_relationships
        .push(ProductCategoryRelationship {
            name: early.name,
            description: early.description,
            category,
            sub_category,
        });
}

/// Lower one `MAKE_FROM_USAGE_OPTION` (faithful optional description; an
/// unsupported MWU subtype quantity drops silently like other MWU
/// consumers; unresolved pdefs surface `MissingReference`).
pub(crate) fn lower_make_from_usage_option(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyMakeFromUsageOption,
) -> Result<(), ConvertError> {
    let relating = ctx.resolve_product_by_pdef(
        entity_id,
        early.relating_product_definition,
        "relating_product_definition",
    )?;
    let related = ctx.resolve_product_by_pdef(
        entity_id,
        early.related_product_definition,
        "related_product_definition",
    )?;
    let Some(quantity) = ctx
        .id_cache
        .get::<crate::ir::id::MeasureWithUnitId>(early.quantity)
    else {
        return Ok(());
    };
    let arena_id = ctx.product_definition_relationships.push(
        crate::ir::assembly::ProductDefinitionRelationship::MakeFrom(
            crate::ir::assembly::MakeFromUsageOption {
                id: early.id,
                name: early.name,
                description: early.description,
                relating,
                related,
                ranking: early.ranking,
                ranking_rationale: early.ranking_rationale,
                quantity,
            },
        ),
    );
    ctx.id_cache.insert(entity_id, arena_id);
    Ok(())
}

/// Lower one plain `PRODUCT_DEFINITION_RELATIONSHIP` (faithful optional
/// description; unresolved pdefs surface `MissingReference`).
pub(crate) fn lower_product_definition_relationship(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyProductDefinitionRelationship,
) -> Result<(), ConvertError> {
    let relating = ctx.resolve_product_by_pdef(
        entity_id,
        early.relating_product_definition,
        "relating_product_definition",
    )?;
    let related = ctx.resolve_product_by_pdef(
        entity_id,
        early.related_product_definition,
        "related_product_definition",
    )?;
    let arena_id = ctx.product_definition_relationships.push(
        crate::ir::assembly::ProductDefinitionRelationship::Plain(
            crate::ir::assembly::PlainProductDefinitionRelationship {
                id: early.id,
                name: early.name,
                description: early.description,
                relating,
                related,
            },
        ),
    );
    ctx.id_cache.insert(entity_id, arena_id);
    Ok(())
}

/// Shared lowering for `PRODUCT_CONTEXT` / `MECHANICAL_CONTEXT` (same
/// `(name, frame_of_reference, discipline_type)` body, distinguished by the
/// L2 enum variant). `frame_of_reference` resolves to an `ApplicationContextId`
/// (an unmapped `APPLICATION_CONTEXT` drops the entity, matching the legacy
/// read); registers the `id_cache` key so a `PRODUCT_DEFINITION_CONTEXT_
/// ASSOCIATION` etc. can resolve it.
fn lower_product_context_kind(
    ctx: &mut ReaderContext,
    entity_id: u64,
    name: String,
    frame_ref: u64,
    discipline_type: String,
    variant: fn(ProductContextData) -> ProductContext,
) {
    let Some(frame_of_reference) = ctx.id_cache.get::<ApplicationContextId>(frame_ref) else {
        return;
    };
    let id = ctx.product_contexts.push(variant(ProductContextData {
        name,
        frame_of_reference,
        discipline_type,
    }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `PRODUCT_CONTEXT` (`ProductContext::Itself`).
pub(crate) fn lower_product_context(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyProductContext,
) {
    lower_product_context_kind(
        ctx,
        entity_id,
        early.name,
        early.frame_of_reference,
        early.discipline_type,
        ProductContext::Itself,
    );
}

/// Lower `MECHANICAL_CONTEXT` (`ProductContext::Mechanical`).
pub(crate) fn lower_mechanical_context(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyMechanicalContext,
) {
    lower_product_context_kind(
        ctx,
        entity_id,
        early.name,
        early.frame_of_reference,
        early.discipline_type,
        ProductContext::Mechanical,
    );
}

/// Shared lowering for `PRODUCT_DEFINITION_CONTEXT` / `DESIGN_CONTEXT` (same
/// `(name, frame_of_reference, life_cycle_stage)` body, distinguished by the
/// L2 enum variant).
fn lower_product_definition_context_kind(
    ctx: &mut ReaderContext,
    entity_id: u64,
    name: String,
    frame_ref: u64,
    life_cycle_stage: String,
    variant: fn(ProductDefinitionContextData) -> ProductDefinitionContext,
) {
    let Some(frame_of_reference) = ctx.id_cache.get::<ApplicationContextId>(frame_ref) else {
        return;
    };
    let id = ctx
        .product_definition_contexts
        .push(variant(ProductDefinitionContextData {
            name,
            frame_of_reference,
            life_cycle_stage,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `PRODUCT_DEFINITION_CONTEXT` (`ProductDefinitionContext::Itself`).
pub(crate) fn lower_product_definition_context(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyProductDefinitionContext,
) {
    lower_product_definition_context_kind(
        ctx,
        entity_id,
        early.name,
        early.frame_of_reference,
        early.life_cycle_stage,
        ProductDefinitionContext::Itself,
    );
}

/// Lower `DESIGN_CONTEXT` (`ProductDefinitionContext::Design`).
pub(crate) fn lower_design_context(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDesignContext,
) {
    lower_product_definition_context_kind(
        ctx,
        entity_id,
        early.name,
        early.frame_of_reference,
        early.life_cycle_stage,
        ProductDefinitionContext::Design,
    );
}

/// Lower `PRODUCT_DEFINITION_CONTEXT_ROLE` (leaf). Registers the `id_cache` key
/// so a `PRODUCT_DEFINITION_CONTEXT_ASSOCIATION` can resolve its `role` ref.
pub(crate) fn lower_product_definition_context_role(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyProductDefinitionContextRole,
) {
    let id = ctx
        .product_definition_context_roles
        .push(ProductDefinitionContextRole {
            name: early.name,
            description: early.description,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `PRODUCT_DEFINITION_CONTEXT_ASSOCIATION`. `definition` refs a
/// `PRODUCT_DEFINITION` → resolved to its `ProductId` (PDEF data lives on
/// Product); `frame_of_reference`/`role` resolve through `id_cache`. Any
/// unresolved member drops the entry (matching the legacy read).
pub(crate) fn lower_product_definition_context_association(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyProductDefinitionContextAssociation,
) {
    let Some(definition) = ctx.product_of_pdef(early.definition) else {
        return;
    };
    let Some(frame_of_reference) = ctx
        .id_cache
        .get::<crate::ir::id::ProductDefinitionContextId>(early.frame_of_reference)
    else {
        return;
    };
    let Some(role) = ctx
        .id_cache
        .get::<crate::ir::id::ProductDefinitionContextRoleId>(early.role)
    else {
        return;
    };
    let id =
        ctx.product_definition_context_associations
            .push(ProductDefinitionContextAssociation {
                definition,
                frame_of_reference,
                role,
            });
    ctx.id_cache.insert(entity_id, id);
}
