//! Bare `NAMED_UNIT(#dimensions)` handler — a dimensionless/count unit.
//!
//! `NAMED_UNIT` is the abstract supertype of the typed units, but it is also
//! directly instantiable as a simple entity carrying just the inherited
//! `dimensions` attribute (e.g. `NAMED_UNIT(#dims)` with all-zero exponents =
//! a count unit). This is the named-unit analogue of bare `MEASURE_WITH_UNIT`.
//! Shares the `named_units` arena via [`NamedUnit::Itself`]; coexists with the
//! complex `(MASS_UNIT()NAMED_UNIT()SI_UNIT())` handlers (simple vs complex
//! dispatch, mirroring `RATIO_UNIT`).

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::units::{NamedUnit, NamedUnitData};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct NamedUnitSimpleHandler;

#[step_entity(name = "NAMED_UNIT")]
impl SimpleEntityHandler for NamedUnitSimpleHandler {
    /// `(target_id, dim_exp_step)` — reserved step id + resolved
    /// `DIMENSIONAL_EXPONENTS` ref (`0` → emit `$`).
    type WriteInput = (u64, u64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "NAMED_UNIT")?;
        let dimensions = match attrs.first() {
            Some(Attribute::EntityRef(n)) => ctx.dim_exp_id_map.get(n).copied(),
            _ => None, // `$` (Unset) / `*` (Derived) — no explicit dimensions
        };
        let id = ctx
            .named_units_arena
            .push(NamedUnit::Itself(NamedUnitData { dimensions }));
        ctx.named_unit_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (target_id, dim_exp_step): (u64, u64),
    ) -> Result<u64, WriteError> {
        let dim_attr = if dim_exp_step == 0 {
            Attribute::Unset
        } else {
            Attribute::EntityRef(dim_exp_step)
        };
        buf.push_simple_with_id(target_id, "NAMED_UNIT", vec![dim_attr]);
        Ok(target_id)
    }
}
