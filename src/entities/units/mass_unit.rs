//! `MASS_UNIT` handler leaf for mass flavour.
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

use crate::early::{bind, lift, lower, serialize};
use crate::entities::ComplexEntityHandler;
use crate::entities::units::shared::{
    CbuFactorRefs, CbuFlavor, has_part, read_conversion_based_unit_body,
};
use crate::ir::error::ConvertError;
use crate::ir::units::{MassFlavor, MassUnit, NamedUnit};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct MassUnitHandler;

#[step_entity_complex(name = "MASS_UNIT", cases = [
    ["CONVERSION_BASED_UNIT", "MASS_UNIT", "NAMED_UNIT"],
    ["MASS_UNIT", "NAMED_UNIT", "SI_UNIT"],
])]
impl ComplexEntityHandler for MassUnitHandler {
    /// Arena flavour. The units pool emitter dispatches the actual emit
    /// (SI plain via `serialize_mass_unit_with_id`, CBU via
    /// `emit_mass_cbu_outer`); this fresh-id `write` is the trait contract.
    type WriteInput = MassFlavor;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // CONVERSION_BASED_UNIT (Pound / gram / ton) takes precedence: the CBU
        // name is the authoritative identity. CBU path stays hand-written
        // (graph-walk identification); the preserved conversion-factor MWU is
        // referenced via `cbu_factor_mwu_id`. SI path is 2-layer.
        if has_part(parts, "CONVERSION_BASED_UNIT") {
            let refs =
                read_conversion_based_unit_body(ctx, entity_id, parts, CbuFlavor::Mass, graph)?;
            let dim_exp = super::shared::read_named_unit_dim_exp(ctx, parts);
            register_named_mass(ctx, entity_id, refs, dim_exp);
            return Ok(());
        }
        let early = bind::bind_mass_unit(entity_id, parts)?;
        lower::lower_mass_si(ctx, entity_id, &early);
        Ok(())
    }

    /// Fresh-id SI serialize (trait contract). The units pool emitter calls
    /// `serialize_mass_unit_with_id` directly for the plain SI path and
    /// `emit_mass_cbu_outer` for the CBU path, so this is not on the hot path.
    fn write(buf: &mut WriteBuffer, flavor: MassFlavor) -> Result<u64, WriteError> {
        Ok(serialize::serialize_mass_unit(
            buf,
            &lift::lift_mass_si(flavor.unit),
        ))
    }
}

/// Push a `NamedUnit::Mass` entry once the SI / CBU branch has resolved
/// the unit into `mass_unit_map`. Mirrors `register_named_length` — CBU outers
/// receive their resolved [`CbuFactorRefs`] (base SI + preserved factor MWU)
/// inline; plain SI never reaches this path (it lowers via `lower_mass_si`).
fn register_named_mass(
    ctx: &mut ReaderContext,
    entity_id: u64,
    refs: Option<CbuFactorRefs>,
    dim_exp: Option<crate::ir::DimensionalExponentsId>,
) {
    if let Some(&unit) = ctx.mass_unit_map.get(&entity_id) {
        let (cbu_base, cbu_factor_mwu_id) =
            refs.map_or((None, None), |r| (r.cbu_base, r.cbu_factor_mwu_id));
        let flavor = MassFlavor {
            unit,
            cbu_base,
            dim_exp,
            cbu_factor_mwu_id,
        };
        let id = ctx.named_units_arena.push(NamedUnit::Mass(flavor));
        ctx.id_cache.insert(entity_id, id);
    }
}

/// Emit a `CONVERSION_BASED_UNIT` mass outer at `target_id` referencing the
/// **preserved** conversion-factor `MASS_MEASURE_WITH_UNIT` at `measure_step`
/// (emitted earlier in the units pool). The base SI kilogram is reached through
/// that MWU. Pound / gram / ton only. Kilogram / Megagram are plain SI and must
/// not reach the CBU path (`cbu_base` is `None` → plain dispatch).
pub(crate) fn emit_mass_cbu_outer(
    buf: &mut WriteBuffer,
    unit: MassUnit,
    measure_step: u64,
    target_id: u64,
    dim_exp_step: u64,
) -> Result<u64, WriteError> {
    let name = match unit {
        MassUnit::Pound => "POUND",
        MassUnit::Gram => "GRAM",
        // Lowercase 'ton' matches the corpus spelling; the reader discards the
        // CBU name string, so the writer must reproduce the source casing to
        // keep the round-trip multiset stable (uppercase "TON" would differ).
        MassUnit::Ton => "ton",
        MassUnit::Kilogram | MassUnit::Megagram => {
            return Err(WriteError::UnsupportedIrVariant {
                detail: format!(
                    "plain-SI mass unit {unit:?} reached the CONVERSION_BASED_UNIT emit path"
                ),
            });
        }
    };
    let dim_exp = if dim_exp_step != 0 {
        dim_exp_step
    } else {
        emit_mass_dim_exponents(buf)
    };
    buf.entities.push(WriterEntity {
        id: target_id,
        body: WriterBody::Complex {
            parts: vec![
                (
                    "CONVERSION_BASED_UNIT".into(),
                    vec![
                        Attribute::String(name.into()),
                        Attribute::EntityRef(measure_step),
                    ],
                ),
                ("MASS_UNIT".into(), vec![]),
                ("NAMED_UNIT".into(), vec![Attribute::EntityRef(dim_exp)]),
            ],
        },
    });
    Ok(target_id)
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
