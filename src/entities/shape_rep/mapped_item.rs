//! `REPRESENTATION_MAP` + `MAPPED_ITEM` handlers (phase mapped-item).
//!
//! `REPRESENTATION_MAP(mapping_origin, mapped_representation)` is a reusable
//! mapping; `MAPPED_ITEM(name, mapping_source, mapping_target)` instantiates
//! one through a `representation_item` transform. Both are read into their
//! own arenas and emitted standalone — their containers (`DRAUGHTING_MODEL`
//! / plain `REPRESENTATION`) are not modelled yet, so a `MAPPED_ITEM` that
//! is otherwise unreferenced still round-trips as an orphan.
//!
//! A `CAMERA_MODEL_D3` mapping origin is schema-valid (it is a
//! `representation_item` subtype) and is preserved as an `Itself` rmap; the
//! writer emits it in a delayed pass once the camera step ids are populated.
//! Refs that step-io genuinely cannot model leave the entity unresolved; it
//! is silently dropped, symmetric on re-read so round-trip equality holds.

use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{
    MappedItem, MappedItemData, MappedRepresentationRef, RepresentationMap, RepresentationMapData,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct RepresentationMapHandler;

#[step_entity(name = "REPRESENTATION_MAP")]
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
            return Ok(()); // unmodelled origin — drop
        };
        // A CAMERA_MODEL mapping_origin is schema-valid for a plain
        // REPRESENTATION_MAP (CAMERA_USAGE narrows it but is a separate entity).
        // Keep it as `Itself`; the writer emits it in a delayed pass once the
        // camera step ids are populated (`emit_camera_origin_mapped_items`).
        // `mapped_representation` is the `representation` supertype: a plain
        // representation, or a PRESENTATION_VIEW/AREA (separate arena). Probe
        // both; presentation-mapped rmaps emit in a delayed pass too.
        let mapped_representation = if let Some(rid) = ctx
            .id_cache
            .get::<crate::ir::id::RepresentationId>(mapped_ref)
        {
            MappedRepresentationRef::Representation(rid)
        } else if let Some(pid) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationRepresentationId>(mapped_ref)
        {
            MappedRepresentationRef::Presentation(pid)
        } else {
            return Ok(());
        };

        let id = ctx
            .representation_maps
            .push(RepresentationMap::Itself(RepresentationMapData {
                mapping_origin,
                mapped_representation,
            }));
        ctx.id_cache.insert(entity_id, id);
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
        let mapped = match d.mapped_representation {
            MappedRepresentationRef::Representation(id) => buf.step_id(id),
            MappedRepresentationRef::Presentation(id) => buf.step_id(id),
        };
        Ok(buf.push_simple(
            "REPRESENTATION_MAP",
            vec![Attribute::EntityRef(origin), Attribute::EntityRef(mapped)],
        ))
    }
}

pub(crate) struct MappedItemHandler;

#[step_entity(name = "MAPPED_ITEM")]
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

        let Some(mapping_source) = ctx
            .id_cache
            .get::<crate::ir::id::RepresentationMapId>(source_ref)
        else {
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
        ctx.id_cache.insert(entity_id, mi_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, mi: MappedItem) -> Result<u64, WriteError> {
        // Only `Itself` emits as a plain MAPPED_ITEM. CameraImage variants
        // emit as their own STEP types via dedicated handlers invoked from
        // `emit_mapped_items`.
        let MappedItem::Itself(d) = mi else {
            return Ok(0);
        };
        let source = buf.step_id(d.mapping_source);
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
