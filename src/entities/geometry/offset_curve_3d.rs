//! `OFFSET_CURVE_3D` handler.
//!
//! `OFFSET_CURVE_3D(name, basis_curve, distance, self_intersect,
//! ref_direction)` — wraps another 3D curve as its basis offset by
//! `distance` along `ref_direction`. Topological dispatch processes the
//! basis curve first, so the ref always resolves (same as `OFFSET_SURFACE`).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::OffsetCurve3d;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct OffsetCurve3dHandler;

#[step_entity(name = "OFFSET_CURVE_3D")]
impl SimpleEntityHandler for OffsetCurve3dHandler {
    type WriteInput = OffsetCurve3d;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_offset_curve_3d(entity_id, attrs)?;
        lower::lower_offset_curve_3d(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, oc: OffsetCurve3d) -> Result<u64, WriteError> {
        let basis = buf.emit_curve(oc.basis)?;
        let dir = buf.emit_direction(oc.ref_direction)?;
        let early = lift::lift_offset_curve_3d(basis, oc.distance, oc.self_intersect, dir);
        Ok(serialize::serialize_offset_curve_3d(buf, &early))
    }
}
