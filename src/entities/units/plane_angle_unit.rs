//! `PLANE_ANGLE_UNIT` handler — Pass 0-1 leaf for plane-angle flavour.
//!
//! Sister of [`crate::entities::units::length_unit::LengthUnitHandler`].
//! Catalog group: `units` (O, part-only — `REQUIRED_PARTS` dispatch keys
//! on `PLANE_ANGLE_UNIT`).

use crate::entities::ComplexEntityHandler;
use crate::entities::units::shared::{
    CbuFlavor, emit_dimensionless_exponents, has_part, match_angle_unit,
    read_conversion_based_unit_body, read_optional_enum,
};
use crate::ir::attr::{check_count, read_enum};
use crate::ir::error::ConvertError;
use crate::ir::id::NamedUnitId;
use crate::ir::shape_rep::AngleUnit;
use crate::ir::units::{NamedUnit, PlaneAngleFlavor};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct PlaneAngleUnitHandler;

#[step_entity_complex(name = "PLANE_ANGLE_UNIT", pass = Pass0Leaf, required = ["PLANE_ANGLE_UNIT"])]
impl ComplexEntityHandler for PlaneAngleUnitHandler {
    /// `(unit, target_id)`.
    type WriteInput = (AngleUnit, u64, u64);

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        if has_part(parts, "CONVERSION_BASED_UNIT") {
            read_conversion_based_unit_body(ctx, entity_id, parts, CbuFlavor::PlaneAngle)?;
            let dim_exp = super::shared::read_named_unit_dim_exp(ctx, parts);
            register_named_plane_angle(ctx, entity_id, None, dim_exp);
            return Ok(());
        }

        if !has_part(parts, "SI_UNIT") {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail:
                    "PLANE_ANGLE_UNIT complex carries neither SI_UNIT nor CONVERSION_BASED_UNIT"
                        .into(),
            });
            return Ok(());
        }

        let si_attrs = require_part_attrs(parts, "SI_UNIT", entity_id)?;
        check_count(si_attrs, 2, entity_id, "SI_UNIT")?;
        let prefix = read_optional_enum(si_attrs, 0, entity_id, "prefix")?;
        let name = read_enum(si_attrs, 1, entity_id, "name")?;

        if let Some(unit) = match_angle_unit(prefix, name) {
            ctx.angle_unit_map.insert(entity_id, unit);
            let dim_exp = super::shared::read_named_unit_dim_exp(ctx, parts);
            register_named_plane_angle(ctx, entity_id, None, dim_exp);
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("unsupported SI angle unit (prefix={prefix:?}, name={name:?})"),
            });
        }
        Ok(())
    }

    /// Emit plain SI radian only — CBU outers (Degree, CBU(RADIAN)) go
    /// through [`emit_plane_angle_cbu_outer`] (units-2 2-pass writer).
    /// Plain SI emission only. Degree (non-SI) and CBU(RADIAN) self-wrap
    /// go through [`emit_plane_angle_cbu_outer`]; if Degree ever reaches
    /// this path (kernel-built IR misuse) we emit plain RADIAN as a
    /// fallback rather than panic.
    fn write(
        buf: &mut WriteBuffer,
        (_unit, target_id, dim_exp_step): (AngleUnit, u64, u64),
    ) -> Result<u64, WriteError> {
        emit_plain_si_radian(buf, target_id, dim_exp_step);
        Ok(target_id)
    }
}

/// Emit a `CONVERSION_BASED_UNIT` plane-angle outer at `target_id` wrapping
/// the already-emitted base SI radian at `base_step`.
pub(crate) fn emit_plane_angle_cbu_outer(
    buf: &mut WriteBuffer,
    unit: AngleUnit,
    base_step: u64,
    target_id: u64,
) -> u64 {
    let (name, factor) = match unit {
        AngleUnit::Radian => ("RADIAN", 1.0),
        AngleUnit::Degree => ("DEGREE", std::f64::consts::PI / 180.0),
    };
    emit_conversion_based_angle(buf, name, factor, base_step, target_id)
}

/// See `length_unit::register_named_length` for the rationale.
fn register_named_plane_angle(
    ctx: &mut ReaderContext,
    entity_id: u64,
    cbu_base: Option<NamedUnitId>,
    dim_exp: Option<crate::ir::DimensionalExponentsId>,
) {
    if let Some(&unit) = ctx.angle_unit_map.get(&entity_id) {
        let flavor = PlaneAngleFlavor {
            unit,
            cbu_base,
            dim_exp,
        };
        let id = ctx.named_units_arena.push(NamedUnit::PlaneAngle(flavor));
        ctx.named_unit_id_map.insert(entity_id, id);
    }
}

/// Canonical plain SI radian — `NAMED_UNIT.dimensions` is `*` Derived
/// (units-3b dropped the input-preserving explicit-DE flag).
fn emit_plain_si_radian(buf: &mut WriteBuffer, target_id: u64, dim_exp_step: u64) {
    let named_unit_attr = if dim_exp_step == 0 {
        Attribute::Derived
    } else {
        Attribute::EntityRef(dim_exp_step)
    };
    buf.entities.push(WriterEntity {
        id: target_id,
        body: WriterBody::Complex {
            parts: vec![
                (
                    "SI_UNIT".into(),
                    vec![Attribute::Unset, Attribute::Enum("RADIAN".into())],
                ),
                ("NAMED_UNIT".into(), vec![named_unit_attr]),
                ("PLANE_ANGLE_UNIT".into(), vec![]),
            ],
        },
    });
}

/// Emit a `CONVERSION_BASED_UNIT` plane-angle outer at `target_id` wrapping
/// the already-emitted base SI at `base_step`. Used for Degree (factor π/180)
/// and CBU(RADIAN) self-wrap (factor 1.0).
fn emit_conversion_based_angle(
    buf: &mut WriteBuffer,
    name: &str,
    factor: f64,
    base_step: u64,
    target_id: u64,
) -> u64 {
    let dim_exp = emit_dimensionless_exponents(buf);
    let measure = buf.fresh();
    buf.entities.push(WriterEntity {
        id: measure,
        body: WriterBody::Simple {
            name: "PLANE_ANGLE_MEASURE_WITH_UNIT".into(),
            attrs: vec![Attribute::Real(factor), Attribute::EntityRef(base_step)],
        },
    });
    buf.entities.push(WriterEntity {
        id: target_id,
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
    target_id
}
