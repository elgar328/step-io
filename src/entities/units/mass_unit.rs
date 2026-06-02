//! `MASS_UNIT` handler — Pass 0-1 leaf for mass flavour.
//!
//! Mirrors the `LENGTH` / `PLANE_ANGLE` / `SOLID_ANGLE` leaves: dispatch keys on
//! the `MASS_UNIT` part, the SI branch reads `(prefix, name)` from
//! `SI_UNIT`, the CBU branch reads the conversion name from
//! `CONVERSION_BASED_UNIT`. Recognised forms:
//!
//! - SI `(KILO, GRAM)` → [`MassUnit::Kilogram`]
//! - SI `(None, GRAM)` → [`MassUnit::Gram`]
//! - CBU `'POUND'`     → [`MassUnit::Pound`]
//! - CBU `'GRAM'`      → [`MassUnit::Gram`] (0.001 of the SI kilogram)
//!
//! Any other SI spelling or CBU name is dropped with a warning rather than
//! being faked as Kilogram — mirroring `length_unit` / `plane_angle_unit`'s
//! unrecognized-CBU policy. The named-unit arena loses the entry, but
//! downstream code never sees a misrepresentation of magnitude.

use crate::entities::ComplexEntityHandler;
use crate::entities::units::shared::{
    CbuFlavor, has_part, read_conversion_based_unit_body, read_optional_enum,
};
use crate::ir::attr::{check_count, read_enum};
use crate::ir::error::ConvertError;
use crate::ir::units::{MassFlavor, MassUnit, NamedUnit};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct MassUnitHandler;

#[step_entity_complex(name = "MASS_UNIT", pass = Pass0Leaf, cases = [
    ["CONVERSION_BASED_UNIT", "MASS_UNIT", "NAMED_UNIT"],
    ["MASS_UNIT", "NAMED_UNIT", "SI_UNIT"],
])]
impl ComplexEntityHandler for MassUnitHandler {
    type WriteInput = (MassUnit, u64, u64);

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        if has_part(parts, "CONVERSION_BASED_UNIT") {
            read_conversion_based_unit_body(ctx, entity_id, parts, CbuFlavor::Mass)?;
            let dim_exp = super::shared::read_named_unit_dim_exp(ctx, parts);
            register_named_mass(ctx, entity_id, None, dim_exp);
            return Ok(());
        }
        if !has_part(parts, "SI_UNIT") {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: "MASS_UNIT complex carries neither SI_UNIT nor CONVERSION_BASED_UNIT"
                    .into(),
            });
            return Ok(());
        }
        let si_attrs = require_part_attrs(parts, "SI_UNIT", entity_id)?;
        check_count(si_attrs, 2, entity_id, "SI_UNIT")?;
        let prefix = read_optional_enum(si_attrs, 0, entity_id, "prefix")?;
        let name = read_enum(si_attrs, 1, entity_id, "name")?;
        let unit = match (prefix, name) {
            (Some("KILO"), "GRAM") => MassUnit::Kilogram,
            (None, "GRAM") => MassUnit::Gram,
            _ => {
                // Unsupported SI mass spelling (e.g. (MEGA, GRAM)). Drop
                // rather than fall back to Kilogram — fake matching
                // misrepresents the magnitude (1 Mg ≠ 1 kg).
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("unsupported SI mass unit (prefix={prefix:?}, name={name:?})"),
                });
                return Ok(());
            }
        };
        ctx.mass_unit_map.insert(entity_id, unit);
        let dim_exp = super::shared::read_named_unit_dim_exp(ctx, parts);
        register_named_mass(ctx, entity_id, None, dim_exp);
        Ok(())
    }

    /// Emit a **plain SI** mass unit. CBU outers (Pound) go through
    /// [`emit_mass_cbu_outer`] which takes the pre-emitted base step id.
    /// Pound reaching this path is a kernel-built IR mistake — fall back
    /// to Kilogram emit.
    fn write(
        buf: &mut WriteBuffer,
        (unit, target_id, dim_exp_step): (MassUnit, u64, u64),
    ) -> Result<u64, WriteError> {
        let prefix = match unit {
            MassUnit::Gram => None,
            // Kilogram / Pound fallback both emit KILO-GRAM.
            MassUnit::Kilogram | MassUnit::Pound => Some("KILO"),
        };
        emit_plain_si_mass(buf, prefix, target_id, dim_exp_step);
        Ok(target_id)
    }
}

