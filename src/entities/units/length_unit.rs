//! `LENGTH_UNIT` handler — Pass 0-1 leaf for length flavour.
//!
//! Mirrors the LENGTH branch of `ReaderContext::convert_unit_leaf` and
//! `WriteBuffer::emit_length_unit` (plus the SI / CBU sub-helpers it
//! calls). Catalog group: `units` (O, part-only — `REQUIRED_PARTS`
//! dispatch keys on the `LENGTH_UNIT` part).

use crate::entities::ComplexEntityHandler;
use crate::entities::units::shared::{
    emit_length_dim_exponents, has_part, match_length_unit, read_conversion_based_unit_body,
    read_optional_enum,
};
use crate::ir::attr::{check_count, read_enum};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::LengthUnit;
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, find_part_attrs, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct LengthUnitHandler;

#[step_entity_complex(name = "LENGTH_UNIT", pass = Pass0Leaf, required = ["LENGTH_UNIT"])]
impl ComplexEntityHandler for LengthUnitHandler {
    /// `(unit, length_cbu_wrapped, dim_exp_explicit)` — flat tuple
    /// matches the `(DirectionId, f64)` style used elsewhere.
    type WriteInput = (LengthUnit, bool, bool);

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // CONVERSION_BASED_UNIT (inch, foot, degree, or CBU-wrapped metric)
        // takes precedence over SI_UNIT: some AP242 files wrap SI units in a
        // CONVERSION_BASED_UNIT, and the CBU name is the authoritative identity.
        if has_part(parts, "CONVERSION_BASED_UNIT") {
            return read_conversion_based_unit_body(ctx, entity_id, parts, true, false);
        }

        if !has_part(parts, "SI_UNIT") {
            return Ok(());
        }

        // Detect ABC-tier explicit DE pattern. CBU outer complexes always
        // carry an explicit DE per spec, so we limit detection to plain SI
        // complexes — fillet_box has CBU outer explicit + plain SI Derived,
        // ABC has both explicit. Sticky cumulative: any plain SI complex
        // with EntityRef in NAMED_UNIT.dimensions locks the flag.
        if let Some(named_attrs) = find_part_attrs(parts, "NAMED_UNIT")
            && let Some(Attribute::EntityRef(_)) = named_attrs.first()
        {
            ctx.dim_exp_explicit = true;
        }

        let si_attrs = require_part_attrs(parts, "SI_UNIT", entity_id)?;
        check_count(si_attrs, 2, entity_id, "SI_UNIT")?;
        let prefix = read_optional_enum(si_attrs, 0, entity_id, "prefix")?;
        let name = read_enum(si_attrs, 1, entity_id, "name")?;

        if let Some(unit) = match_length_unit(prefix, name) {
            ctx.length_unit_map.insert(entity_id, unit);
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("unsupported SI length unit (prefix={prefix:?}, name={name:?})"),
            });
        }
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (unit, cbu_wrapped, dim_exp_explicit): (LengthUnit, bool, bool),
    ) -> Result<u64, WriteError> {
        let key = (unit, cbu_wrapped);
        if let Some(&n) = buf.length_unit_ids.get(&key) {
            return Ok(n);
        }
        let n = match unit {
            LengthUnit::Millimetre if cbu_wrapped => emit_conversion_based_length(
                buf,
                "MILLIMETRE",
                Some("MILLI"),
                1.0,
                dim_exp_explicit,
            ),
            LengthUnit::Centimetre if cbu_wrapped => emit_conversion_based_length(
                buf,
                "CENTIMETRE",
                Some("CENTI"),
                1.0,
                dim_exp_explicit,
            ),
            LengthUnit::Metre if cbu_wrapped => {
                emit_conversion_based_length(buf, "METRE", None, 1.0, dim_exp_explicit)
            }
            LengthUnit::Millimetre => emit_plain_si_length(buf, Some("MILLI"), dim_exp_explicit),
            LengthUnit::Centimetre => emit_plain_si_length(buf, Some("CENTI"), dim_exp_explicit),
            LengthUnit::Metre => emit_plain_si_length(buf, None, dim_exp_explicit),
            LengthUnit::Inch => {
                emit_conversion_based_length(buf, "INCH", Some("MILLI"), 25.4, dim_exp_explicit)
            }
            LengthUnit::Foot => {
                emit_conversion_based_length(buf, "FOOT", Some("MILLI"), 304.8, dim_exp_explicit)
            }
        };
        buf.length_unit_ids.insert(key, n);
        Ok(n)
    }
}

