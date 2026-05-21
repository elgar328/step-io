//! `VIEW_VOLUME` handler — Pass 8-pre-d.
//!
//! A `founded_item` subtype describing the projection volume a
//! `CAMERA_MODEL_D3` views through. Pushed into the shared `founded_item`
//! arena as the [`FoundedItem::ViewVolume`] variant. A `VIEW_VOLUME` whose
//! `projection_point` / `view_window` ref does not resolve is silently
//! dropped, symmetric on re-read.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_enum, read_real};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{FoundedItem, Projection, ViewVolume, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// Map a STEP `central_or_parallel` enum value to [`Projection`].
fn read_projection(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Projection, ConvertError> {
    match read_enum(attrs, index, entity_id, field_name)? {
        "CENTRAL" => Ok(Projection::Central),
        "PARALLEL" => Ok(Projection::Parallel),
        other => Err(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!("{field_name}: unknown central_or_parallel '.{other}.'"),
        }),
    }
}

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

#[step_entity(name = "VIEW_VOLUME", pass = Pass8ViewVolume)]
impl SimpleEntityHandler for ViewVolumeHandler {
    type WriteInput = ViewVolume;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 9, entity_id, "VIEW_VOLUME")?;
        let projection_type = read_projection(attrs, 0, entity_id, "projection_type")?;
        let projection_point_ref = read_entity_ref(attrs, 1, entity_id, "projection_point")?;
        let view_plane_distance = read_real(attrs, 2, entity_id, "view_plane_distance")?;
        let front_plane_distance = read_real(attrs, 3, entity_id, "front_plane_distance")?;
        let front_plane_clipping = read_bool(attrs, 4, entity_id, "front_plane_clipping")?;
        let back_plane_distance = read_real(attrs, 5, entity_id, "back_plane_distance")?;
        let back_plane_clipping = read_bool(attrs, 6, entity_id, "back_plane_clipping")?;
        let view_volume_sides_clipping =
            read_bool(attrs, 7, entity_id, "view_volume_sides_clipping")?;
        let view_window_ref = read_entity_ref(attrs, 8, entity_id, "view_window")?;

        let Some(&projection_point) = ctx.point_map.get(&projection_point_ref) else {
            return Ok(()); // projection_point unresolved — drop the view volume
        };
        let Some(&view_window) = ctx.planar_extent_id_map.get(&view_window_ref) else {
            return Ok(());
        };

        ctx.visualization
            .get_or_insert_with(VisualizationPool::default)
            .founded_items
            .push(FoundedItem::ViewVolume(ViewVolume {
                projection_type,
                projection_point,
                view_plane_distance,
                front_plane_distance,
                front_plane_clipping,
                back_plane_distance,
                back_plane_clipping,
                view_volume_sides_clipping,
                view_window,
            }));
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
