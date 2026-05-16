//! `SOLID_ANGLE_UNIT` handler — Pass 0-1 leaf for solid-angle flavour.
//!
//! Sister of `LengthUnitHandler` / `PlaneAngleUnitHandler`. Catalog
//! group: `units` (O, part-only). `CONVERSION_BASED_UNIT` form for
//! solid angle is unobserved in fixtures, so the handler covers only
//! the SI path; `WriteInput` carries no `cbu_wrapped` flag.

use crate::entities::ComplexEntityHandler;
use crate::entities::units::shared::{
    emit_dimensionless_exponents, has_part, match_solid_angle_unit, read_optional_enum,
};
use crate::ir::attr::{check_count, read_enum};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::SolidAngleUnit;
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, find_part_attrs, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct SolidAngleUnitHandler;

#[step_entity_complex(name = "SOLID_ANGLE_UNIT", pass = Pass0Leaf, required = ["SOLID_ANGLE_UNIT"])]
impl ComplexEntityHandler for SolidAngleUnitHandler {
    /// `(unit, dim_exp_explicit)`. No `cbu_wrapped` flag — solid-angle
    /// CBU forms are unobserved.
    type WriteInput = (SolidAngleUnit, bool);

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // SOLID_ANGLE_UNIT + CONVERSION_BASED_UNIT is theoretically possible
        // but not observed in practice; ignore silently to mirror the legacy
        // convert_unit_leaf behaviour.
        if has_part(parts, "CONVERSION_BASED_UNIT") {
            return Ok(());
        }

        if !has_part(parts, "SI_UNIT") {
            return Ok(());
        }

        if let Some(named_attrs) = find_part_attrs(parts, "NAMED_UNIT")
            && let Some(Attribute::EntityRef(_)) = named_attrs.first()
        {
            ctx.dim_exp_explicit = true;
        }

        let si_attrs = require_part_attrs(parts, "SI_UNIT", entity_id)?;
        check_count(si_attrs, 2, entity_id, "SI_UNIT")?;
        let prefix = read_optional_enum(si_attrs, 0, entity_id, "prefix")?;
        let name = read_enum(si_attrs, 1, entity_id, "name")?;

        if let Some(unit) = match_solid_angle_unit(prefix, name) {
            ctx.solid_angle_unit_map.insert(entity_id, unit);
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "unsupported SI solid-angle unit (prefix={prefix:?}, name={name:?})"
                ),
            });
        }
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (unit, dim_exp_explicit): (SolidAngleUnit, bool),
    ) -> Result<u64, WriteError> {
        if let Some(&n) = buf.solid_angle_unit_ids.get(&unit) {
            return Ok(n);
        }
        let dim_exp_attr = if dim_exp_explicit {
            Attribute::EntityRef(emit_dimensionless_exponents(buf))
        } else {
            Attribute::Derived
        };
        let parts = vec![
            (
                "SI_UNIT".into(),
                vec![Attribute::Unset, Attribute::Enum("STERADIAN".into())],
            ),
            ("NAMED_UNIT".into(), vec![dim_exp_attr]),
            ("SOLID_ANGLE_UNIT".into(), vec![]),
        ];
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex { parts },
        });
        buf.solid_angle_unit_ids.insert(unit, n);
        Ok(n)
    }
}
