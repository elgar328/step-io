//! `SHAPE_DEFINITION_REPRESENTATION` handler.
//!
//! Reader classifies each product as `Solid` / `SurfaceBody` / `Wireframe`
//! / `Group` based on the resolved shape representation. NAUO-tagged SDRs
//! ("Placement of an item") fall through to Phase B and are skipped here.
//! Writer emits the bare two-attr SDR linking a `PRODUCT_DEFINITION_SHAPE`
//! and the inner shape representation.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::{GeometryLeaf, SolidContent, SurfaceBodyContent};
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ShapeDefinitionRepresentationWriteInput {
    pub(crate) pdef_shape: u64,
    pub(crate) shape_rep: u64,
}

pub(crate) struct ShapeDefinitionRepresentationHandler;

#[step_entity(name = "SHAPE_DEFINITION_REPRESENTATION")]
impl SimpleEntityHandler for ShapeDefinitionRepresentationHandler {
    type WriteInput = ShapeDefinitionRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SHAPE_DEFINITION_REPRESENTATION")?;
        let pdef_shape_ref = read_entity_ref(attrs, 0, entity_id, "definition")?;
        let shape_rep_ref = read_entity_ref(attrs, 1, entity_id, "used_representation")?;

        // Only consider SDRs where `pdef_shape.definition` is a
        // PRODUCT_DEFINITION. Others (definition = a PROPERTY_DEFINITION, e.g.
        // geometric-validation / CATIA geometric-set PMI shapes) are stashed
        // raw and resolved after all entities are read (the PD is read by the
        // property handler, later than this SDR). NAUO-tagged PDS resolve to neither and
        // stay dropped (assembly-instance shape — a later phase).
        let Some(pdef_ref) = ctx.pdef_shape_to_pdef.get(&pdef_shape_ref).copied() else {
            ctx.sdr_link_refs.push((pdef_shape_ref, shape_rep_ref));
            return Ok(());
        };
        let Some(&product_step_id) = ctx.pdef_to_product.get(&pdef_ref) else {
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: pdef_ref,
                field_name: "definition.definition",
            });
        };
        let Some(&pid) = ctx.product_arena_map.get(&product_step_id) else {
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: product_step_id,
                field_name: "definition.product",
            });
        };

        // Defer the geometry classification: it follows the indirect chain
        // `SDR -> plain SR -> SHAPE_REPRESENTATION_RELATIONSHIP -> geometry rep`
        // through maps (`srr_equiv_map`, `wireframe_data_map`, ...) that this
        // SDR does not reference, so under topological dispatch they may not be
        // populated yet. `resolve_sdr_product_geometry` runs the body below
        // once every relationship and geometry representation has been read.
        ctx.pending_sdr_geometry
            .push(crate::reader::PendingSdrGeometry {
                pid,
                shape_rep_ref,
                entity_id,
            });
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ShapeDefinitionRepresentationWriteInput {
            pdef_shape,
            shape_rep,
        }: ShapeDefinitionRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "SHAPE_DEFINITION_REPRESENTATION",
            vec![
                Attribute::EntityRef(pdef_shape),
                Attribute::EntityRef(shape_rep),
            ],
        ))
    }
}

impl ReaderContext {
    /// Classify each deferred SDR's product as `Solid` / `SurfaceBody` /
    /// `Wireframe` (or leave it a `Group`). Runs after the full dispatch so the
    /// indirection maps are complete, and before `resolve_nauo_instances` so
    /// the duplicate-claim guard observes the same (instance-free) product
    /// state it did under the legacy SDR-before-NAUO ordering.
    pub(crate) fn resolve_sdr_product_geometry(&mut self) {
        let ctx = self;
        for pending in std::mem::take(&mut ctx.pending_sdr_geometry) {
            let PendingSdrGeometry {
                pid,
                shape_rep_ref,
                entity_id,
            } = pending;

            // Guard against a second SDR pinning the same product: keep the
            // first classification and warn on the duplicate. A product that
            // already has geometry assigned, or that has accumulated child
            // instances, has been "claimed" — a second SDR is a no-op + warning.
            let product = &ctx.assembly_products[pid];
            if product.geometry.is_some() || !product.instances.is_empty() {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "duplicate SHAPE_DEFINITION_REPRESENTATION for product (pid {})",
                        pid.0
                    ),
                });
                continue;
            }

            classify_sdr_geometry(ctx, pid, shape_rep_ref, entity_id);
        }
    }
}

