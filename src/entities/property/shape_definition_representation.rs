//! `SHAPE_DEFINITION_REPRESENTATION` handler — Pass 6-5.
//!
//! Reader classifies each product as `Solid` / `SurfaceBody` / `Wireframe`
//! / `Group` based on the resolved shape representation. NAUO-tagged SDRs
//! ("Placement of an item") fall through to Phase B and are skipped here.
//! Writer emits the bare two-attr SDR linking a `PRODUCT_DEFINITION_SHAPE`
//! and the inner shape representation.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::assembly::ProductContent;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct ShapeDefinitionRepresentationWriteInput {
    pub(crate) pdef_shape: u64,
    pub(crate) shape_rep: u64,
}

pub(crate) struct ShapeDefinitionRepresentationHandler;

impl SimpleEntityHandler for ShapeDefinitionRepresentationHandler {
    const NAME: &'static str = "SHAPE_DEFINITION_REPRESENTATION";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6Sdr;
    type WriteInput = ShapeDefinitionRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SHAPE_DEFINITION_REPRESENTATION")?;
        let pdef_shape_ref = read_entity_ref(attrs, 0, entity_id, "definition")?;
        let shape_rep_ref = read_entity_ref(attrs, 1, entity_id, "used_representation")?;

        // Only consider SDRs where `pdef_shape.definition` is a
        // PRODUCT_DEFINITION. NAUO-tagged SDRs fall through to Phase B.
        let Some(pdef_ref) = ctx.pdef_shape_to_pdef.get(&pdef_shape_ref).copied() else {
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

        // Guard against a second SDR pinning the same product: keep the
        // first classification and warn on the duplicate.
        match &ctx.assembly_products[pid].content {
            ProductContent::Solid(_)
            | ProductContent::SurfaceBody(_)
            | ProductContent::Wireframe(_) => {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "duplicate SHAPE_DEFINITION_REPRESENTATION for product #{product_step_id}"
                    ),
                });
                return Ok(());
            }
            ProductContent::Group(instances) if !instances.is_empty() => {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "duplicate SHAPE_DEFINITION_REPRESENTATION for product #{product_step_id}"
                    ),
                });
                return Ok(());
            }
            ProductContent::Group(_) => {}
        }

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
            ctx.assembly_products[pid].content = ProductContent::Solid(solid_ids);
        } else if let Some(shells) = ctx.mssr_shells_map.get(&effective_ref).cloned() {
            if shells.is_empty() {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "MSSR #{effective_ref} has no shells — product content left empty"
                    ),
                });
            } else {
                ctx.assembly_products[pid].content = ProductContent::SurfaceBody(shells);
            }
        } else if let Some(wf) = ctx.wireframe_data_map.get(&effective_ref).cloned() {
            ctx.assembly_products[pid].content = ProductContent::Wireframe(wf);
        }
        if let Some(&ref_frame) = ctx.absr_ref_frame_map.get(&effective_ref) {
            ctx.assembly_products[pid].shape_ref_frame = ref_frame;
        } else if let Some(&ref_frame) = ctx.mssr_ref_frame_map.get(&effective_ref) {
            ctx.assembly_products[pid].shape_ref_frame = ref_frame;
        } else if let Some(&ref_frame) = ctx.wireframe_ref_frame_map.get(&effective_ref) {
            ctx.assembly_products[pid].shape_ref_frame = ref_frame;
        }
        // Attach the unit / uncertainty context referenced by this product's
        // inner shape representation. Look up by the resolved ABSR/MSSR/etc.
        // entity, not the outer plain SR — the inner ctx is the geometry-side
        // one that downstream tooling cares about.
        if let Some(&ctx_id) = ctx.repr_context_map.get(&effective_ref) {
            ctx.assembly_products[pid].geometry_context = Some(ctx_id);
        }
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static SDR_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ShapeDefinitionRepresentationHandler::NAME,
    pass_level: ShapeDefinitionRepresentationHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ShapeDefinitionRepresentationHandler::read,
    },
};
