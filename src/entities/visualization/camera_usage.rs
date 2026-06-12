//! `CAMERA_USAGE` handler (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::CameraUsage;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CameraUsageHandler;

#[step_entity(name = "CAMERA_USAGE")]
impl SimpleEntityHandler for CameraUsageHandler {
    type WriteInput = CameraUsage;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_camera_usage(entity_id, attrs)?;
        lower::lower_camera_usage(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, cu: CameraUsage) -> Result<u64, WriteError> {
        let origin = buf.step_id(cu.mapping_origin);
        let mapped = buf.step_id(cu.mapped_representation);
        let early = lift::lift_camera_usage(origin, mapped);
        Ok(serialize::serialize_camera_usage(buf, &early))
    }
}
