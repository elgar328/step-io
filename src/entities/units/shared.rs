//! Helpers shared by the three Pass 0-1 unit-leaf handlers
//! (`LengthUnitHandler` / `PlaneAngleUnitHandler` / `SolidAngleUnitHandler`).
//!
//! Reader side: SI / `CONVERSION_BASED_UNIT` matchers and the shared
//! `read_conversion_based_unit_body` covering the CBU branch (length /
//! plane-angle only — solid-angle CBU forms are unobserved).
//!
//! Writer side: cached `DIMENSIONAL_EXPONENTS` emitters used by every
//! leaf when it produces an explicit `NAMED_UNIT.dimensions`
//! (ABC-tier loyalty) — the cache fields live on `WriteBuffer`, so the
//! helpers take `&mut WriteBuffer`.

use crate::ir::attr::{check_count, read_enum, read_string};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{AngleUnit, LengthUnit, SolidAngleUnit};
use crate::parser::entity::{Attribute, RawEntityPart};
use crate::reader::{ReaderContext, has_all_parts, require_part_attrs};
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

/// Whether a `CONVERSION_BASED_UNIT` is "natural" (Inch / Foot / Degree —
/// non-SI units that have no plain-SI form) or "self-wrap" (METRE /
/// MILLIMETRE / RADIAN — SI units re-expressed as CBU). The writer
/// preserves self-wraps via the corresponding `UnitContext.*_cbu_wrapped`
/// flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConversionForm {
    SiSelf,
    NonSi,
}

pub(super) fn match_length_conversion(upper_name: &str) -> Option<(LengthUnit, ConversionForm)> {
    match upper_name {
        "INCH" => Some((LengthUnit::Inch, ConversionForm::NonSi)),
        "FOOT" => Some((LengthUnit::Foot, ConversionForm::NonSi)),
        // Some AP242 / ABC exports wrap SI units in a CONVERSION_BASED_UNIT;
        // the writer reproduces the wrapper when `length_cbu_wrapped` is set.
        "MILLIMETRE" => Some((LengthUnit::Millimetre, ConversionForm::SiSelf)),
        "CENTIMETRE" => Some((LengthUnit::Centimetre, ConversionForm::SiSelf)),
        "METRE" => Some((LengthUnit::Metre, ConversionForm::SiSelf)),
        _ => None,
    }
}

pub(super) fn match_angle_conversion(upper_name: &str) -> Option<(AngleUnit, ConversionForm)> {
    match upper_name {
        "DEGREE" | "DEGREES" => Some((AngleUnit::Degree, ConversionForm::NonSi)),
        "RADIAN" | "RADIANS" => Some((AngleUnit::Radian, ConversionForm::SiSelf)),
        _ => None,
    }
}

/// Reader body shared by `LengthUnitHandler` and `PlaneAngleUnitHandler`
/// for the `CONVERSION_BASED_UNIT` branch. Mirrors the legacy
/// `ReaderContext::convert_conversion_based_unit` — only the boolean
/// `is_length` / `is_plane_angle` selector remains (Plan 1's
/// "semantic-preserving migration" — `UnitKind` enum cleanup is out of
/// scope). `SOLID_ANGLE_UNIT + CONVERSION_BASED_UNIT` is unobserved and
/// therefore uncovered here.
pub(super) fn read_conversion_based_unit_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    parts: &[RawEntityPart],
    is_length: bool,
    is_plane_angle: bool,
) -> Result<(), ConvertError> {
    let cbu_attrs = require_part_attrs(parts, "CONVERSION_BASED_UNIT", entity_id)?;
    check_count(cbu_attrs, 2, entity_id, "CONVERSION_BASED_UNIT")?;
    let name = read_string(cbu_attrs, 0, entity_id, "name")?;
    let upper = name.to_uppercase();

    if is_length {
        if let Some((unit, form)) = match_length_conversion(&upper) {
            ctx.length_unit_map.insert(entity_id, unit);
            if form == ConversionForm::SiSelf {
                ctx.length_cbu_wrapped = true;
            }
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("unsupported CONVERSION_BASED_UNIT length name: {name:?}"),
            });
        }
    } else if is_plane_angle {
        if let Some((unit, form)) = match_angle_conversion(&upper) {
            ctx.angle_unit_map.insert(entity_id, unit);
            if form == ConversionForm::SiSelf {
                ctx.plane_angle_cbu_wrapped = true;
            }
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("unsupported CONVERSION_BASED_UNIT angle name: {name:?}"),
            });
        }
    }
    Ok(())
}

/// Emit the length-flavour `DIMENSIONAL_EXPONENTS` (1, 0, 0, 0, 0, 0, 0).
/// Fresh entity per call — writer does no dedup; IR multiplicity rules.
pub(super) fn emit_length_dim_exponents(buf: &mut WriteBuffer) -> u64 {
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
    n
}

/// Emit the dimensionless `DIMENSIONAL_EXPONENTS` (0, 0, 0, 0, 0, 0, 0).
/// Fresh entity per call — writer does no dedup; IR multiplicity rules.
pub(super) fn emit_dimensionless_exponents(buf: &mut WriteBuffer) -> u64 {
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Simple {
            name: "DIMENSIONAL_EXPONENTS".into(),
            attrs: vec![Attribute::Real(0.0); 7],
        },
    });
    n
}
