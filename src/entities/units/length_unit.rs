//! `LENGTH_UNIT` handler leaf for length flavour.
//!
//! Mirrors the LENGTH branch of `ReaderContext::convert_unit_leaf` and
//! `WriteBuffer::emit_length_unit` (plus the SI / CBU sub-helpers it
//! calls). Catalog group: `units` (O, part-only — `REQUIRED_PARTS`
//! dispatch keys on the `LENGTH_UNIT` part).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::ComplexEntityHandler;
use crate::entities::units::shared::{
    CbuFactorRefs, CbuFlavor, emit_length_dim_exponents, has_part, read_conversion_based_unit_body,
};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::LengthUnit;
use crate::ir::units::{LengthFlavor, NamedUnit};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct LengthUnitHandler;

#[step_entity_complex(name = "LENGTH_UNIT", cases = [
    ["CONVERSION_BASED_UNIT", "LENGTH_UNIT", "NAMED_UNIT"],
    ["LENGTH_UNIT", "NAMED_UNIT", "SI_UNIT"],
])]
impl ComplexEntityHandler for LengthUnitHandler {
    /// Arena flavour. The units pool emitter dispatches the actual emit
    /// (SI plain via `serialize_length_unit_with_id`, CBU via
    /// `emit_length_cbu_outer`); this fresh-id `write` is the trait contract.
    type WriteInput = LengthFlavor;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // CONVERSION_BASED_UNIT (inch, foot, or CBU-wrapped metric) takes
        // precedence over SI_UNIT: some AP242 files wrap SI units in a
        // CONVERSION_BASED_UNIT, and the CBU name is the authoritative identity.
        // CBU path stays hand-written (graph-walk identification); the preserved
        // conversion-factor MWU is referenced via `cbu_factor_mwu_id`. SI path is
        // 2-layer.
        if has_part(parts, "CONVERSION_BASED_UNIT") {
            let refs =
                read_conversion_based_unit_body(ctx, entity_id, parts, CbuFlavor::Length, graph)?;
            let dim_exp = super::shared::read_named_unit_dim_exp(ctx, parts);
            register_named_length(ctx, entity_id, refs, dim_exp);
            return Ok(());
        }
        let early = bind::bind_length_unit(entity_id, parts)?;
        lower::lower_length_si(ctx, entity_id, &early);
        Ok(())
    }

    /// Emit a **plain SI** length unit complex at the *caller-supplied*
    /// `target_id`. CBU outers (Inch / Foot / SI self-wrap) are emitted via
    /// [`emit_length_cbu_outer`]. Pre-reserving step ids in arena order lets
    /// the writer preserve the input file's `NAMED_UNIT` entity-id ordering
    /// across round-trip. `dim_exp_step` carries the explicit
    /// `DIMENSIONAL_EXPONENTS` ref step id (0 = Derived / `*`).
    /// Fresh-id SI serialize (trait contract). The units pool emitter calls
    /// `serialize_length_unit_with_id` directly for the plain SI path and
    /// `emit_length_cbu_outer` for the CBU path, so this is not on the hot path.
    fn write(buf: &mut WriteBuffer, flavor: LengthFlavor) -> Result<u64, WriteError> {
        Ok(serialize::serialize_length_unit(
            buf,
            &lift::lift_length_si(flavor.unit),
        ))
    }
}

/// Emit a `CONVERSION_BASED_UNIT` length outer at `target_id` referencing the
/// **preserved** conversion-factor `MEASURE_WITH_UNIT` at `measure_step`
/// (emitted earlier in the units pool). The base SI is reached through that
/// MWU's `unit_component`, so the outer carries only `(name, measure_step)`
/// plus the explicit `NAMED_UNIT.dimensions`. `dim_exp_step` 0 → synthesize the
/// shared length DE (kernel-built IR).
pub(crate) fn emit_length_cbu_outer(
    buf: &mut WriteBuffer,
    unit: LengthUnit,
    measure_step: u64,
    target_id: u64,
    dim_exp_step: u64,
) -> u64 {
    let name = match unit {
        LengthUnit::Millimetre => "MILLIMETRE",
        LengthUnit::Centimetre => "CENTIMETRE",
        LengthUnit::Metre => "METRE",
        LengthUnit::Inch => "INCH",
        LengthUnit::Foot => "FOOT",
    };
    let dim_exp = if dim_exp_step != 0 {
        dim_exp_step
    } else {
        emit_length_dim_exponents(buf)
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
                ("LENGTH_UNIT".into(), vec![]),
                ("NAMED_UNIT".into(), vec![Attribute::EntityRef(dim_exp)]),
            ],
        },
    });
    target_id
}

/// Record this `LENGTH_UNIT` complex in the `NamedUnit` arena so that
/// MWU / DUE consumers and `GLOBAL_UNIT_ASSIGNED_CONTEXT` can resolve their
/// `unit_component` / `units` refs. CBU outers receive their resolved
/// [`CbuFactorRefs`] (base SI + preserved factor MWU) inline; plain SI never
/// reaches this path (it lowers via `lower_length_si`).
fn register_named_length(
    ctx: &mut ReaderContext,
    entity_id: u64,
    refs: Option<CbuFactorRefs>,
    dim_exp: Option<crate::ir::DimensionalExponentsId>,
) {
    if let Some(&unit) = ctx.length_unit_map.get(&entity_id) {
        let (cbu_base, cbu_factor_mwu_id) =
            refs.map_or((None, None), |r| (r.cbu_base, r.cbu_factor_mwu_id));
        let flavor = LengthFlavor {
            unit,
            cbu_base,
            dim_exp,
            cbu_factor_mwu_id,
        };
        let id = ctx.named_units_arena.push(NamedUnit::Length(flavor));
        ctx.id_cache.insert(entity_id, id);
    }
}
