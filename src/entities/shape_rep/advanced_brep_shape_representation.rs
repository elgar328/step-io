//! `ADVANCED_BREP_SHAPE_REPRESENTATION` handler — Pass 6-4a.
//!
//! Reader collects every `MANIFOLD_SOLID_BREP` item (multi-body STEP files
//! carry more than one) and binds the list to the ABSR id; the first
//! `AXIS2_PLACEMENT_3D` becomes the coordinate reference frame. Writer
//! emits the ABSR line with the per-product axis placement followed by all
//! solid refs the chain orchestrator pre-emitted, and the bound unit context.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::Product;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AdvancedBrepShapeRepresentationWriteInput {
    pub(crate) product: Product,
    pub(crate) solid_refs: Vec<u64>,
    pub(crate) unit_ctx: u64,
}

pub(crate) struct AdvancedBrepShapeRepresentationHandler;

#[step_entity(name = "ADVANCED_BREP_SHAPE_REPRESENTATION", pass = Pass6ShapeRep)]
impl SimpleEntityHandler for AdvancedBrepShapeRepresentationHandler {
    type WriteInput = AdvancedBrepShapeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "ADVANCED_BREP_SHAPE_REPRESENTATION")?;
        let name = read_string(attrs, 0, entity_id, "name")?.to_owned();
        let items = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        let context = ctx.resolve_repr_context(ctx_ref);
        if let Some(crate::ir::shape_rep::RepresentationContextRef::Unitful(ctx_id)) = context {
            ctx.repr_context_map.insert(entity_id, ctx_id);
        }

        let solid_ids: Vec<_> = items
            .iter()
            .filter_map(|r| ctx.solid_map.get(r).copied())
            .collect();
        // Pick the first AXIS2_PLACEMENT_3D in the items list as the coordinate
        // reference frame. In practice commercial CAD output places it first.
        let ref_frame = items.iter().find_map(|r| ctx.placement_map.get(r).copied());
        if let Some(placement_id) = ref_frame {
            ctx.absr_ref_frame_map.insert(entity_id, placement_id);
        }
        if solid_ids.is_empty() {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: String::from(
                    "ADVANCED_BREP_SHAPE_REPRESENTATION without a MANIFOLD_SOLID_BREP item",
                ),
            });
        } else {
            ctx.absr_solid_map.insert(entity_id, solid_ids.clone());
        }

        // representation-refactor A-1: dual-write into the unified arena.
        let repr_id = ctx
            .representations
            .push(crate::ir::shape_rep::Representation::AdvancedBrep(
                crate::ir::shape_rep::AdvancedBrepRepr {
                    name,
                    context,
                    ref_frame,
                    solids: solid_ids,
                },
            ));
        ctx.repr_id_map.insert(entity_id, repr_id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        AdvancedBrepShapeRepresentationWriteInput {
            product,
            solid_refs,
            unit_ctx,
        }: AdvancedBrepShapeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        let axis_ref = buf.emit_axis2_placement_3d(product.shape_ref_frame)?;
        let mut items = Vec::with_capacity(1 + solid_refs.len());
        items.push(Attribute::EntityRef(axis_ref));
        items.extend(solid_refs.into_iter().map(Attribute::EntityRef));
        Ok(buf.push_simple(
            "ADVANCED_BREP_SHAPE_REPRESENTATION",
            vec![
                Attribute::String(String::new()),
                Attribute::List(items),
                Attribute::EntityRef(unit_ctx),
            ],
        ))
    }
}
