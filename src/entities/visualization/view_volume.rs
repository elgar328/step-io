//! `VIEW_VOLUME` handler.
//!
//! A `founded_item` subtype describing the projection volume a
//! `CAMERA_MODEL_D3` views through. Pushed into the shared `founded_item`
//! arena as the [`FoundedItem::ViewVolume`] variant. A `VIEW_VOLUME` whose
//! `projection_point` / `view_window` ref does not resolve is silently
//! dropped, symmetric on re-read.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::ViewVolume;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ViewVolumeHandler;

#[step_entity(name = "VIEW_VOLUME")]
impl SimpleEntityHandler for ViewVolumeHandler {
    type WriteInput = ViewVolume;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // 2-layer path: bind → L1, then lower → L2. `lower` registers the typed
        // `EarlyViewVolumeId` cache key so the camera-model handlers resolve
        // `perspective_of_volume` by L1 type (replaces `viz_view_volume_id_map`).
        let early = bind::bind_view_volume(entity_id, attrs)?;
        lower::lower_view_volume(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, vv: ViewVolume) -> Result<u64, WriteError> {
        // 2-layer write path: lift L2 → L1 (emitting the geometry sub-refs),
        // then serialize L1 → Part21 text.
        let early = lift::lift_view_volume(buf, &vv)?;
        Ok(serialize::serialize_view_volume(buf, &early))
    }
}
