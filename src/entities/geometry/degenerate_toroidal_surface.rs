//! `DEGENERATE_TOROIDAL_SURFACE` handler leaf surface.
//!
//! Subtype of `TOROIDAL_SURFACE` adding the `select_outer` boolean which
//! chooses the outer or inner sheet when the minor radius produces a
//! degenerate (self-intersecting) torus.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::DegenerateToroidalSurface;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DegenerateToroidalSurfaceHandler;

#[step_entity(name = "DEGENERATE_TOROIDAL_SURFACE")]
impl SimpleEntityHandler for DegenerateToroidalSurfaceHandler {
    type WriteInput = DegenerateToroidalSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_degenerate_toroidal_surface(entity_id, attrs)?;
        lower::lower_degenerate_toroidal_surface(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, dts: DegenerateToroidalSurface) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, dts.position)?;
        let early = lift::lift_degenerate_toroidal_surface(
            pos,
            dts.major_radius,
            dts.minor_radius,
            dts.select_outer,
        );
        Ok(serialize::serialize_degenerate_toroidal_surface(
            buf, &early,
        ))
    }
}
