//! `PRESENTATION_VIEW` / `PRESENTATION_AREA` / `PRESENTATION_SET` —
//! phase pr-core.
//!
//! All three are `representation_item` subtype simple instances. View and
//! Area share the same arena (`PresentationRepresentation` enum) and
//! resolve `items` via the generic helper used by SDR / `DraughtingModel`;
//! Set is a minimal `name`-only carrier required by `AREA_IN_SET.in_set`.

use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    PresentationReprData, PresentationRepresentation, PresentationSet, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PresentationViewHandler;

#[step_entity(name = "PRESENTATION_VIEW")]
impl SimpleEntityHandler for PresentationViewHandler {
    type WriteInput = PresentationReprData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let data = read_presentation_repr(ctx, entity_id, attrs, "PRESENTATION_VIEW")?;
        if data.items.is_empty() {
            return Ok(());
        }
        let id = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default)
            .presentation_representations
            .push(PresentationRepresentation::View(data));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: PresentationReprData) -> Result<u64, WriteError> {
        emit_presentation_repr(buf, "PRESENTATION_VIEW", data)
    }
}

pub(crate) struct PresentationAreaHandler;

#[step_entity(name = "PRESENTATION_AREA")]
impl SimpleEntityHandler for PresentationAreaHandler {
    type WriteInput = PresentationReprData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let data = read_presentation_repr(ctx, entity_id, attrs, "PRESENTATION_AREA")?;
        if data.items.is_empty() {
            return Ok(());
        }
        let id = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default)
            .presentation_representations
            .push(PresentationRepresentation::Area(data));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: PresentationReprData) -> Result<u64, WriteError> {
        emit_presentation_repr(buf, "PRESENTATION_AREA", data)
    }
}

pub(crate) struct PresentationSetHandler;

#[step_entity(name = "PRESENTATION_SET")]
impl SimpleEntityHandler for PresentationSetHandler {
    type WriteInput = PresentationSet;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 0, entity_id, "PRESENTATION_SET")?;
        let id = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default)
            .presentation_sets
            .push(PresentationSet);
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, _set: PresentationSet) -> Result<u64, WriteError> {
        Ok(buf.push_simple("PRESENTATION_SET", vec![]))
    }
}

fn read_presentation_repr(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    name: &'static str,
) -> Result<PresentationReprData, ConvertError> {
    check_count(attrs, 3, entity_id, name)?;
    let item_name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
    let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
    let context = ctx.resolve_repr_context(ctx_ref);
    let mut items = Vec::with_capacity(item_refs.len());
    for r in item_refs {
        if let Some(item) = resolve_representation_item_ref(ctx, r) {
            items.push(item);
        }
    }
    Ok(PresentationReprData {
        name: item_name,
        items,
        context,
    })
}

fn emit_presentation_repr(
    buf: &mut WriteBuffer,
    name: &'static str,
    data: PresentationReprData,
) -> Result<u64, WriteError> {
    let mut item_refs = Vec::with_capacity(data.items.len());
    for item in data.items {
        let step = buf.emit_representation_item_ref(item)?;
        item_refs.push(Attribute::EntityRef(step));
    }
    let ctx_attr = buf.repr_context_attr(data.context);
    Ok(buf.push_simple(
        name,
        vec![
            Attribute::String(data.name),
            Attribute::List(item_refs),
            ctx_attr,
        ],
    ))
}
