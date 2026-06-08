//! Plain `SHAPE_REPRESENTATION` handler.
//!
//! Catches the bare `SHAPE_REPRESENTATION` form used by Group products and
//! by the outer wrapper of Fusion 360 / CATIA indirect-SR chains. Reader
//! captures the first `AXIS2_PLACEMENT_3D` from `items` so the SDR pass can
//! re-emit it as `Product.outer_sr_frame` when the indirection chain is
//! taken. The dispatch registry exact-matches entity names, so ABSR / MSSR
//! never reach this handler.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::Product;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ShapeRepresentationWriteInput {
    pub(crate) product: Product,
    pub(crate) unit_ctx: u64,
}

pub(crate) struct ShapeRepresentationHandler;

#[step_entity(name = "SHAPE_REPRESENTATION")]
impl SimpleEntityHandler for ShapeRepresentationHandler {
    type WriteInput = ShapeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SHAPE_REPRESENTATION")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let items = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        let context = ctx.resolve_repr_context(ctx_ref);
        if let Some(crate::ir::shape_rep::RepresentationContextRef::Unitful(ctx_id)) = context {
            ctx.repr_context_map.insert(entity_id, ctx_id);
        }

        let frame = items.iter().find_map(|r| ctx.placement_map.get(r).copied());
        if let Some(placement_id) = frame {
            ctx.plain_sr_frame_map.insert(entity_id, placement_id);
        }

        // representation-refactor A-1: dual-write into the unified arena.
        let repr_id = ctx
            .representations
            .push(crate::ir::shape_rep::Representation::Plain(
                crate::ir::shape_rep::PlainRepr {
                    name,
                    context,
                    frame,
                },
            ));
        ctx.id_cache.insert(entity_id, repr_id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ShapeRepresentationWriteInput { product, unit_ctx }: ShapeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        let axis_ref = buf.emit_axis2_placement_3d(product.shape_ref_frame)?;
        Ok(buf.push_simple(
            "SHAPE_REPRESENTATION",
            vec![
                Attribute::String(String::new()),
                Attribute::List(vec![Attribute::EntityRef(axis_ref)]),
                Attribute::EntityRef(unit_ctx),
            ],
        ))
    }
}
