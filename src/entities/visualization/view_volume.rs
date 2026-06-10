//! `VIEW_VOLUME` handler.
//!
//! A `founded_item` subtype describing the projection volume a
//! `CAMERA_MODEL_D3` views through. Pushed into the shared `founded_item`
//! arena as the [`FoundedItem::ViewVolume`] variant. A `VIEW_VOLUME` whose
//! `projection_point` / `view_window` ref does not resolve is silently
//! dropped, symmetric on re-read.

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{Projection, ViewVolume};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// [`Projection`] → a STEP enum `Attribute`.
fn projection_attr(p: Projection) -> Attribute {
    Attribute::Enum(
        match p {
            Projection::Central => "CENTRAL",
            Projection::Parallel => "PARALLEL",
        }
        .into(),
    )
}

/// `bool` → a STEP boolean enum `Attribute` (`.T.` / `.F.`).
fn bool_attr(b: bool) -> Attribute {
    Attribute::Enum(if b { "T" } else { "F" }.into())
}

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
        let projection_point = buf.emit_point(vv.projection_point)?;
        let view_window = buf.emit_planar_extent(vv.view_window)?;
        Ok(buf.push_simple(
            "VIEW_VOLUME",
            vec![
                projection_attr(vv.projection_type),
                Attribute::EntityRef(projection_point),
                Attribute::Real(vv.view_plane_distance),
                Attribute::Real(vv.front_plane_distance),
                bool_attr(vv.front_plane_clipping),
                Attribute::Real(vv.back_plane_distance),
                bool_attr(vv.back_plane_clipping),
                bool_attr(vv.view_volume_sides_clipping),
                Attribute::EntityRef(view_window),
            ],
        ))
    }
}
