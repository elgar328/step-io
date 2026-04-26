//! Unit context converters (Pass 0).
//!
//! Handles the complex entities that describe length / plane-angle /
//! solid-angle units and the `GLOBAL_UNIT_ASSIGNED_CONTEXT` that ties them
//! together. Supports both `SI_UNIT`-based units (mm, cm, m, radian,
//! steradian) and `CONVERSION_BASED_UNIT`-based units (inch, foot, degree,
//! plus CBU-wrapped metric variants such as `'MILLIMETRE'` that appear in
//! some AP242 outputs).

use super::{ReaderContext, has_all_parts, require_part_attrs};
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_enum, read_string};
use crate::ir::error::ConvertError;
use crate::ir::model::{AngleUnit, LengthUnit, SolidAngleUnit, UnitContext};
use crate::parser::entity::{Attribute, RawEntityPart};

impl ReaderContext {
    // ------------------------------------------------------------------
    // Pass 0-1: `SI_UNIT`-bearing complex entities
    // ------------------------------------------------------------------

    pub(super) fn convert_unit_leaf(
        &mut self,
        entity_id: u64,
        parts: &[RawEntityPart],
    ) -> Result<(), ConvertError> {
        let is_length = has_part(parts, "LENGTH_UNIT");
        let is_plane_angle = has_part(parts, "PLANE_ANGLE_UNIT");
        let is_solid_angle = has_part(parts, "SOLID_ANGLE_UNIT");
        if !(is_length || is_plane_angle || is_solid_angle) {
            return Ok(());
        }

        // CONVERSION_BASED_UNIT (inch, foot, degree, or CBU-wrapped metric)
        // takes precedence over SI_UNIT: some AP242 files wrap SI units in a
        // CONVERSION_BASED_UNIT, and the CBU name is the authoritative identity.
        if has_part(parts, "CONVERSION_BASED_UNIT") {
            return self.convert_conversion_based_unit(entity_id, parts, is_length, is_plane_angle);
        }

        if !has_part(parts, "SI_UNIT") {
            return Ok(());
        }

        let si_attrs = require_part_attrs(parts, "SI_UNIT", entity_id)?;
        check_count(si_attrs, 2, entity_id, "SI_UNIT")?;
        let prefix = read_optional_enum(si_attrs, 0, entity_id, "prefix")?;
        let name = read_enum(si_attrs, 1, entity_id, "name")?;

        if is_length {
            if let Some(unit) = match_length_unit(prefix, name) {
                self.length_unit_map.insert(entity_id, unit);
            } else {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "unsupported SI length unit (prefix={prefix:?}, name={name:?})"
                    ),
                });
            }
        } else if is_plane_angle {
            if let Some(unit) = match_angle_unit(prefix, name) {
                self.angle_unit_map.insert(entity_id, unit);
            } else {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("unsupported SI angle unit (prefix={prefix:?}, name={name:?})"),
                });
            }
        } else if is_solid_angle {
            if let Some(unit) = match_solid_angle_unit(prefix, name) {
                self.solid_angle_unit_map.insert(entity_id, unit);
            } else {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "unsupported SI solid-angle unit (prefix={prefix:?}, name={name:?})"
                    ),
                });
            }
        }
        Ok(())
    }

    fn convert_conversion_based_unit(
        &mut self,
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
                self.length_unit_map.insert(entity_id, unit);
                if form == ConversionForm::SiSelf {
                    self.length_cbu_wrapped = true;
                }
            } else {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("unsupported CONVERSION_BASED_UNIT length name: {name:?}"),
                });
            }
        } else if is_plane_angle {
            if let Some((unit, form)) = match_angle_conversion(&upper) {
                self.angle_unit_map.insert(entity_id, unit);
                if form == ConversionForm::SiSelf {
                    self.plane_angle_cbu_wrapped = true;
                }
            } else {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("unsupported CONVERSION_BASED_UNIT angle name: {name:?}"),
                });
            }
        }
        // SOLID_ANGLE_UNIT + CONVERSION_BASED_UNIT is theoretically possible
        // but not observed in practice; ignore silently.
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 0-1b: `UNCERTAINTY_MEASURE_WITH_UNIT`
    // ------------------------------------------------------------------

    /// Record the numeric value of every `UNCERTAINTY_MEASURE_WITH_UNIT`
    /// whose unit component resolved to a length unit. Angle / tolerance
    /// uncertainties are ignored for now (not observed in practice).
    pub(super) fn convert_uncertainty_measure_with_unit(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "UNCERTAINTY_MEASURE_WITH_UNIT")?;
        let value = match attrs.first() {
            Some(Attribute::Typed { value, .. }) => match value.as_ref() {
                Attribute::Real(v) => *v,
                _ => return Ok(()),
            },
            _ => return Ok(()),
        };
        let unit_ref = read_entity_ref(attrs, 1, entity_id, "unit_component")?;
        // attrs[2] = name (보통 'distance_accuracy_value'), attrs[3] = description — 무시.
        if self.length_unit_map.contains_key(&unit_ref) {
            self.length_uncertainty_map.insert(entity_id, value);
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 0-2: `GLOBAL_UNIT_ASSIGNED_CONTEXT`
    // ------------------------------------------------------------------

    pub(super) fn convert_global_unit_assigned_context(
        &mut self,
        entity_id: u64,
        parts: &[RawEntityPart],
    ) -> Result<(), ConvertError> {
        let guac_attrs = require_part_attrs(parts, "GLOBAL_UNIT_ASSIGNED_CONTEXT", entity_id)?;
        check_count(guac_attrs, 1, entity_id, "GLOBAL_UNIT_ASSIGNED_CONTEXT")?;
        let unit_refs = read_entity_ref_list(guac_attrs, 0, entity_id, "units")?;

        let mut length = None;
        let mut plane_angle = None;
        let mut solid_angle = None;
        for r in &unit_refs {
            if let Some(&u) = self.length_unit_map.get(r) {
                length = Some(u);
            } else if let Some(&u) = self.angle_unit_map.get(r) {
                plane_angle = Some(u);
            } else if let Some(&u) = self.solid_angle_unit_map.get(r) {
                solid_angle = Some(u);
            }
        }

        match (length, plane_angle, solid_angle) {
            (Some(length), Some(plane_angle), Some(solid_angle)) => {
                let length_uncertainty = self.extract_length_uncertainty(parts);
                let ctx_id = self.units.push(UnitContext {
                    length,
                    plane_angle,
                    solid_angle,
                    length_uncertainty,
                    length_cbu_wrapped: self.length_cbu_wrapped,
                    plane_angle_cbu_wrapped: self.plane_angle_cbu_wrapped,
                });
                self.context_id_map.insert(entity_id, ctx_id);
            }
            _ => {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "incomplete unit context: length={}, plane_angle={}, solid_angle={}",
                        length.is_some(),
                        plane_angle.is_some(),
                        solid_angle.is_some(),
                    ),
                });
            }
        }
        Ok(())
    }

    /// Pick the first length-flavour uncertainty value referenced by the
    /// `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT` part of the same complex entity,
    /// if any. Returns `None` when the part is absent or holds no length
    /// uncertainty. Invoked from `convert_global_unit_assigned_context`.
    fn extract_length_uncertainty(&self, parts: &[RawEntityPart]) -> Option<f64> {
        let guac = parts
            .iter()
            .find(|p| p.name == "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT")?;
        let refs = read_entity_ref_list(&guac.attributes, 0, 0, "uncertainty").ok()?;
        refs.iter()
            .find_map(|r| self.length_uncertainty_map.get(r).copied())
    }
}

