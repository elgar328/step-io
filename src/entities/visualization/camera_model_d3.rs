//! `CAMERA_MODEL_D3` handler.
//!
//! A `camera_model` subtype pairing a view reference frame with a
//! `VIEW_VOLUME`. Pushed into `VisualizationPool::camera_models` as the
//! [`CameraModel::CameraModelD3`] variant. A `CAMERA_MODEL_D3` whose
//! `view_reference_system` / `perspective_of_volume` ref does not resolve
//! is silently dropped, symmetric on re-read.

use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::CameraModelD3;
use crate::parser::entity::Attribute;
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
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_camera_model_d3(entity_id, attrs)?;
        crate::early::lower::lower_camera_model_d3(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, cm: CameraModelD3) -> Result<u64, WriteError> {
        let vrs = buf.emit_axis2_placement_3d(cm.view_reference_system)?;
        let pov = buf.step_id(cm.perspective_of_volume);
        let early = crate::early::lift::lift_camera_model_d3(cm.name, vrs, pov);
        Ok(crate::early::serialize::serialize_camera_model_d3(
            buf, &early,
        ))
    }
}