use crate::ir::id::ProductId;
use crate::reader::PendingSdrGeometry;

/// The deferred SDR product-geometry classification body. Resolves the
/// indirect SR chain and attaches geometry / reference frame / context.
fn classify_sdr_geometry(
    ctx: &mut ReaderContext,
    pid: ProductId,
    shape_rep_ref: u64,
    entity_id: u64,
) {
    // Follow the Fusion 360 / CATIA indirection:
    //   SDR → plain SR → SHAPE_REPRESENTATION_RELATIONSHIP → ABSR / MSSR
    // Direct references fall through the map untouched.
    let effective_ref = ctx
        .srr_equiv_map
        .get(&shape_rep_ref)
        .copied()
        .unwrap_or(shape_rep_ref);

    // When the indirection chain was taken, attach the plain SR's frame
    // so the writer can re-emit the wrapper. `plain_sr_frame_map` may be
    // missing the entry if the plain SR had no axis item — in that case
    // outer_sr_frame stays None and the writer emits the direct form.
    if effective_ref != shape_rep_ref
        && let Some(&plain_frame) = ctx.plain_sr_frame_map.get(&shape_rep_ref)
    {
        ctx.assembly_products[pid].outer_sr_frame = Some(plain_frame);
    }

    let in_absr = ctx.absr_solid_map.contains_key(&effective_ref);
    let in_mssr = ctx.mssr_shells_map.contains_key(&effective_ref);
    if in_absr && in_mssr {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "shape_rep #{effective_ref} registered as both ABSR and MSSR — using ABSR"
            ),
        });
    }
    if let Some(solid_ids) = ctx.absr_solid_map.get(&effective_ref).cloned() {
        ctx.assembly_products[pid].geometry =
            Some(GeometryLeaf::Solid(SolidContent { ids: solid_ids }));
    } else if let Some(shells) = ctx.mssr_shells_map.get(&effective_ref).cloned() {
        if shells.is_empty() {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("MSSR #{effective_ref} has no shells — product content left empty"),
            });
        } else {
            ctx.assembly_products[pid].geometry =
                Some(GeometryLeaf::SurfaceBody(SurfaceBodyContent {
                    ids: shells,
                }));
        }
    } else if let Some(wf) = ctx.wireframe_data_map.get(&effective_ref).cloned() {
        ctx.assembly_products[pid].geometry = Some(GeometryLeaf::Wireframe(wf));
    }
    if let Some(&ref_frame) = ctx.absr_ref_frame_map.get(&effective_ref) {
        ctx.assembly_products[pid].shape_ref_frame = ref_frame;
    } else if let Some(&ref_frame) = ctx.mssr_ref_frame_map.get(&effective_ref) {
        ctx.assembly_products[pid].shape_ref_frame = ref_frame;
    } else if let Some(&ref_frame) = ctx.wireframe_ref_frame_map.get(&effective_ref) {
        ctx.assembly_products[pid].shape_ref_frame = ref_frame;
    } else if let Some(&ref_frame) = ctx.plain_sr_frame_map.get(&effective_ref) {
        // Plain SHAPE_REPRESENTATION with no underlying ABSR/MSSR/wireframe
        // (e.g. empty-Group products emitted by the writer). Without this
        // fallback their shape_ref_frame stays at the PRODUCT-pass
        // placeholder (Placement3dId(0)).
        ctx.assembly_products[pid].shape_ref_frame = ref_frame;
    }
    // Attach the unit / uncertainty context referenced by this product's
    // inner shape representation. Look up by the resolved ABSR/MSSR/etc.
    // entity, not the outer plain SR — the inner ctx is the geometry-side
    // one that downstream tooling cares about.
    if let Some(&ctx_id) = ctx.repr_context_map.get(&effective_ref) {
        ctx.assembly_products[pid].geometry_context = Some(ctx_id);
    }
    // representation-refactor: link the product to its unified Representation
    // arena entry. `representation_id` is the resolved geometry rep
    // (ABSR/MSSR/wireframe/plain). `outer_representation_id` is the outer
    // plain SR wrapper when the indirect pattern was taken — the writer
    // re-uses both cached step ids instead of re-emitting.
    ctx.assembly_products[pid].representation_id = ctx.repr_id_map.get(&effective_ref).copied();
    if effective_ref != shape_rep_ref {
        ctx.assembly_products[pid].outer_representation_id =
            ctx.repr_id_map.get(&shape_rep_ref).copied();
    }
}
