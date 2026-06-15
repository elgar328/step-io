//! `CAMERA_MODEL_D3_WITH_HLHSR` + `CAMERA_MODEL_D3_MULTI_CLIPPING`
//! handlers — phase cm-variants.
//!
//! Both subtype `camera_model_d3` and reuse its first 3 attrs
//! (`name`, `view_reference_system`, `perspective_of_volume`); each adds
//! one extra attr (boolean `hidden_line_surface_removal` for hlhsr; a
//! SET of `shape_clipping` for `multi_clipping`).

use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{CameraModelD3MultiClipping, CameraModelD3WithHlhsr, ShapeClipping};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CameraModelD3WithHlhsrHandler;

#[step_entity(name = "CAMERA_MODEL_D3_WITH_HLHSR")]
impl SimpleEntityHandler for CameraModelD3WithHlhsrHandler {
    type WriteInput = CameraModelD3WithHlhsr;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_camera_model_d3_with_hlhsr(entity_id, attrs)?;
        crate::early::lower::lower_camera_model_d3_with_hlhsr(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: CameraModelD3WithHlhsr) -> Result<u64, WriteError> {
        let vrs = buf.emit_axis2_placement_3d(c.inherited.view_reference_system)?;
        let pov = buf.step_id(c.inherited.perspective_of_volume);
        let early = crate::early::lift::lift_camera_model_d3_with_hlhsr(
            c.inherited.name,
            vrs,
            pov,
            c.hidden_line_surface_removal,
        );
        Ok(crate::early::serialize::serialize_camera_model_d3_with_hlhsr(buf, &early))
    }
}

pub(crate) struct CameraModelD3MultiClippingHandler;

#[step_entity(name = "CAMERA_MODEL_D3_MULTI_CLIPPING")]
impl SimpleEntityHandler for CameraModelD3MultiClippingHandler {
    type WriteInput = CameraModelD3MultiClipping;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_camera_model_d3_multi_clipping(entity_id, attrs)?;
        crate::early::lower::lower_camera_model_d3_multi_clipping(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: CameraModelD3MultiClipping) -> Result<u64, WriteError> {
        let vrs = buf.emit_axis2_placement_3d(c.inherited.view_reference_system)?;
        let pov = buf.step_id(c.inherited.perspective_of_volume);
        let mut clipping = Vec::with_capacity(c.shape_clipping.len());
        for sc in c.shape_clipping {
            match sc {
                ShapeClipping::Plane(id) => clipping.push(buf.emit_surface(id)?),
            }
        }
        let early = crate::early::lift::lift_camera_model_d3_multi_clipping(
            c.inherited.name,
            vrs,
            pov,
            clipping,
        );
        Ok(crate::early::serialize::serialize_camera_model_d3_multi_clipping(buf, &early))
    }
}