fn has_part(parts: &[RawEntityPart], name: &str) -> bool {
    has_all_parts(parts, &[name])
}

/// Read an enum attribute, treating `$` (Unset) as `None`.
fn read_optional_enum<'a>(
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

fn match_length_unit(prefix: Option<&str>, name: &str) -> Option<LengthUnit> {
    match (prefix, name) {
        (None, "METRE") => Some(LengthUnit::Metre),
        (Some("MILLI"), "METRE") => Some(LengthUnit::Millimetre),
        (Some("CENTI"), "METRE") => Some(LengthUnit::Centimetre),
        _ => None,
    }
}

fn match_angle_unit(prefix: Option<&str>, name: &str) -> Option<AngleUnit> {
    match (prefix, name) {
        (None, "RADIAN") => Some(AngleUnit::Radian),
        _ => None,
    }
}

fn match_solid_angle_unit(prefix: Option<&str>, name: &str) -> Option<SolidAngleUnit> {
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

fn match_length_conversion(upper_name: &str) -> Option<(LengthUnit, ConversionForm)> {
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

fn match_angle_conversion(upper_name: &str) -> Option<(AngleUnit, ConversionForm)> {
    match upper_name {
        "DEGREE" => Some((AngleUnit::Degree, ConversionForm::NonSi)),
        "RADIAN" => Some((AngleUnit::Radian, ConversionForm::SiSelf)),
        _ => None,
    }
}
