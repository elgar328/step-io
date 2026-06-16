//! `SOLID_ANGLE_UNIT` handler leaf for solid-angle flavour.
//!
//! Sister of `LengthUnitHandler` / `PlaneAngleUnitHandler`. Catalog
//! group: `units` (O, part-only). `CONVERSION_BASED_UNIT` form for
//! solid angle is unobserved in fixtures, so the handler covers only
//! the SI path; `WriteInput` carries no `cbu_wrapped` flag.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::ComplexEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::units::SolidAngleFlavor;
use crate::parser::entity::{EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct SolidAngleUnitHandler;

#[step_entity_complex(name = "SOLID_ANGLE_UNIT", cases = [
    ["NAMED_UNIT", "SI_UNIT", "SOLID_ANGLE_UNIT"],
])]
impl ComplexEntityHandler for SolidAngleUnitHandler {
    /// Arena flavour. The units pool emitter pre-reserves the `NamedUnit` step id
    /// and calls `serialize_solid_angle_unit_with_id` directly (see
    /// `emit_named_unit_plain`); this fresh-id `write` exists for the trait
    /// contract.
    type WriteInput = SolidAngleFlavor;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_solid_angle_unit(entity_id, parts)?;
        lower::lower_solid_angle_unit(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, _flavor: SolidAngleFlavor) -> Result<u64, WriteError> {
        Ok(serialize::serialize_solid_angle_unit(
            buf,
            &lift::lift_solid_angle_unit(),
        ))
    }
}
