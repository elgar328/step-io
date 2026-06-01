//! `SEAM_CURVE` handler — Pass 4-3.
//!
//! Sister handler of `SURFACE_CURVE`. Both share their read/write body
//! in `surface_curve.rs`; only the entity name on emission differs.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::surface_curve::{
    read_surface_or_seam_curve_body, write_surface_or_seam_curve_body,
};
use crate::ir::PCurveOrSurface;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SeamCurveHandler;

#[step_entity(name = "SEAM_CURVE", pass = Pass4_3SurfaceCurve)]
impl SimpleEntityHandler for SeamCurveHandler {
    type WriteInput = (u64, Vec<PCurveOrSurface>);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_surface_or_seam_curve_body(ctx, entity_id, attrs, "SEAM_CURVE")
    }

    fn write(
        buf: &mut WriteBuffer,
        (curve_3d_ref, members): (u64, Vec<PCurveOrSurface>),
    ) -> Result<u64, WriteError> {
        write_surface_or_seam_curve_body(buf, curve_3d_ref, &members, true)
    }
}
