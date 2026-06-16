//! `REPRESENTATION_MAP` + `MAPPED_ITEM` handlers (2-layer path).
//!
//! `REPRESENTATION_MAP(mapping_origin, mapped_representation)` is a reusable
//! mapping; `MAPPED_ITEM(name, mapping_source, mapping_target)` instantiates one
//! through a `representation_item` transform. Both read into their own arenas and
//! emit standalone (orphans still round-trip). Only the `Itself` variants emit
//! here; `CameraUsage` (`REPRESENTATION_MAP`) / `CameraImage*` (`MAPPED_ITEM`) are
//! handled by delayed passes / dedicated handlers.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{MappedItem, MappedRepresentationRef, RepresentationMap};
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
        let early = bind::bind_representation_map(entity_id, attrs)?;
        lower::lower_representation_map(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, rmap: RepresentationMap) -> Result<u64, WriteError> {
        // Only `Itself` emits here; `CameraUsage` is handled by the delayed
        // `emit_camera_usage_arena`.
        let RepresentationMap::Itself(d) = rmap else {
            return Ok(0);
        };
        let origin = buf.emit_representation_item_ref(d.mapping_origin)?;
        let mapped = match d.mapped_representation {
            MappedRepresentationRef::Representation(id) => buf.step_id(id),
            MappedRepresentationRef::Presentation(id) => buf.step_id(id),
        };
        let early = lift::lift_representation_map(origin, mapped);
        Ok(serialize::serialize_representation_map(buf, &early))
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
        let early = bind::bind_mapped_item(entity_id, attrs)?;
        lower::lower_mapped_item(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, mi: MappedItem) -> Result<u64, WriteError> {
        // Only `Itself` emits as a plain MAPPED_ITEM. CameraImage variants emit
        // as their own STEP types via dedicated handlers from `emit_mapped_items`.
        let MappedItem::Itself(d) = mi else {
            return Ok(0);
        };
        let source = buf.step_id(d.mapping_source);
        let target = buf.emit_representation_item_ref(d.mapping_target)?;
        let early = lift::lift_mapped_item(d.name, source, target);
        Ok(serialize::serialize_mapped_item(buf, &early))
    }
}
