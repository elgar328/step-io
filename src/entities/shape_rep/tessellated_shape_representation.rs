//! `TESSELLATED_SHAPE_REPRESENTATION` handler — phase tsr.
//!
//! `shape_representation` SUBTYPE whose `items` SET is narrowed to
//! `tessellated_item`. EXPRESS WHERE rule requires `context_of_items`
//! to be a `GLOBAL_UNIT_ASSIGNED_CONTEXT`; non-unitful contexts drop
//! the carrier (symmetric on re-read). Items unresolved by
//! `resolve_tessellated_item_ref` are skipped from the set; an empty
//! resolved set drops the carrier.
//!
//! Emit is delayed (`Mdgpr` / `DraughtingModel` pattern) —
//! `emit_representations_pre_pass` skips this variant, and
//! `emit_tessellated_shape_representations` runs after
//! `emit_tessellation` so the `tessellated_*_step_ids` caches are
//! fully populated before `items` refs resolve.

use crate::entities::SimpleEntityHandler;
use crate::entities::tessellation::resolve_tessellated_item_ref;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{Representation, TessellatedShapeRepresentation};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct TessellatedShapeRepresentationHandler;

#[step_entity(name = "TESSELLATED_SHAPE_REPRESENTATION")]
impl SimpleEntityHandler for TessellatedShapeRepresentationHandler {
    type WriteInput = TessellatedShapeRepresentation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "TESSELLATED_SHAPE_REPRESENTATION")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        // EXPRESS WHERE wr1: must be GLOBAL_UNIT_ASSIGNED_CONTEXT. Only
        // accept the Unitful variant; other contexts drop the carrier.
        let context = ctx.resolve_repr_context(ctx_ref);
        let Some(crate::ir::shape_rep::RepresentationContextRef::Unitful(ctx_id)) = context else {
            return Ok(());
        };
        ctx.repr_context_map.insert(entity_id, ctx_id);

        let items: Vec<_> = item_refs
            .iter()
            .filter_map(|&r| resolve_tessellated_item_ref(ctx, r))
            .collect();
        if items.is_empty() {
            return Ok(());
        }

        let repr_id = ctx
            .representations
            .push(Representation::TessellatedShapeRepresentation(
                TessellatedShapeRepresentation {
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
        tsr: TessellatedShapeRepresentation,
    ) -> Result<u64, WriteError> {
        let items: Vec<Attribute> = tsr
            .items
            .iter()
            .map(|&r| Attribute::EntityRef(buf.emit_tessellated_item_ref(r)))
            .collect();
        let ctx_attr = buf.repr_context_attr(tsr.context);
        Ok(buf.push_simple(
            "TESSELLATED_SHAPE_REPRESENTATION",
            vec![
                Attribute::String(tsr.name),
                Attribute::List(items),
                ctx_attr,
            ],
        ))
    }
}
