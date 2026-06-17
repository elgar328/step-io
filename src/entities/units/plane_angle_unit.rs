//! `PLANE_ANGLE_UNIT` handler leaf for plane-angle flavour.
//!
//! Sister of [`crate::entities::units::length_unit::LengthUnitHandler`].
//! Catalog group: `units` (O, part-only — `REQUIRED_PARTS` dispatch keys
//! on `PLANE_ANGLE_UNIT`).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::ComplexEntityHandler;
use crate::entities::units::shared::{
    CbuFlavor, emit_dimensionless_exponents, has_part, read_conversion_based_unit_body,
};
use crate::ir::error::ConvertError;
use crate::ir::id::NamedUnitId;
use crate::ir::shape_rep::AngleUnit;
use crate::ir::units::{NamedUnit, PlaneAngleFlavor};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct PlaneAngleUnitHandler;

#[step_entity_complex(name = "PLANE_ANGLE_UNIT", cases = [
    ["CONVERSION_BASED_UNIT", "NAMED_UNIT", "PLANE_ANGLE_UNIT"],
    ["NAMED_UNIT", "PLANE_ANGLE_UNIT", "SI_UNIT"],
])]
impl ComplexEntityHandler for PlaneAngleUnitHandler {
    /// Arena flavour. The units pool emitter dispatches the actual emit
    /// (SI plain via `serialize_plane_angle_unit_with_id`, CBU via
    /// `emit_plane_angle_cbu_outer`); this fresh-id `write` is the trait contract.
    type WriteInput = PlaneAngleFlavor;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // CONVERSION_BASED_UNIT (Degree, or CBU-wrapped radian) takes precedence:
        // the CBU name is the authoritative identity. CBU path stays hand-written
        // (graph-walk + backfill); SI path is 2-layer.
        if has_part(parts, "CONVERSION_BASED_UNIT") {
            read_conversion_based_unit_body(ctx, entity_id, parts, CbuFlavor::PlaneAngle, graph)?;
            let dim_exp = super::shared::read_named_unit_dim_exp(ctx, parts);
            register_named_plane_angle(ctx, entity_id, None, dim_exp);
            return Ok(());
        }
        let early = bind::bind_plane_angle_unit(entity_id, parts)?;
        lower::lower_plane_angle_si(ctx, entity_id, &early);
        Ok(())
    }

    /// Fresh-id SI serialize (trait contract). The units pool emitter calls
    /// `serialize_plane_angle_unit_with_id` directly for the plain SI path and
    /// `emit_plane_angle_cbu_outer` for the CBU path, so this is not on the hot
    /// path.
    fn write(buf: &mut WriteBuffer, _flavor: PlaneAngleFlavor) -> Result<u64, WriteError> {
        Ok(serialize::serialize_plane_angle_unit(
            buf,
            &lift::lift_plane_angle_si(),
        ))
    }
}

/// Emit a `CONVERSION_BASED_UNIT` plane-angle outer at `target_id` wrapping
/// the already-emitted base SI radian at `base_step`.
pub(crate) fn emit_plane_angle_cbu_outer(
    buf: &mut WriteBuffer,
    unit: AngleUnit,
    base_step: u64,
    target_id: u64,
    dim_exp_step: u64,
) -> u64 {
    let (name, factor) = match unit {
        AngleUnit::Radian => ("RADIAN", 1.0),
        AngleUnit::Degree => ("DEGREE", std::f64::consts::PI / 180.0),
    };
    emit_conversion_based_angle(buf, name, factor, base_step, target_id, dim_exp_step)
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
        ctx.id_cache.insert(entity_id, id);
    }
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
    dim_exp_step: u64,
) -> u64 {
    // Reference the flavour's own DE (IR arena) when present so the round-trip
    // is idempotent; only synthesize for kernel-built IR. See
    // `length_unit::emit_conversion_based_length`.
    let dim_exp = if dim_exp_step != 0 {
        dim_exp_step
    } else {
        emit_dimensionless_exponents(buf)
    };
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
