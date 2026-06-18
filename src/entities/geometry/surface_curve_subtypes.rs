//! `BOUNDED_SURFACE_CURVE` + `INTERSECTION_CURVE` handlers — 2-layer path
//! (generated `bind`/`serialize`, hand `lower`/`lift`).
//!
//! Separate from the existing `surface_curve.rs` alias path (base
//! `SURFACE_CURVE` unwraps to `curve_3d`). Both subtypes are corpus 0
//! per `ir.toml`; the alias and this arena never overlap.
//!
//! `associated_geometry` SELECT is partial — only the `surface` branch is
//! modelled. L1 keeps the full `Vec<u64>` (schema-strict); the surface-only
//! filter + `pcurve`/unresolved drop lives in [`lower`](crate::early::lower).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::SurfaceCurveData;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct BoundedSurfaceCurveHandler;

#[step_entity(name = "BOUNDED_SURFACE_CURVE")]
impl SimpleEntityHandler for BoundedSurfaceCurveHandler {
    type WriteInput = SurfaceCurveData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_bounded_surface_curve(entity_id, attrs)?;
        lower::lower_bounded_surface_curve(ctx, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, body: SurfaceCurveData) -> Result<u64, WriteError> {
        let early = lift::lift_bounded_surface_curve(buf, body)?;
        Ok(serialize::serialize_bounded_surface_curve(buf, &early))
    }
}

pub(crate) struct IntersectionCurveHandler;

#[step_entity(name = "INTERSECTION_CURVE")]
impl SimpleEntityHandler for IntersectionCurveHandler {
    type WriteInput = SurfaceCurveData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_intersection_curve(entity_id, attrs)?;
        lower::lower_intersection_curve(ctx, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, body: SurfaceCurveData) -> Result<u64, WriteError> {
        let early = lift::lift_intersection_curve(buf, body)?;
        Ok(serialize::serialize_intersection_curve(buf, &early))
    }
}
