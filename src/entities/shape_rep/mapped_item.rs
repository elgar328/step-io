//! `REPRESENTATION_MAP` + `MAPPED_ITEM` handlers (phase mapped-item).
//!
//! `REPRESENTATION_MAP(mapping_origin, mapped_representation)` is a reusable
//! mapping; `MAPPED_ITEM(name, mapping_source, mapping_target)` instantiates
//! one through a `representation_item` transform. Both are read into their
//! own arenas and emitted standalone — their containers (`DRAUGHTING_MODEL`
//! / plain `REPRESENTATION`) are not modelled yet, so a `MAPPED_ITEM` that
//! is otherwise unreferenced still round-trips as an orphan.
//!
//! Refs that step-io cannot model (e.g. a `CAMERA_MODEL_D3` mapping origin)
//! leave the entity unresolved; it is silently dropped, symmetric on
//! re-read so round-trip equality holds for the supported subset.

use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{MappedItem, MappedItemData, RepresentationMap, RepresentationMapData};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct RepresentationMapHandler;

#[step_entity(name = "REPRESENTATION_MAP", pass = Pass6RepresentationMap)]
impl SimpleEntityHandler for RepresentationMapHandler {
    type WriteInput = RepresentationMap;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "REPRESENTATION_MAP")?;
        let origin_ref = read_entity_ref(attrs, 0, entity_id, "mapping_origin")?;
        let mapped_ref = read_entity_ref(attrs, 1, entity_id, "mapped_representation")?;

        let Some(mapping_origin) = resolve_representation_item_ref(ctx, origin_ref) else {
            return Ok(()); // unmodelled origin (e.g. CAMERA_MODEL_D3) — drop
        };
        let Some(&mapped_representation) = ctx.repr_id_map.get(&mapped_ref) else {
            return Ok(());
        };

        let id = ctx
            .representation_maps
            .push(RepresentationMap::Itself(RepresentationMapData {
                mapping_origin,
                mapped_representation,
            }));
        ctx.representation_map_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, rmap: RepresentationMap) -> Result<u64, WriteError> {
        // Only `Itself` is emitted here; `CameraUsage` is handled by the
        // delayed `emit_camera_usage_arena` so its `mapped_representation`
        // (which may point at a DRAUGHTING_MODEL) resolves through a fully
        // populated `representation_step_ids` cache.
        let RepresentationMap::Itself(d) = rmap else {
            return Ok(0);
        };
        let origin = buf.emit_representation_item_ref(d.mapping_origin)?;
        let mapped = buf.representation_step_ids[d.mapped_representation.0 as usize];
        Ok(buf.push_simple(
            "REPRESENTATION_MAP",
            vec![Attribute::EntityRef(origin), Attribute::EntityRef(mapped)],
        ))
    }
}

pub(crate) struct MappedItemHandler;

#[step_entity(name = "MAPPED_ITEM", pass = Pass6MappedItem)]
impl SimpleEntityHandler for MappedItemHandler {
    type WriteInput = MappedItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "MAPPED_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let source_ref = read_entity_ref(attrs, 1, entity_id, "mapping_source")?;
        let target_ref = read_entity_ref(attrs, 2, entity_id, "mapping_target")?;

        let Some(&mapping_source) = ctx.representation_map_id_map.get(&source_ref) else {
            return Ok(()); // mapping_source's REPRESENTATION_MAP was dropped
        };
        let Some(mapping_target) = resolve_representation_item_ref(ctx, target_ref) else {
            return Ok(());
        };

        let mi_id = ctx.mapped_items.push(MappedItem::Itself(MappedItemData {
            name,
            mapping_source,
            mapping_target,
        }));
        ctx.mapped_item_id_map.insert(entity_id, mi_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, mi: MappedItem) -> Result<u64, WriteError> {
        // Only `Itself` emits as a plain MAPPED_ITEM. CameraImage variants
        // emit as their own STEP types via dedicated handlers invoked from
        // `emit_mapped_items`.
        let MappedItem::Itself(d) = mi else {
            return Ok(0);
        };
        let source = buf.representation_map_step_ids[d.mapping_source.0 as usize];
        let target = buf.emit_representation_item_ref(d.mapping_target)?;
        Ok(buf.push_simple(
            "MAPPED_ITEM",
            vec![
                Attribute::String(d.name),
                Attribute::EntityRef(source),
                Attribute::EntityRef(target),
            ],
        ))
    }
}
