//! `SOLID_ANGLE_UNIT` handler — Pass 0-1 leaf for solid-angle flavour.
//!
//! Sister of `LengthUnitHandler` / `PlaneAngleUnitHandler`. Catalog
//! group: `units` (O, part-only). `CONVERSION_BASED_UNIT` form for
//! solid angle is unobserved in fixtures, so the handler covers only
//! the SI path; `WriteInput` carries no `cbu_wrapped` flag.

use crate::entities::ComplexEntityHandler;
use crate::entities::units::shared::{has_part, match_solid_angle_unit, read_optional_enum};
use crate::ir::attr::{check_count, read_enum};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::SolidAngleUnit;
use crate::ir::units::{NamedUnit, SolidAngleFlavor};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct SolidAngleUnitHandler;

#[step_entity_complex(name = "SOLID_ANGLE_UNIT", pass = Pass0Leaf, required = ["SOLID_ANGLE_UNIT"])]
impl ComplexEntityHandler for SolidAngleUnitHandler {
    /// `(unit, target_id)`. No `cbu_wrapped` / `dim_exp_explicit` flags —
    /// solid-angle CBU forms are unobserved and `NAMED_UNIT.dimensions`
    /// is canonical Derived.
    type WriteInput = (SolidAngleUnit, u64, u64);

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // SOLID_ANGLE_UNIT + CONVERSION_BASED_UNIT is theoretically possible
        // but not observed in practice; drop with a warning so the caller
        // can see what was lost.
        if has_part(parts, "CONVERSION_BASED_UNIT") {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: "SOLID_ANGLE_UNIT + CONVERSION_BASED_UNIT is unsupported".into(),
            });
            return Ok(());
        }

        if !has_part(parts, "SI_UNIT") {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail:
                    "SOLID_ANGLE_UNIT complex carries neither SI_UNIT nor CONVERSION_BASED_UNIT"
                        .into(),
            });
            return Ok(());
        }

        let si_attrs = require_part_attrs(parts, "SI_UNIT", entity_id)?;
        check_count(si_attrs, 2, entity_id, "SI_UNIT")?;
        let prefix = read_optional_enum(si_attrs, 0, entity_id, "prefix")?;
        let name = read_enum(si_attrs, 1, entity_id, "name")?;

        if let Some(unit) = match_solid_angle_unit(prefix, name) {
            ctx.solid_angle_unit_map.insert(entity_id, unit);
            let dim_exp = super::shared::read_named_unit_dim_exp(ctx, parts);
            let flavor = SolidAngleFlavor { unit, dim_exp };
            let id = ctx.named_units_arena.push(NamedUnit::SolidAngle(flavor));
            ctx.named_unit_id_map.insert(entity_id, id);
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
        (_unit, target_id, dim_exp_step): (SolidAngleUnit, u64, u64),
    ) -> Result<u64, WriteError> {
        let named_unit_attr = if dim_exp_step == 0 {
            Attribute::Derived
        } else {
            Attribute::EntityRef(dim_exp_step)
        };
        let parts = vec![
            (
                "SI_UNIT".into(),
                vec![Attribute::Unset, Attribute::Enum("STERADIAN".into())],
            ),
            ("NAMED_UNIT".into(), vec![named_unit_attr]),
            ("SOLID_ANGLE_UNIT".into(), vec![]),
        ];
        buf.entities.push(WriterEntity {
            id: target_id,
            body: WriterBody::Complex { parts },
        });
        Ok(target_id)
    }
}
