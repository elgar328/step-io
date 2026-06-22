//! `PLANE_ANGLE_UNIT` handler leaf for plane-angle flavour.
//!
//! Both cases (SI / `CONVERSION_BASED_UNIT`) are generated 2-layer
//! (units-CBU-②): `bind_plane_angle_unit` yields the `EarlyPlaneAngleUnit` enum
//! and `lower_plane_angle` dispatches to the SI / CBU lowering. CBU unit
//! identification is **by conversion factor** (fixed radian base) in
//! `lower_plane_angle_cbu`; the CBU outer is emitted via the generated
//! `serialize_plane_angle_unit_with_id` by the units pool emitter.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::ComplexEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::units::PlaneAngleFlavor;
use crate::parser::entity::RawEntityPart;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct PlaneAngleUnitHandler;

#[step_entity_complex(name = "PLANE_ANGLE_UNIT", cases = [
    ["CONVERSION_BASED_UNIT", "NAMED_UNIT", "PLANE_ANGLE_UNIT"],
    ["NAMED_UNIT", "PLANE_ANGLE_UNIT", "SI_UNIT"],
])]
impl ComplexEntityHandler for PlaneAngleUnitHandler {
    /// Arena flavour. The units pool emitter dispatches the actual emit
    /// (`serialize_plane_angle_unit_with_id`, SI or CBU variant); this fresh-id
    /// `write` is the trait contract.
    type WriteInput = PlaneAngleFlavor;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_plane_angle_unit(entity_id, parts)?;
        lower::lower_plane_angle(ctx, entity_id, &early);
        Ok(())
    }

    /// Fresh-id SI serialize (trait contract). The units pool emitter calls
    /// `serialize_plane_angle_unit_with_id` directly (SI / CBU variant), so this
    /// is not on the hot path.
    fn write(buf: &mut WriteBuffer, _flavor: PlaneAngleFlavor) -> Result<u64, WriteError> {
        Ok(serialize::serialize_plane_angle_unit(
            buf,
            &lift::lift_plane_angle_si(),
        ))
    }
}
