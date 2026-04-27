//! Unit context converters (Pass 0).
//!
//! Handles the complex entities that describe length / plane-angle /
//! solid-angle units and the `GLOBAL_UNIT_ASSIGNED_CONTEXT` that ties them
//! together. Supports both `SI_UNIT`-based units (mm, cm, m, radian,
//! steradian) and `CONVERSION_BASED_UNIT`-based units (inch, foot, degree,
//! plus CBU-wrapped metric variants such as `'MILLIMETRE'` that appear in
//! some AP242 outputs).

use super::{ReaderContext, require_part_attrs};
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::model::UnitContext;
use crate::parser::entity::{Attribute, RawEntityPart};

impl ReaderContext {
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
                    dim_exp_explicit: self.dim_exp_explicit,
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
