//! `MASS_UNIT` handler — Pass 0-1 leaf for mass flavour (units-1).
//!
//! Mirrors the `LENGTH` / `PLANE_ANGLE` / `SOLID_ANGLE` leaves: dispatch keys on
//! the `MASS_UNIT` part, the SI branch reads `(prefix, name)` from
//! `SI_UNIT`, the CBU branch reads the conversion name from
//! `CONVERSION_BASED_UNIT`. Recognised forms:
//!
//! - SI `(KILO, GRAM)` → [`MassUnit::Kilogram`]
//! - SI `(None, GRAM)` → [`MassUnit::Gram`]
//! - CBU `'POUND'`     → [`MassUnit::Pound`]
//!
//! Unrecognised SI prefixes or CBU names fall back to
//! [`MassUnit::Kilogram`] (round-trip lossy) so the named-unit arena
//! still captures the entity; the reader emits a warning so the fallback
//! is visible.

use crate::entities::ComplexEntityHandler;
use crate::entities::units::shared::{has_part, read_optional_enum};
use crate::ir::attr::{check_count, read_enum, read_string};
use crate::ir::error::ConvertError;
use crate::ir::units::{MassUnit, NamedUnit};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct MassUnitHandler;

#[step_entity_complex(name = "MASS_UNIT", pass = Pass0Leaf, required = ["MASS_UNIT"])]
impl ComplexEntityHandler for MassUnitHandler {
    type WriteInput = MassUnit;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let unit = if has_part(parts, "CONVERSION_BASED_UNIT") {
            let cbu_attrs = require_part_attrs(parts, "CONVERSION_BASED_UNIT", entity_id)?;
            check_count(cbu_attrs, 2, entity_id, "CONVERSION_BASED_UNIT")?;
            // Suppress duplicate MWU arena entries — see `shared.rs`.
            if let Some(Attribute::EntityRef(mwu_ref)) = cbu_attrs.get(1) {
                ctx.cbu_internal_mwu_refs.insert(*mwu_ref);
            }
            let name = read_string(cbu_attrs, 0, entity_id, "name")?;
            if name.eq_ignore_ascii_case("POUND") {
                MassUnit::Pound
            } else {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("unsupported CONVERSION_BASED_UNIT mass name: {name:?}"),
                });
                MassUnit::Kilogram
            }
        } else if has_part(parts, "SI_UNIT") {
            let si_attrs = require_part_attrs(parts, "SI_UNIT", entity_id)?;
            check_count(si_attrs, 2, entity_id, "SI_UNIT")?;
            let prefix = read_optional_enum(si_attrs, 0, entity_id, "prefix")?;
            let name = read_enum(si_attrs, 1, entity_id, "name")?;
            match (prefix, name) {
                (Some("KILO"), "GRAM") => MassUnit::Kilogram,
                (None, "GRAM") => MassUnit::Gram,
                _ => {
                    ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                        entity_id,
                        detail: format!(
                            "unsupported SI mass unit (prefix={prefix:?}, name={name:?})"
                        ),
                    });
                    MassUnit::Kilogram
                }
            }
        } else {
            // Neither SI nor CBU — uncovered shape. Skip silently to match
            // the legacy unit-leaf handling of malformed complexes.
            return Ok(());
        };
        let id = ctx.named_units_arena.push(NamedUnit::Mass(unit));
        ctx.named_unit_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, unit: MassUnit) -> Result<u64, WriteError> {
        let n = match unit {
            MassUnit::Kilogram => emit_plain_si_mass(buf, Some("KILO")),
            MassUnit::Gram => emit_plain_si_mass(buf, None),
            MassUnit::Pound => emit_pound(buf),
        };
        Ok(n)
    }
}

fn emit_plain_si_mass(buf: &mut WriteBuffer, prefix: Option<&'static str>) -> u64 {
    let prefix_attr = match prefix {
        Some(p) => Attribute::Enum(p.into()),
        None => Attribute::Unset,
    };
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Complex {
            parts: vec![
                ("MASS_UNIT".into(), vec![]),
                ("NAMED_UNIT".into(), vec![Attribute::Derived]),
                (
                    "SI_UNIT".into(),
                    vec![prefix_attr, Attribute::Enum("GRAM".into())],
                ),
            ],
        },
    });
    n
}

/// Emit a `CONVERSION_BASED_UNIT` mass chain for `POUND`.
///
/// AP214 schema requires the CBU's `conversion_factor` to be a
/// `MASS_MEASURE_WITH_UNIT` whose `unit_component` is a base SI kilogram.
/// 1 pound = 0.45359237 kg; the base SI complex carries explicit
/// `DIMENSIONAL_EXPONENTS(0, 0, 1, 0, 0, 0, 0)` (mass exponent = 1) since
/// the CBU outer needs the same DE in its `NAMED_UNIT.dimensions` slot.
fn emit_pound(buf: &mut WriteBuffer) -> u64 {
    let dim_exp = emit_mass_dim_exponents(buf);
    let base_kg = emit_base_si_kilogram(buf);
    let measure = buf.fresh();
    buf.entities.push(WriterEntity {
        id: measure,
        body: WriterBody::Simple {
            name: "MASS_MEASURE_WITH_UNIT".into(),
            attrs: vec![
                Attribute::Typed {
                    type_name: "MASS_MEASURE".into(),
                    value: Box::new(Attribute::Real(0.453_592_37)),
                },
                Attribute::EntityRef(base_kg),
            ],
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
                        Attribute::String("POUND".into()),
                        Attribute::EntityRef(measure),
                    ],
                ),
                ("MASS_UNIT".into(), vec![]),
                ("NAMED_UNIT".into(), vec![Attribute::EntityRef(dim_exp)]),
            ],
        },
    });
    outer
}

fn emit_base_si_kilogram(buf: &mut WriteBuffer) -> u64 {
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Complex {
            parts: vec![
                ("MASS_UNIT".into(), vec![]),
                ("NAMED_UNIT".into(), vec![Attribute::Derived]),
                (
                    "SI_UNIT".into(),
                    vec![
                        Attribute::Enum("KILO".into()),
                        Attribute::Enum("GRAM".into()),
                    ],
                ),
            ],
        },
    });
    n
}

fn emit_mass_dim_exponents(buf: &mut WriteBuffer) -> u64 {
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
    n
}
