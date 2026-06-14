//! `BOUNDED_PCURVE` handler — phase bpc.
//!
//! `parameter_space_curve` SUBTYPE — orphan in step-io (no inbound refs).
//! 3 attr inherited from `pcurve` (`name` + `basis_surface` + `reference_to_curve`).
//! `reference_to_curve` narrows to a `definitional_representation` — step-io
//! stores it as a plain `RepresentationId`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::BoundedPCurve;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct BoundedPCurveHandler;

#[step_entity(name = "BOUNDED_PCURVE")]
impl SimpleEntityHandler for BoundedPCurveHandler {
    type WriteInput = BoundedPCurve;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_bounded_pcurve(entity_id, attrs)?;
        lower::lower_bounded_pcurve(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, b: BoundedPCurve) -> Result<u64, WriteError> {
        let basis_step = buf.emit_surface(b.basis_surface)?;
        let ref_step = buf.step_id(b.reference_to_curve);
        let early = lift::lift_bounded_pcurve(b.name, basis_step, ref_step);
        Ok(serialize::serialize_bounded_pcurve(buf, &early))
    }
}
