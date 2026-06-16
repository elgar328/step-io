//! Units-domain `lower` fns (derived-unit cluster). See the
//! [module docs](super) for the lowering contract.

use crate::early::model::{
    EarlyDerivedUnit, EarlyDerivedUnitElement, EarlyDimensionalExponents,
    EarlyLengthMeasureWithUnit, EarlyMassMeasureWithUnit, EarlyMeasureValue, EarlyMeasureWithUnit,
    EarlyNamedUnit, EarlyPlaneAngleMeasureWithUnit, EarlyRatioMeasureWithUnit,
    EarlyUncertaintyMeasureWithUnit,
};
use crate::ir::error::ConvertError;
use crate::ir::representation_item::MeasureValue;
use crate::ir::shape_rep::LengthUncertainty;
use crate::ir::units::{
    DerivedUnit, DerivedUnitElement, DerivedUnitKind, DimensionalExponents, MeasureWithUnit,
    MeasureWithUnitData, NamedUnit, NamedUnitData,
};
use crate::reader::ReaderContext;

/// Lower one `DERIVED_UNIT_ELEMENT` (unresolved unit = silent drop).
pub(crate) fn lower_derived_unit_element(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDerivedUnitElement,
) {
    let Some(unit_id) = ctx.id_cache.get::<crate::ir::id::NamedUnitId>(early.unit) else {
        return;
    };
    let id = ctx.due_arena.push(DerivedUnitElement {
        unit: unit_id,
        exponent: early.exponent,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one plain `DERIVED_UNIT`: resolve the element refs (unresolved
/// members skip; an all-empty result warns and drops — schema WHERE SET[1:?]).
pub(crate) fn lower_derived_unit(ctx: &mut ReaderContext, entity_id: u64, early: EarlyDerivedUnit) {
    let mut elements = Vec::with_capacity(early.elements.len());
    for r in early.elements {
        if let Some(due_id) = ctx.id_cache.get::<crate::ir::id::DerivedUnitElementId>(r) {
            elements.push(due_id);
        }
    }
    if elements.is_empty() {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: "DERIVED_UNIT has no resolvable elements (schema WHERE: SET[1:?])".into(),
        });
        return;
    }
    let id = ctx.derived_unit_arena.push(DerivedUnit {
        elements,
        kind: DerivedUnitKind::Plain,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DIMENSIONAL_EXPONENTS` (pure pass-through).
pub(crate) fn lower_dimensional_exponents(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyDimensionalExponents,
) {
    let id = ctx.dimensional_exponents.push(DimensionalExponents {
        length_exponent: early.length_exponent,
        mass_exponent: early.mass_exponent,
        time_exponent: early.time_exponent,
        electric_current_exponent: early.electric_current_exponent,
        thermodynamic_temperature_exponent: early.thermodynamic_temperature_exponent,
        amount_of_substance_exponent: early.amount_of_substance_exponent,
        luminous_intensity_exponent: early.luminous_intensity_exponent,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower bare `MEASURE_WITH_UNIT` (carrier `Itself`). `value_component` reuses
/// the synth `measure_value` bridge (real measures only — `EarlyMeasureValue`
/// has no integer member, so `measure_value_to_l2` yields `Real`/`Text`;
/// `Text`/descriptive has no `f64` → drop, matching the legacy non-Real drop).
/// `unit_component` narrows to `NamedUnit` (a `DerivedUnit` ref drops, as before).
/// The CBU-internal guard is upstream in the handler.
pub(crate) fn lower_measure_with_unit(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyMeasureWithUnit,
) {
    let (measure_type, value) =
        match super::shape_rep::measure_value_to_l2(early.value_component.clone()) {
            MeasureValue::Real { type_name, value } => (type_name, value),
            MeasureValue::Text { .. } => return,
            MeasureValue::Integer { .. } => {
                unreachable!("EarlyMeasureValue numeric members are all real")
            }
        };
    let Some(unit) = ctx
        .id_cache
        .get::<crate::ir::id::NamedUnitId>(early.unit_component)
    else {
        return;
    };
    let id = ctx
        .mwu_arena
        .push(MeasureWithUnit::Itself(MeasureWithUnitData {
            measure_type,
            value,
            unit,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Extract the `f64` from a typed-MWU `value_component`. The typed subtypes
/// carry the measure kind in the entity name, so the inner `measure_value`
/// member name is irrelevant — only the real value matters (descriptive/text →
/// `None` drop, matching the legacy `read_mwu_attrs` non-Real drop).
fn mwu_value(vc: &EarlyMeasureValue) -> Option<f64> {
    match super::shape_rep::measure_value_to_l2(vc.clone()) {
        MeasureValue::Real { value, .. } => Some(value),
        MeasureValue::Text { .. } => None,
        MeasureValue::Integer { .. } => {
            unreachable!("EarlyMeasureValue numeric members are all real")
        }
    }
}

/// Lower `LENGTH_MEASURE_WITH_UNIT` (typed subtype → `MeasureWithUnit::Length`).
pub(crate) fn lower_length_measure_with_unit(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyLengthMeasureWithUnit,
) {
    let Some(value) = mwu_value(&early.value_component) else {
        return;
    };
    let Some(unit) = ctx
        .id_cache
        .get::<crate::ir::id::NamedUnitId>(early.unit_component)
    else {
        return;
    };
    let id = ctx.mwu_arena.push(MeasureWithUnit::Length { value, unit });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `MASS_MEASURE_WITH_UNIT` (typed subtype → `MeasureWithUnit::Mass`).
pub(crate) fn lower_mass_measure_with_unit(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyMassMeasureWithUnit,
) {
    let Some(value) = mwu_value(&early.value_component) else {
        return;
    };
    let Some(unit) = ctx
        .id_cache
        .get::<crate::ir::id::NamedUnitId>(early.unit_component)
    else {
        return;
    };
    let id = ctx.mwu_arena.push(MeasureWithUnit::Mass { value, unit });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `PLANE_ANGLE_MEASURE_WITH_UNIT` (→ `MeasureWithUnit::PlaneAngle`).
pub(crate) fn lower_plane_angle_measure_with_unit(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyPlaneAngleMeasureWithUnit,
) {
    let Some(value) = mwu_value(&early.value_component) else {
        return;
    };
    let Some(unit) = ctx
        .id_cache
        .get::<crate::ir::id::NamedUnitId>(early.unit_component)
    else {
        return;
    };
    let id = ctx
        .mwu_arena
        .push(MeasureWithUnit::PlaneAngle { value, unit });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `RATIO_MEASURE_WITH_UNIT` (typed subtype → `MeasureWithUnit::Ratio`).
pub(crate) fn lower_ratio_measure_with_unit(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyRatioMeasureWithUnit,
) {
    let Some(value) = mwu_value(&early.value_component) else {
        return;
    };
    let Some(unit) = ctx
        .id_cache
        .get::<crate::ir::id::NamedUnitId>(early.unit_component)
    else {
        return;
    };
    let id = ctx.mwu_arena.push(MeasureWithUnit::Ratio { value, unit });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `UNCERTAINTY_MEASURE_WITH_UNIT` into the context-specific uncertainty
/// side-map (entity-id keyed, no `id_cache`; GUAC read consumes it). The
/// `value_component`'s measure kind is irrelevant — the unit ref's category
/// (length / plane-angle / solid-angle) routes it; an unrecognized unit drops
/// the value (legacy no-op).
pub(crate) fn lower_uncertainty_measure_with_unit(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyUncertaintyMeasureWithUnit,
) {
    let Some(value) = mwu_value(&early.value_component) else {
        return;
    };
    let unc = LengthUncertainty {
        value,
        name: early.name.clone(),
        // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
        description: early.description.clone().unwrap_or_default(),
    };
    if ctx.length_unit_map.contains_key(&early.unit_component) {
        ctx.length_uncertainty_map.insert(entity_id, unc);
    } else if ctx.angle_unit_map.contains_key(&early.unit_component) {
        ctx.plane_angle_uncertainty_map.insert(entity_id, unc);
    } else if ctx.solid_angle_unit_map.contains_key(&early.unit_component) {
        ctx.solid_angle_uncertainty_map.insert(entity_id, unc);
    }
}

/// Lower bare `NAMED_UNIT(#dimensions)` (`NamedUnit::Itself` — a
/// dimensionless/count unit). `dimensions` resolves through the typed
/// `DimensionalExponentsId` cache (unresolved → `None`, matching the legacy
/// handler); registers the `NamedUnitId` key.
pub(crate) fn lower_named_unit(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyNamedUnit) {
    let dimensions = ctx
        .id_cache
        .get::<crate::ir::id::DimensionalExponentsId>(early.dimensions);
    let id = ctx
        .named_units_arena
        .push(NamedUnit::Itself(NamedUnitData { dimensions }));
    ctx.id_cache.insert(entity_id, id);
}
