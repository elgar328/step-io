//! `DIMENSIONAL_EXPONENTS` handler — phase dim-exp-arena-a.
//!
//! Schema-faithful `single_struct`: the seven SI base-quantity exponents
//! as `REAL`. Read before the `NAMED_UNIT` subtype handlers (added in
//! phase dim-exp-arena-b) so they can resolve their `dimensions` ref
//! through `dim_exp_id_map`.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_real};
use crate::ir::error::ConvertError;
use crate::ir::units::DimensionalExponents;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DimensionalExponentsHandler;

#[step_entity(name = "DIMENSIONAL_EXPONENTS")]
impl SimpleEntityHandler for DimensionalExponentsHandler {
    type WriteInput = DimensionalExponents;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 7, entity_id, "DIMENSIONAL_EXPONENTS")?;
        let dim_exp = DimensionalExponents {
            length_exponent: read_real(attrs, 0, entity_id, "length_exponent")?,
            mass_exponent: read_real(attrs, 1, entity_id, "mass_exponent")?,
            time_exponent: read_real(attrs, 2, entity_id, "time_exponent")?,
            electric_current_exponent: read_real(attrs, 3, entity_id, "electric_current_exponent")?,
            thermodynamic_temperature_exponent: read_real(
                attrs,
                4,
                entity_id,
                "thermodynamic_temperature_exponent",
            )?,
            amount_of_substance_exponent: read_real(
                attrs,
                5,
                entity_id,
                "amount_of_substance_exponent",
            )?,
            luminous_intensity_exponent: read_real(
                attrs,
                6,
                entity_id,
                "luminous_intensity_exponent",
            )?,
        };
        let id = ctx.dimensional_exponents.push(dim_exp);
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, de: DimensionalExponents) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "DIMENSIONAL_EXPONENTS",
            vec![
                Attribute::Real(de.length_exponent),
                Attribute::Real(de.mass_exponent),
                Attribute::Real(de.time_exponent),
                Attribute::Real(de.electric_current_exponent),
                Attribute::Real(de.thermodynamic_temperature_exponent),
                Attribute::Real(de.amount_of_substance_exponent),
                Attribute::Real(de.luminous_intensity_exponent),
            ],
        ))
    }
}
