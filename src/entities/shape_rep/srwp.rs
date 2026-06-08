//! `SHAPE_REPRESENTATION_WITH_PARAMETERS` handler — phase srwp.
//!
//! `shape_representation` SUBTYPE that narrows `items` to a SELECT of
//! `direction` / `placement` / `descriptive_representation_item` /
//! `measure_representation_item` (the last resolved through the
//! `representation_item` arena via `repr_item_id_map`). Emit delayed
//! (Mdgpr / DM / TSR / CGR pattern) — the pre-pass skips this variant
//! and `emit_shape_representation_with_parameters` writes into the
//! `representation_step_ids` slot by `RepresentationId`.

use crate::entities::SimpleEntityHandler;
use crate::entities::shape_rep::descriptive_representation_item::DescriptiveRepresentationItemHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{Representation, ShapeRepresentationWithParameters, SrwpItem};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ShapeRepresentationWithParametersHandler;

#[step_entity(name = "SHAPE_REPRESENTATION_WITH_PARAMETERS")]
impl SimpleEntityHandler for ShapeRepresentationWithParametersHandler {
    type WriteInput = ShapeRepresentationWithParameters;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SHAPE_REPRESENTATION_WITH_PARAMETERS")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        let context = ctx.resolve_repr_context(ctx_ref);
        if let Some(crate::ir::shape_rep::RepresentationContextRef::Unitful(ctx_id)) = context {
            ctx.repr_context_map.insert(entity_id, ctx_id);
        }
        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(&did) = ctx.direction_map.get(&r) {
                items.push(SrwpItem::Direction(did));
            } else if let Some(&pid) = ctx.placement_map.get(&r) {
                items.push(SrwpItem::Placement(pid));
            } else if let Some(d) = ctx.descriptive_item_map.get(&r).cloned() {
                items.push(SrwpItem::Descriptive(d));
            } else if let Some(&id) = ctx.repr_item_id_map.get(&r) {
                // MEASURE_REPRESENTATION_ITEM lives in the representation_item
                // arena. Guard on the variant: repr_item_id_map also holds
                // QRI / VRI.
                if matches!(
                    ctx.representation_items[id],
                    crate::ir::representation_item::RepresentationItem::MeasureRepresentationItem(
                        _
                    )
                ) {
                    items.push(SrwpItem::MeasureItem(id));
                }
            }
        }
        if items.is_empty() {
            return Ok(());
        }
        let repr_id = ctx
            .representations
            .push(Representation::ShapeRepresentationWithParameters(
                ShapeRepresentationWithParameters {
                    name,
                    items,
                    context,
                },
            ));
        ctx.repr_id_map.insert(entity_id, repr_id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        srwp: ShapeRepresentationWithParameters,
    ) -> Result<u64, WriteError> {
        let mut item_refs = Vec::with_capacity(srwp.items.len());
        for item in srwp.items {
            let step = match item {
                SrwpItem::Direction(id) => buf.emit_direction(id)?,
                SrwpItem::Placement(id) => buf.emit_axis2_placement_3d(id)?,
                SrwpItem::Descriptive(d) => DescriptiveRepresentationItemHandler::write(buf, d)?,
                SrwpItem::MeasureItem(id) => buf.representation_item_step_ids[id.0 as usize],
            };
            item_refs.push(Attribute::EntityRef(step));
        }
        let ctx_attr = buf.repr_context_attr(srwp.context);
        Ok(buf.push_simple(
            "SHAPE_REPRESENTATION_WITH_PARAMETERS",
            vec![
                Attribute::String(srwp.name),
                Attribute::List(item_refs),
                ctx_attr,
            ],
        ))
    }
}
