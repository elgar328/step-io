//! `LENGTH_UNIT` handler leaf for length flavour.
//!
//! Both cases (SI / `CONVERSION_BASED_UNIT`) are generated 2-layer
//! (units-CBU-②): `bind_length_unit` yields the `EarlyLengthUnit` enum and
//! `lower_length` dispatches to the SI / CBU lowering (unit identification +
//! `cbu_base` / `cbu_factor_mwu_id` resolution live in `lower`). The CBU outer
//! is emitted via the generated `serialize_length_unit_with_id` (CBU variant)
//! by the units pool emitter (`writer::buffer::units`).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::ComplexEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::units::LengthFlavor;
use crate::parser::entity::RawEntityPart;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct LengthUnitHandler;

#[step_entity_complex(name = "LENGTH_UNIT", cases = [
    ["CONVERSION_BASED_UNIT", "LENGTH_UNIT", "NAMED_UNIT"],
    ["LENGTH_UNIT", "NAMED_UNIT", "SI_UNIT"],
])]
impl ComplexEntityHandler for LengthUnitHandler {
    /// Arena flavour. The units pool emitter dispatches the actual emit
    /// (`serialize_length_unit_with_id`, SI or CBU variant); this fresh-id
    /// `write` is the trait contract.
    type WriteInput = LengthFlavor;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_length_unit(entity_id, parts)?;
        lower::lower_length(ctx, entity_id, &early);
        Ok(())
    }

    /// Fresh-id SI serialize (trait contract). The units pool emitter calls
    /// `serialize_length_unit_with_id` directly (SI / CBU variant), so this is
    /// not on the hot path.
    fn write(buf: &mut WriteBuffer, flavor: LengthFlavor) -> Result<u64, WriteError> {
        Ok(serialize::serialize_length_unit(
            buf,
            &lift::lift_length_si(flavor.unit),
        ))
    }
}
