//! `CONSTRUCTIVE_GEOMETRY_REPRESENTATION` handler — phase cgr.
//!
//! `representation` SUBTYPE carrying a SET of geometry items narrowed by
//! EXPRESS WHERE wr2 to `placement` / `curve` / `edge` / `face` /
//! `point` / `surface` / `face_surface` / `vertex_point`. step-io's
//! `resolve_representation_item_ref` covers all eight types via the
//! per-arena id maps. Items unresolved by the resolver are skipped
//! (symmetric on re-read).
//!
//! Emit is delayed (`Mdgpr` / `DraughtingModel` / `TSR` pattern) —
//! `emit_representations_pre_pass` skips this variant, and
//! `emit_constructive_geometry_representations` writes into the
//! `representation_step_ids` slot by `RepresentationId`.

use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{ConstructiveGeometryRepr, Representation};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ConstructiveGeometryRepresentationHandler;

#[step_entity(name = "CONSTRUCTIVE_GEOMETRY_REPRESENTATION")]
impl SimpleEntityHandler for ConstructiveGeometryRepresentationHandler {
    type WriteInput = ConstructiveGeometryRepr;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "CONSTRUCTIVE_GEOMETRY_REPRESENTATION")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        let context = ctx.resolve_repr_context(ctx_ref);
        if let Some(crate::ir::shape_rep::RepresentationContextRef::Unitful(ctx_id)) = context {
            ctx.repr_context_map.insert(entity_id, ctx_id);
        }
        let items: Vec<_> = item_refs
            .iter()
            .filter_map(|&r| resolve_representation_item_ref(ctx, r))
            .collect();
        if items.is_empty() {
            return Ok(());
        }
        let repr_id = ctx
            .representations
            .push(Representation::ConstructiveGeometry(
                ConstructiveGeometryRepr {
                    name,
                    items,
                    context,
                },
            ));
        ctx.id_cache.insert(entity_id, repr_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, cgr: ConstructiveGeometryRepr) -> Result<u64, WriteError> {
        let mut item_refs = Vec::with_capacity(cgr.items.len());
        for item in cgr.items {
            let step = buf.emit_representation_item_ref(item)?;
            item_refs.push(Attribute::EntityRef(step));
        }
        let ctx_attr = buf.repr_context_attr(cgr.context);
        Ok(buf.push_simple(
            "CONSTRUCTIVE_GEOMETRY_REPRESENTATION",
            vec![
                Attribute::String(cgr.name),
                Attribute::List(item_refs),
                ctx_attr,
            ],
        ))
    }
}
