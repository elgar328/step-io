//! `SHAPE_DIMENSION_REPRESENTATION` handler — phase sdr-dcr.
//!
//! SDR is a SUBTYPE OF `SHAPE_REPRESENTATION`; unlike the [`PlainRepr`]
//! narrowing (which extracts a single placement frame), SDR preserves
//! the full `items` SET because blueprint members are
//! `dimension_representation_item` SELECT entries — generic
//! [`RepresentationItemRef`] resolution covers the variants step-io
//! models. Unresolved items skip silently.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{Representation, ShapeDimensionRepresentation};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ShapeDimensionRepresentationHandler;

#[step_entity(name = "SHAPE_DIMENSION_REPRESENTATION")]
impl SimpleEntityHandler for ShapeDimensionRepresentationHandler {
    type WriteInput = ShapeDimensionRepresentation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SHAPE_DIMENSION_REPRESENTATION")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        let context = ctx.resolve_repr_context(ctx_ref);
        if let Some(crate::ir::shape_rep::RepresentationContextRef::Unitful(ctx_id)) = context {
            ctx.repr_context_map.insert(entity_id, ctx_id);
        }
        // SDR reads at Pass6ShapeRep, before the complex MEASURE_REPRESENTATION_ITEM
        // arena push at Pass8Measure. Defer item resolution: store the raw refs
        // and push an empty `items`; `resolve_deferred_sdr_items` (run after
        // Pass8Measure) rebuilds `items` once every referenced item is read.
        let repr_id = ctx
            .representations
            .push(Representation::ShapeDimensionRepresentation(
                ShapeDimensionRepresentation {
                    name,
                    context,
                    items: Vec::new(),
                },
            ));
        ctx.repr_id_map.insert(entity_id, repr_id);
        ctx.sdr_raw_items.insert(repr_id, item_refs);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, sdr: ShapeDimensionRepresentation) -> Result<u64, WriteError> {
        let mut item_refs = Vec::with_capacity(sdr.items.len());
        for item in sdr.items {
            let step = buf.emit_representation_item_ref(item)?;
            item_refs.push(Attribute::EntityRef(step));
        }
        let ctx_attr = buf.repr_context_attr(sdr.context);
        Ok(buf.push_simple(
            "SHAPE_DIMENSION_REPRESENTATION",
            vec![
                Attribute::String(sdr.name),
                Attribute::List(item_refs),
                ctx_attr,
            ],
        ))
    }
}
