//! `ELLIPSE` handler — leaf (placement + measures) (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::Ellipse3;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct EllipseHandler;

#[step_entity(name = "ELLIPSE")]
impl SimpleEntityHandler for EllipseHandler {
    type WriteInput = Ellipse3;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_ellipse(entity_id, attrs)?;
        lower::lower_ellipse(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, ellipse: Ellipse3) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, ellipse.position)?;
        let early = lift::lift_ellipse(pos, ellipse.semi_axis_1, ellipse.semi_axis_2);
        Ok(serialize::serialize_ellipse(buf, &early))
    }
}
