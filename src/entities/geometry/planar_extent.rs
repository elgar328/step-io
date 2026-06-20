//! `PLANAR_EXTENT` / `PLANAR_BOX` handlers.
//!
//! A `concrete_supertype` pair: both push into the single `planar_extents`
//! arena as variants of [`PlanarExtent`]. `PLANAR_EXTENT(name, size_in_x,
//! size_in_y)` is the base; `PLANAR_BOX` adds a trailing `placement`
//! (`axis2_placement` SELECT — 2D or 3D).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::{PlanarBox, PlanarBoxPlacement, PlanarExtentData};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PlanarExtentHandler;

#[step_entity(name = "PLANAR_EXTENT")]
impl SimpleEntityHandler for PlanarExtentHandler {
    type WriteInput = PlanarExtentData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_planar_extent(entity_id, attrs)?;
        lower::lower_planar_extent(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: PlanarExtentData) -> Result<u64, WriteError> {
        let early = lift::lift_planar_extent(data.name, data.size_in_x, data.size_in_y);
        Ok(serialize::serialize_planar_extent(buf, &early))
    }
}

pub(crate) struct PlanarBoxHandler;

#[step_entity(name = "PLANAR_BOX")]
impl SimpleEntityHandler for PlanarBoxHandler {
    type WriteInput = PlanarBox;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_planar_box(entity_id, attrs)?;
        lower::lower_planar_box(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, pb: PlanarBox) -> Result<u64, WriteError> {
        let placement_step = match pb.placement {
            PlanarBoxPlacement::Placement3d(id) => buf.emit_axis2_placement_3d(id)?,
            PlanarBoxPlacement::Placement2d(id) => buf.emit_axis2_placement_2d(id)?,
        };
        let early = lift::lift_planar_box(pb.name, pb.size_in_x, pb.size_in_y, placement_step);
        Ok(serialize::serialize_planar_box(buf, &early))
    }
}
