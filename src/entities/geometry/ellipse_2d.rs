//! `ELLIPSE` handler — 2D sister (pcurve subtree) → `Curve2d::Ellipse` (2-layer
//! path; reuses the 3D `ELLIPSE` generated bind/serialize).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_2d::Axis2Placement2dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::Ellipse2;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct Ellipse2dHandler;

#[step_entity(name = "ELLIPSE", is_2d)]
impl SimpleEntityHandler for Ellipse2dHandler {
    type WriteInput = Ellipse2;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_ellipse(entity_id, attrs)?;
        lower::lower_ellipse_2d(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, e: Ellipse2) -> Result<u64, WriteError> {
        let pos = Axis2Placement2dHandler::write(buf, e.position)?;
        let early = lift::lift_ellipse(pos, e.semi_axis_1, e.semi_axis_2);
        Ok(serialize::serialize_ellipse(buf, &early))
    }
}
