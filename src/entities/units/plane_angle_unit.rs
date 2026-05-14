//! `PLANE_ANGLE_UNIT` handler — Pass 0-1 leaf for plane-angle flavour.
//!
//! Sister of [`crate::entities::units::length_unit::LengthUnitHandler`].
//! Catalog group: `units` (O, part-only — `REQUIRED_PARTS` dispatch keys
//! on `PLANE_ANGLE_UNIT`).

use crate::entities::units::shared::{
    emit_dimensionless_exponents, has_part, match_angle_unit, read_conversion_based_unit_body,
    read_optional_enum,
};
use crate::entities::{
    ComplexEntityHandler, ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind,
};
use crate::ir::attr::{check_count, read_enum};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::AngleUnit;
use crate::parser::entity::{Attribute, RawEntityPart};
use crate::reader::{ReaderContext, find_part_attrs, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct PlaneAngleUnitHandler;

impl ComplexEntityHandler for PlaneAngleUnitHandler {
    const NAME: &'static str = "PLANE_ANGLE_UNIT";
    const PASS_LEVEL: PassLevel = PassLevel::Pass0Leaf;
    const REQUIRED_PARTS: &'static [&'static str] = &["PLANE_ANGLE_UNIT"];
    /// `(unit, plane_angle_cbu_wrapped, dim_exp_explicit)`.
    type WriteInput = (AngleUnit, bool, bool);

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
    ) -> Result<(), ConvertError> {
        if has_part(parts, "CONVERSION_BASED_UNIT") {
            return read_conversion_based_unit_body(ctx, entity_id, parts, false, true);
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

        if let Some(unit) = match_angle_unit(prefix, name) {
            ctx.angle_unit_map.insert(entity_id, unit);
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("unsupported SI angle unit (prefix={prefix:?}, name={name:?})"),
            });
        }
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (unit, cbu_wrapped, dim_exp_explicit): (AngleUnit, bool, bool),
    ) -> Result<u64, WriteError> {
        let key = (unit, cbu_wrapped);
        if let Some(&n) = buf.angle_unit_ids.get(&key) {
            return Ok(n);
        }
        let n = match unit {
            AngleUnit::Radian if cbu_wrapped => {
                emit_conversion_based_angle(buf, "RADIAN", 1.0, dim_exp_explicit)
            }
            AngleUnit::Radian => emit_plain_si_radian(buf, dim_exp_explicit),
            AngleUnit::Degree => emit_conversion_based_angle(
                buf,
                "DEGREE",
                std::f64::consts::PI / 180.0,
                dim_exp_explicit,
            ),
        };
        buf.angle_unit_ids.insert(key, n);
        Ok(n)
    }
}

fn emit_plain_si_radian(buf: &mut WriteBuffer, dim_exp_explicit: bool) -> u64 {
    let dim_exp_attr = if dim_exp_explicit {
        Attribute::EntityRef(emit_dimensionless_exponents(buf))
    } else {
        Attribute::Derived
    };
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Complex {
            parts: vec![
                (
                    "SI_UNIT".into(),
                    vec![Attribute::Unset, Attribute::Enum("RADIAN".into())],
                ),
                ("NAMED_UNIT".into(), vec![dim_exp_attr]),
                ("PLANE_ANGLE_UNIT".into(), vec![]),
            ],
        },
    });
    n
}

/// Emit a bare SI radian entity used as the base inside a Degree
/// `CONVERSION_BASED_UNIT` chain. Mirrors `emit_plain_si_radian`'s
/// `dim_exp_explicit` branching for ABC-tier loyalty.
fn emit_base_si_radian(buf: &mut WriteBuffer, dim_exp_explicit: bool) -> u64 {
    let dim_exp_attr = if dim_exp_explicit {
        Attribute::EntityRef(emit_dimensionless_exponents(buf))
    } else {
        Attribute::Derived
    };
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Complex {
            parts: vec![
                ("NAMED_UNIT".into(), vec![dim_exp_attr]),
                ("PLANE_ANGLE_UNIT".into(), vec![]),
                (
                    "SI_UNIT".into(),
                    vec![Attribute::Unset, Attribute::Enum("RADIAN".into())],
                ),
            ],
        },
    });
    n
}

/// Emit a `CONVERSION_BASED_UNIT` plane-angle chain. Used for genuine
/// non-SI angles (Degree — factor π/180) and for SI self-wrap (Radian
/// — factor 1.0). Base is always plain SI RADIAN.
fn emit_conversion_based_angle(
    buf: &mut WriteBuffer,
    name: &str,
    factor: f64,
    dim_exp_explicit: bool,
) -> u64 {
    let base_si = emit_base_si_radian(buf, dim_exp_explicit);
    let dim_exp = emit_dimensionless_exponents(buf);
    let measure = buf.fresh();
    buf.entities.push(WriterEntity {
        id: measure,
        body: WriterBody::Simple {
            name: "PLANE_ANGLE_MEASURE_WITH_UNIT".into(),
            attrs: vec![Attribute::Real(factor), Attribute::EntityRef(base_si)],
        },
    });
    let outer = buf.fresh();
    buf.entities.push(WriterEntity {
        id: outer,
        body: WriterBody::Complex {
            parts: vec![
                (
                    "CONVERSION_BASED_UNIT".into(),
                    vec![
                        Attribute::String(name.into()),
                        Attribute::EntityRef(measure),
                    ],
                ),
                ("NAMED_UNIT".into(), vec![Attribute::EntityRef(dim_exp)]),
                ("PLANE_ANGLE_UNIT".into(), vec![]),
            ],
        },
    });
    outer
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static PLANE_ANGLE_UNIT_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: PlaneAngleUnitHandler::NAME,
    pass_level: PlaneAngleUnitHandler::PASS_LEVEL,
    kind: ReadKind::Complex {
        required_parts: PlaneAngleUnitHandler::REQUIRED_PARTS,
        read: PlaneAngleUnitHandler::read_complex,
    },
};