/// Push a `NamedUnit::Mass` entry once the SI / CBU branch has resolved
/// the unit into `mass_unit_map`. Mirrors `register_named_length` —
/// `cbu_base` is set to `None` here and patched by the post-Pass0Leaf
/// `backfill_cbu_base` once the outer↔base SI link is known.
fn register_named_mass(
    ctx: &mut ReaderContext,
    entity_id: u64,
    cbu_base: Option<crate::ir::id::NamedUnitId>,
    dim_exp: Option<crate::ir::DimensionalExponentsId>,
) {
    if let Some(&unit) = ctx.mass_unit_map.get(&entity_id) {
        let flavor = MassFlavor {
            unit,
            cbu_base,
            dim_exp,
        };
        let id = ctx.named_units_arena.push(NamedUnit::Mass(flavor));
        ctx.named_unit_id_map.insert(entity_id, id);
    }
}

/// Emit a `CONVERSION_BASED_UNIT` mass outer at `target_id` wrapping the
/// already-emitted base SI kilogram at `base_step` — Pound (0.45359237) or
/// gram (0.001). Returns `Result` to mirror the dispatcher signature.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn emit_mass_cbu_outer(
    buf: &mut WriteBuffer,
    unit: MassUnit,
    base_step: u64,
    target_id: u64,
    dim_exp_step: u64,
) -> Result<u64, WriteError> {
    let (name, factor) = match unit {
        MassUnit::Pound => ("POUND", 0.453_592_37),
        MassUnit::Gram => ("GRAM", 0.001),
        // Kilogram reaching the CBU path is unexpected (kernel-built IR);
        // fall back to the already-emitted base step id (no extra entity).
        MassUnit::Kilogram => return Ok(base_step),
    };
    // Reference the flavour's own DE (IR arena) when present so the round-trip
    // is idempotent; only synthesize for kernel-built IR. See
    // `length_unit::emit_conversion_based_length`.
    let dim_exp = if dim_exp_step != 0 {
        dim_exp_step
    } else {
        emit_mass_dim_exponents(buf)
    };
    let measure = buf.fresh();
    buf.entities.push(WriterEntity {
        id: measure,
        body: WriterBody::Simple {
            name: "MASS_MEASURE_WITH_UNIT".into(),
            attrs: vec![
                Attribute::Typed {
                    type_name: "MASS_MEASURE".into(),
                    value: Box::new(Attribute::Real(factor)),
                },
                Attribute::EntityRef(base_step),
            ],
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
                ("MASS_UNIT".into(), vec![]),
                ("NAMED_UNIT".into(), vec![Attribute::EntityRef(dim_exp)]),
            ],
        },
    });
    Ok(target_id)
}

fn emit_plain_si_mass(
    buf: &mut WriteBuffer,
    prefix: Option<&'static str>,
    target_id: u64,
    dim_exp_step: u64,
) {
    let prefix_attr = match prefix {
        Some(p) => Attribute::Enum(p.into()),
        None => Attribute::Unset,
    };
    let named_unit_attr = if dim_exp_step == 0 {
        Attribute::Derived
    } else {
        Attribute::EntityRef(dim_exp_step)
    };
    buf.entities.push(WriterEntity {
        id: target_id,
        body: WriterBody::Complex {
            parts: vec![
                ("MASS_UNIT".into(), vec![]),
                ("NAMED_UNIT".into(), vec![named_unit_attr]),
                (
                    "SI_UNIT".into(),
                    vec![prefix_attr, Attribute::Enum("GRAM".into())],
                ),
            ],
        },
    });
}

fn emit_mass_dim_exponents(buf: &mut WriteBuffer) -> u64 {
    if let Some(id) = buf.mass_dim_exp_step {
        return id;
    }
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Simple {
            name: "DIMENSIONAL_EXPONENTS".into(),
            attrs: vec![
                Attribute::Real(0.0),
                Attribute::Real(0.0),
                Attribute::Real(1.0),
                Attribute::Real(0.0),
                Attribute::Real(0.0),
                Attribute::Real(0.0),
                Attribute::Real(0.0),
            ],
        },
    });
    buf.mass_dim_exp_step = Some(n);
    n
}
