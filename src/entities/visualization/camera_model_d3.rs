//! `CAMERA_MODEL_D3` handler.
//!
//! A `camera_model` subtype pairing a view reference frame with a
//! `VIEW_VOLUME`. Pushed into `VisualizationPool::camera_models` as the
//! [`CameraModel::CameraModelD3`] variant. A `CAMERA_MODEL_D3` whose
//! `view_reference_system` / `perspective_of_volume` ref does not resolve
//! is silently dropped, symmetric on re-read.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{CameraModel, CameraModelD3, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CameraModelD3Handler;

#[step_entity(name = "CAMERA_MODEL_D3")]
impl SimpleEntityHandler for CameraModelD3Handler {
    type WriteInput = CameraModelD3;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "CAMERA_MODEL_D3")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let vrs_ref = read_entity_ref(attrs, 1, entity_id, "view_reference_system")?;
        let pov_ref = read_entity_ref(attrs, 2, entity_id, "perspective_of_volume")?;

        let Some(&view_reference_system) = ctx.placement_map.get(&vrs_ref) else {
            return Ok(()); // view_reference_system unresolved — drop the camera
        };
        let Some(&perspective_of_volume) = ctx.viz_view_volume_id_map.get(&pov_ref) else {
            return Ok(());
        };

        let id = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default)
            .camera_models
            .push(CameraModel::CameraModelD3(CameraModelD3 {
                name,
                view_reference_system,
                perspective_of_volume,
            }));
        ctx.viz_camera_model_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, cm: CameraModelD3) -> Result<u64, WriteError> {
        let vrs = buf.emit_axis2_placement_3d(cm.view_reference_system)?;
        let pov = buf.founded_item_step_ids[cm.perspective_of_volume.0 as usize];
        Ok(buf.push_simple(
            "CAMERA_MODEL_D3",
            vec![
                Attribute::String(cm.name),
                Attribute::EntityRef(vrs),
                Attribute::EntityRef(pov),
            ],
        ))
    }
}