/// Emit a plain SI-based length unit. Caching is handled by the caller
/// (`LengthUnitHandler::write`) keyed on the IR fields.
/// `dim_exp_explicit=true` puts a shared length DE entity ref in
/// `NAMED_UNIT.dimensions` (ABC pattern); `false` emits `*` Derived.
fn emit_plain_si_length(
    buf: &mut WriteBuffer,
    prefix: Option<&'static str>,
    dim_exp_explicit: bool,
) -> u64 {
    let si_attrs = match prefix {
        Some(p) => vec![Attribute::Enum(p.into()), Attribute::Enum("METRE".into())],
        None => vec![Attribute::Unset, Attribute::Enum("METRE".into())],
    };
    let dim_exp_attr = if dim_exp_explicit {
        Attribute::EntityRef(emit_length_dim_exponents(buf))
    } else {
        Attribute::Derived
    };
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Complex {
            parts: vec![
                ("LENGTH_UNIT".into(), vec![]),
                ("NAMED_UNIT".into(), vec![dim_exp_attr]),
                ("SI_UNIT".into(), si_attrs),
            ],
        },
    });
    n
}

/// Emit an SI length unit complex used internally as the base for a
/// `CONVERSION_BASED_UNIT` length chain. `prefix = None` for plain METRE,
/// `Some("MILLI")` for MILLIMETRE, `Some("CENTI")` for CENTIMETRE, etc.
/// `dim_exp_explicit` mirrors the plain SI path — ABC's CBU base SI
/// (`#329`) carries the same DE ref as the rest of the file.
fn emit_base_si_length(
    buf: &mut WriteBuffer,
    prefix: Option<&'static str>,
    dim_exp_explicit: bool,
) -> u64 {
    let prefix_attr = match prefix {
        Some(p) => Attribute::Enum(p.into()),
        None => Attribute::Unset,
    };
    let dim_exp_attr = if dim_exp_explicit {
        Attribute::EntityRef(emit_length_dim_exponents(buf))
    } else {
        Attribute::Derived
    };
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Complex {
            parts: vec![
                ("LENGTH_UNIT".into(), vec![]),
                ("NAMED_UNIT".into(), vec![dim_exp_attr]),
                (
                    "SI_UNIT".into(),
                    vec![prefix_attr, Attribute::Enum("METRE".into())],
                ),
            ],
        },
    });
    n
}

/// Emit a `CONVERSION_BASED_UNIT` length chain. Used for both genuine
/// non-SI units (Inch / Foot — base MILLI METRE, factor 25.4 / 304.8)
/// and SI self-wraps (METRE / MILLIMETRE / CENTIMETRE — base same as
/// the unit, factor 1.0). Wraps `LENGTH_MEASURE_WITH_UNIT` referencing
/// the SI base and the shared `DIMENSIONAL_EXPONENTS(1, ...)`.
fn emit_conversion_based_length(
    buf: &mut WriteBuffer,
    name: &str,
    base_prefix: Option<&'static str>,
    factor: f64,
    dim_exp_explicit: bool,
) -> u64 {
    let base_si = emit_base_si_length(buf, base_prefix, dim_exp_explicit);
    let dim_exp = emit_length_dim_exponents(buf);
    let measure = buf.fresh();
    buf.entities.push(WriterEntity {
        id: measure,
        body: WriterBody::Simple {
            name: "LENGTH_MEASURE_WITH_UNIT".into(),
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
                ("LENGTH_UNIT".into(), vec![]),
                ("NAMED_UNIT".into(), vec![Attribute::EntityRef(dim_exp)]),
            ],
        },
    });
    outer
}
