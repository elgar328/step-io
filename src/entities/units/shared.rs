//! Helpers shared by the four Pass 0-1 unit-leaf handlers
//! (`LengthUnitHandler` / `PlaneAngleUnitHandler` / `SolidAngleUnitHandler` /
//! `MassUnitHandler`).
//!
//! Reader side: SI / `CONVERSION_BASED_UNIT` matchers and the shared
//! `read_conversion_based_unit_body` covering the CBU branch (length /
//! plane-angle / mass — solid-angle CBU forms are unobserved).
//!
//! Writer side: cached `DIMENSIONAL_EXPONENTS` emitters used by every
//! leaf when it produces an explicit `NAMED_UNIT.dimensions`
//! (ABC-tier loyalty) — the cache fields live on `WriteBuffer`, so the
//! helpers take `&mut WriteBuffer`.

use crate::ir::attr::{check_count, read_entity_ref, read_enum, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{AngleUnit, LengthUnit, SolidAngleUnit};
use crate::ir::units::MassUnit;
use crate::parser::entity::{Attribute, RawEntityPart};
use crate::reader::{ReaderContext, find_part_attrs, has_all_parts, require_part_attrs};
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(super) fn has_part(parts: &[RawEntityPart], name: &str) -> bool {
    has_all_parts(parts, &[name])
}

/// Read an enum attribute, treating `$` (Unset) as `None`.
pub(super) fn read_optional_enum<'a>(
    attrs: &'a [Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Option<&'a str>, ConvertError> {
    match attrs.get(index) {
        Some(Attribute::Unset) => Ok(None),
        Some(_) => read_enum(attrs, index, entity_id, field_name).map(Some),
        None => Err(ConvertError::AttributeIndex {
            entity_id,
            field_name,
            index,
            len: attrs.len(),
        }),
    }
}

pub(super) fn match_length_unit(prefix: Option<&str>, name: &str) -> Option<LengthUnit> {
    match (prefix, name) {
        (None, "METRE") => Some(LengthUnit::Metre),
        (Some("MILLI"), "METRE") => Some(LengthUnit::Millimetre),
        (Some("CENTI"), "METRE") => Some(LengthUnit::Centimetre),
        _ => None,
    }
}

pub(super) fn match_angle_unit(prefix: Option<&str>, name: &str) -> Option<AngleUnit> {
    match (prefix, name) {
        (None, "RADIAN") => Some(AngleUnit::Radian),
        _ => None,
    }
}

pub(super) fn match_solid_angle_unit(prefix: Option<&str>, name: &str) -> Option<SolidAngleUnit> {
    match (prefix, name) {
        (None, "STERADIAN") => Some(SolidAngleUnit::Steradian),
        _ => None,
    }
}

pub(super) fn match_length_conversion(upper_name: &str) -> Option<LengthUnit> {
    match upper_name {
        "INCH" => Some(LengthUnit::Inch),
        "FOOT" => Some(LengthUnit::Foot),
        // Some AP242 / ABC exports wrap SI units in a CONVERSION_BASED_UNIT.
        // Self-wrap is represented structurally via `cbu_base = Some(<base_id>)`
        // with `outer.unit == base.unit`; the writer reproduces the wrapper
        // by virtue of `cbu_base` being `Some`.
        "MILLIMETRE" => Some(LengthUnit::Millimetre),
        "CENTIMETRE" => Some(LengthUnit::Centimetre),
        "METRE" => Some(LengthUnit::Metre),
        _ => None,
    }
}

pub(super) fn match_angle_conversion(upper_name: &str) -> Option<AngleUnit> {
    match upper_name {
        "DEGREE" | "DEGREES" => Some(AngleUnit::Degree),
        "RADIAN" | "RADIANS" => Some(AngleUnit::Radian),
        _ => None,
    }
}

pub(super) fn match_mass_conversion(upper_name: &str) -> Option<MassUnit> {
    match upper_name {
        "POUND" => Some(MassUnit::Pound),
        // gram defined as a CONVERSION_BASED_UNIT (0.001 of the SI kg) —
        // a genuine conversion, like INCH for length.
        "GRAM" => Some(MassUnit::Gram),
        _ => None,
    }
}

/// Which flavour's CBU branch is being read. Replaces the prior asymmetric
/// `(is_length, is_plane_angle)` bool pair on
/// [`read_conversion_based_unit_body`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CbuFlavor {
    Length,
    PlaneAngle,
    Mass,
}

