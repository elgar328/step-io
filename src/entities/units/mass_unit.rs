//! `MASS_UNIT` handler leaf for mass flavour.
//!
//! Both cases (SI / `CONVERSION_BASED_UNIT`) are generated 2-layer
//! (units-CBU-②): `bind_mass_unit` yields the `EarlyMassUnit` enum and
//! `lower_mass` dispatches to the SI / CBU lowering. CBU unit identification is
//! **by conversion factor** (fixed kg base) in `lower_mass_cbu`; the CBU outer
//! is emitted via the generated `serialize_mass_unit_with_id` by the units pool
//! emitter (`writer::buffer::units`).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::ComplexEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::units::MassFlavor;
use crate::parser::entity::{EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct MassUnitHandler;

#[step_entity_complex(name = "MASS_UNIT", cases = [
    ["CONVERSION_BASED_UNIT", "MASS_UNIT", "NAMED_UNIT"],
    ["MASS_UNIT", "NAMED_UNIT", "SI_UNIT"],
])]
impl ComplexEntityHandler for MassUnitHandler {
    /// Arena flavour. The units pool emitter dispatches the actual emit
    /// (`serialize_mass_unit_with_id`, SI or CBU variant); this fresh-id `write`
    /// is the trait contract.
    type WriteInput = MassFlavor;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_mass_unit(entity_id, parts)?;
        lower::lower_mass(ctx, entity_id, &early);
        Ok(())
    }

    /// Fresh-id SI serialize (trait contract). The units pool emitter calls
    /// `serialize_mass_unit_with_id` directly (SI / CBU variant), so this is not
    /// on the hot path.
    fn write(buf: &mut WriteBuffer, flavor: MassFlavor) -> Result<u64, WriteError> {
        Ok(serialize::serialize_mass_unit(
            buf,
            &lift::lift_mass_si(flavor.unit),
        ))
    }
}
