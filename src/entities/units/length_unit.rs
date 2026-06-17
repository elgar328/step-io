//! `LENGTH_UNIT` handler leaf for length flavour.
//!
//! Mirrors the LENGTH branch of `ReaderContext::convert_unit_leaf` and
//! `WriteBuffer::emit_length_unit` (plus the SI / CBU sub-helpers it
//! calls). Catalog group: `units` (O, part-only — `REQUIRED_PARTS`
//! dispatch keys on the `LENGTH_UNIT` part).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::ComplexEntityHandler;
use crate::entities::units::shared::{
    CbuFlavor, emit_length_dim_exponents, has_part, read_conversion_based_unit_body,
};
use crate::ir::error::ConvertError;
use crate::ir::id::NamedUnitId;
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
        // CONVERSION_BASED_UNIT (inch, foot, degree, or CBU-wrapped metric)
        // takes precedence over SI_UNIT: some AP242 files wrap SI units in a
        // CONVERSION_BASED_UNIT, and the CBU name is the authoritative identity.
        // CBU path stays hand-written (graph-walk + backfill); SI path is 2-layer.
        if has_part(parts, "CONVERSION_BASED_UNIT") {
            read_conversion_based_unit_body(ctx, entity_id, parts, CbuFlavor::Length, graph)?;
            let dim_exp = super::shared::read_named_unit_dim_exp(ctx, parts);
            register_named_length(ctx, entity_id, None, dim_exp);
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

/// Emit a `CONVERSION_BASED_UNIT` length outer at `target_id` referencing
/// `base_step`. Sub-entities (DE, MWU) use `buf.fresh()` and get ids after
/// the pre-reserved `NamedUnit` id block.
pub(crate) fn emit_length_cbu_outer(
    buf: &mut WriteBuffer,
    unit: LengthUnit,
    base_step: u64,
    target_id: u64,
    dim_exp_step: u64,
    cbu_factor_bare: bool,
) -> u64 {
    let (name, factor) = match unit {
        LengthUnit::Millimetre => ("MILLIMETRE", 1.0),
        LengthUnit::Centimetre => ("CENTIMETRE", 1.0),
        LengthUnit::Metre => ("METRE", 1.0),
        LengthUnit::Inch => ("INCH", 25.4),
        LengthUnit::Foot => ("FOOT", 304.8),
    };
    emit_conversion_based_length(
        buf,
        name,
        factor,
        base_step,
        target_id,
        dim_exp_step,
        cbu_factor_bare,
    )
}

/// Record this `LENGTH_UNIT` complex in the `NamedUnit` arena so that
/// MWU / DUE consumers and `GLOBAL_UNIT_ASSIGNED_CONTEXT` can resolve their
/// `unit_component` / `units` refs through `named_unit_id_map`. CBU outers
/// pass `cbu_base = None` here; the `backfill_cbu_base` post-pass patches in the
/// actual base `NamedUnitId` once both ends of the chain are registered.
fn register_named_length(
    ctx: &mut ReaderContext,
    entity_id: u64,
    cbu_base: Option<NamedUnitId>,
    dim_exp: Option<crate::ir::DimensionalExponentsId>,
) {
    if let Some(&unit) = ctx.length_unit_map.get(&entity_id) {
        let flavor = LengthFlavor {
            unit,
            cbu_base,
            dim_exp,
            cbu_factor_bare: ctx.length_cbu_factor_bare.contains(&entity_id),
        };
        let id = ctx.named_units_arena.push(NamedUnit::Length(flavor));
        ctx.id_cache.insert(entity_id, id);
    }
}

/// Emit a `CONVERSION_BASED_UNIT` length chain wrapping the **already-emitted**
/// base SI at `base_step`. Used for both genuine non-SI units (Inch / Foot
/// — factor 25.4 / 304.8) and SI self-wraps (METRE / MILLIMETRE / CENTIMETRE
/// — factor 1.0). Wraps `LENGTH_MEASURE_WITH_UNIT(factor, base_step)` plus
/// the shared `DIMENSIONAL_EXPONENTS(1, ...)`.
fn emit_conversion_based_length(
    buf: &mut WriteBuffer,
    name: &str,
    factor: f64,
    base_step: u64,
    target_id: u64,
    dim_exp_step: u64,
    cbu_factor_bare: bool,
) -> u64 {
    // CBU outer always carries explicit DE per spec. Reference the flavour's
    // own DE (from the IR arena) when available so the CBU and its base SI
    // share a single DIMENSIONAL_EXPONENTS — re-emitting a fresh one each pass
    // makes the round-trip non-idempotent (the fresh DE re-reads into the arena
    // and the next write adds another). Fall back to a synthesized DE only for
    // kernel-built IR that carries no `dim_exp`.
    let dim_exp = if dim_exp_step != 0 {
        dim_exp_step
    } else {
        emit_length_dim_exponents(buf)
    };
    let measure = buf.fresh();
    // Reproduce the input entity form: a bare MEASURE_WITH_UNIT supertype with a
    // typed LENGTH_MEASURE value (NIST ctc_05 inch), else the canonical
    // LENGTH_MEASURE_WITH_UNIT subtype with a plain real.
    let (measure_name, value_attr) = if cbu_factor_bare {
        (
            "MEASURE_WITH_UNIT",
            Attribute::Typed {
                type_name: "LENGTH_MEASURE".into(),
                value: Box::new(Attribute::Real(factor)),
            },
        )
    } else {
        ("LENGTH_MEASURE_WITH_UNIT", Attribute::Real(factor))
    };
    buf.entities.push(WriterEntity {
        id: measure,
        body: WriterBody::Simple {
            name: measure_name.into(),
            attrs: vec![value_attr, Attribute::EntityRef(base_step)],
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
                ("LENGTH_UNIT".into(), vec![]),
                ("NAMED_UNIT".into(), vec![Attribute::EntityRef(dim_exp)]),
            ],
        },
    });
    target_id
}
