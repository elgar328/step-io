//! `RATIONAL_B_SPLINE_SURFACE` handler (`ComplexEntityHandler`, 2-layer path).
//!
//! `emit_nurbs_surface` (`src/writer/buffer/geometry.rs`) routes every rational
//! `Surface::Nurbs` here. Shares its entity name with the curve handler — the two
//! complex entities key on different part-sets so dispatch never confuses them.
//! The generated complex `bind`/`serialize` (gen-early `[complex.rational_b_spline_surface]`)
//! handle the multi-part wire form, including the `weights_data` 2D grid.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::ComplexEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::{NurbsSurface, NurbsSurfaceKind};
use crate::parser::entity::RawEntityPart;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct RationalBsplineSurfaceHandler;

#[step_entity_complex(name = "RATIONAL_B_SPLINE_SURFACE", cases = [[
        "BOUNDED_SURFACE", "B_SPLINE_SURFACE", "B_SPLINE_SURFACE_WITH_KNOTS",
        "GEOMETRIC_REPRESENTATION_ITEM", "RATIONAL_B_SPLINE_SURFACE", "REPRESENTATION_ITEM", "SURFACE"
    ]])]
impl ComplexEntityHandler for RationalBsplineSurfaceHandler {
    type WriteInput = NurbsSurface;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_rational_b_spline_surface(entity_id, parts)?;
        lower::lower_rational_bspline_surface(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, nurbs: NurbsSurface) -> Result<u64, WriteError> {
        let NurbsSurfaceKind::Rational { weights } = &nurbs.kind else {
            return Err(WriteError::UnsupportedIrVariant {
                detail: "RationalBsplineSurfaceHandler::write requires weights".into(),
            });
        };
        let weights = weights.clone();
        let mut control_points = Vec::with_capacity(nurbs.control_points.len());
        for row in &nurbs.control_points {
            let mut pt_row = Vec::with_capacity(row.len());
            for &pid in row {
                pt_row.push(buf.emit_point(pid)?);
            }
            control_points.push(pt_row);
        }
        let early = lift::lift_rational_bspline_surface(&nurbs, control_points, weights);
        Ok(serialize::serialize_rational_b_spline_surface(buf, &early))
    }
}