/// Reader body shared by `LengthUnitHandler` / `PlaneAngleUnitHandler` /
/// `MassUnitHandler` for the `CONVERSION_BASED_UNIT` branch. The flavour
/// selector picks the right name matcher and per-flavour bookkeeping (e.g.
/// `length_cbu_wrapped`). Unrecognised CBU names are dropped with a warning
/// and **not** recorded in `cbu_outer_to_mwu` — the outer never reaches
/// `NamedUnit` registration so the backfill lookup would be a dead entry.
/// `SOLID_ANGLE_UNIT + CONVERSION_BASED_UNIT` is unobserved and therefore
/// uncovered here.
pub(super) fn read_conversion_based_unit_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    parts: &[RawEntityPart],
    flavor: CbuFlavor,
) -> Result<(), ConvertError> {
    let cbu_attrs = require_part_attrs(parts, "CONVERSION_BASED_UNIT", entity_id)?;
    check_count(cbu_attrs, 2, entity_id, "CONVERSION_BASED_UNIT")?;
    let name = read_string_or_unset(cbu_attrs, 0, entity_id, "name")?;
    let upper = name.to_uppercase();
    let mwu_ref = match cbu_attrs.get(1) {
        Some(Attribute::EntityRef(r)) => Some(*r),
        _ => None,
    };
    // units-1: always suppress the embedded `conversion_factor` MWU duplicate;
    // `cbu_outer_to_mwu` is populated only on recognised names so the
    // post-Pass0Leaf backfill never chases a dead outer.
    if let Some(r) = mwu_ref {
        ctx.cbu_internal_mwu_refs.insert(r);
    }

    let recognised = match flavor {
        CbuFlavor::Length => {
            if let Some(unit) = match_length_conversion(&upper) {
                ctx.length_unit_map.insert(entity_id, unit);
                true
            } else {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("unsupported CONVERSION_BASED_UNIT length name: {name:?}"),
                });
                false
            }
        }
        CbuFlavor::PlaneAngle => {
            if let Some(unit) = match_angle_conversion(&upper) {
                ctx.angle_unit_map.insert(entity_id, unit);
                true
            } else {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("unsupported CONVERSION_BASED_UNIT angle name: {name:?}"),
                });
                false
            }
        }
        CbuFlavor::Mass => {
            if let Some(unit) = match_mass_conversion(&upper) {
                ctx.mass_unit_map.insert(entity_id, unit);
                true
            } else {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("unsupported CONVERSION_BASED_UNIT mass name: {name:?}"),
                });
                false
            }
        }
    };
    if recognised && let Some(r) = mwu_ref {
        ctx.cbu_outer_to_mwu.insert(entity_id, r);
    }
    Ok(())
}

/// Read the shared `(value_component, unit_component)` shape of all four
/// `MEASURE_WITH_UNIT` subtypes (`LENGTH` / `MASS` / `PLANE_ANGLE` /
/// `RATIO`).
///
/// `value_component` is a typed real (`LENGTH_MEASURE(1.0)` /
/// `MASS_MEASURE(0.45)` / etc); `unit_component` is an entity ref to a
/// `NAMED_UNIT` complex. Returns `None` when the attribute shape doesn't
/// match the expected typed-real form (the unsupported variants — e.g.
/// `THERMODYNAMIC_TEMPERATURE_MEASURE` — are silently dropped per the
/// units-1 permanent-loss-boundary note).
pub(crate) fn read_mwu_attrs(
    attrs: &[Attribute],
    entity_id: u64,
    entity_name: &'static str,
) -> Result<Option<(f64, u64)>, ConvertError> {
    check_count(attrs, 2, entity_id, entity_name)?;
    let value = match attrs.first() {
        Some(Attribute::Typed { value, .. }) => match value.as_ref() {
            Attribute::Real(v) => *v,
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };
    let unit_step = read_entity_ref(attrs, 1, entity_id, "unit_component")?;
    Ok(Some((value, unit_step)))
}

/// Emit the length-flavour `DIMENSIONAL_EXPONENTS` (1, 0, 0, 0, 0, 0, 0)
/// once per `WriteBuffer` and cache the step id (units-3c dedup); later
/// callers receive the cached id.
pub(super) fn emit_length_dim_exponents(buf: &mut WriteBuffer) -> u64 {
    if let Some(id) = buf.length_dim_exp_step {
        return id;
    }
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Simple {
            name: "DIMENSIONAL_EXPONENTS".into(),
            attrs: vec![
                Attribute::Real(1.0),
                Attribute::Real(0.0),
                Attribute::Real(0.0),
                Attribute::Real(0.0),
                Attribute::Real(0.0),
                Attribute::Real(0.0),
                Attribute::Real(0.0),
            ],
        },
    });
    buf.length_dim_exp_step = Some(n);
    n
}

/// Emit the dimensionless `DIMENSIONAL_EXPONENTS` (0, 0, 0, 0, 0, 0, 0)
/// once per `WriteBuffer` and cache the step id.
pub(super) fn emit_dimensionless_exponents(buf: &mut WriteBuffer) -> u64 {
    if let Some(id) = buf.dimensionless_dim_exp_step {
        return id;
    }
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Simple {
            name: "DIMENSIONAL_EXPONENTS".into(),
            attrs: vec![Attribute::Real(0.0); 7],
        },
    });
    buf.dimensionless_dim_exp_step = Some(n);
    n
}

/// Read `NAMED_UNIT.dimensions` field of a unit complex (phase
/// dim-exp-arena-c). Returns `Some(id)` when the source emitted an
/// explicit `DIMENSIONAL_EXPONENTS` ref, `None` for the `*` (Derived)
/// form. Unknown refs (cross-cascade) silently degrade to `None`.
pub(crate) fn read_named_unit_dim_exp(
    ctx: &ReaderContext,
    parts: &[crate::parser::entity::RawEntityPart],
) -> Option<crate::ir::DimensionalExponentsId> {
    let attrs = find_part_attrs(parts, "NAMED_UNIT")?;
    match attrs.first()? {
        Attribute::EntityRef(n) => ctx.dim_exp_id_map.get(n).copied(),
        _ => None,
    }
}
