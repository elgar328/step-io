//! Units-domain `lower` fns (derived-unit cluster). See the
//! [module docs](super) for the lowering contract.

use crate::early::model::{
    EarlyDerivedUnit, EarlyDerivedUnitElement, EarlyDimensionalExponents,
    EarlyLengthMeasureWithUnit, EarlyLengthUnit, EarlyLengthUnitCbu, EarlyLengthUnitSi,
    EarlyMassMeasureWithUnit, EarlyMassUnit, EarlyMassUnitCbu, EarlyMassUnitSi, EarlyMeasureValue,
    EarlyMeasureWithUnit, EarlyNamedUnit, EarlyPlaneAngleMeasureWithUnit, EarlyPlaneAngleUnit,
    EarlyPlaneAngleUnitCbu, EarlyPlaneAngleUnitSi, EarlyRatioMeasureWithUnit, EarlyRatioUnit,
    EarlySolidAngleUnit, EarlyUncertaintyMeasureWithUnit,
};
use crate::ir::error::ConvertError;
use crate::ir::representation_item::MeasureValue;
use crate::ir::shape_rep::{AngleUnit, LengthUnit, SolidAngleUnit};
use crate::ir::units::{
    DerivedUnit, DerivedUnitElement, DerivedUnitKind, DimensionalExponents, LengthFlavor,
    MassFlavor, MassUnit, MeasureWithUnit, MeasureWithUnitData, NamedUnit, NamedUnitData,
    PlaneAngleFlavor, RatioFlavor, SiPrefix, SiUnitName, SolidAngleFlavor,
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

/// Lower the simple `RATIO_UNIT(dimensions)` (c3d) → `NamedUnit::Ratio`.
/// `dimensions` resolves through the `DimensionalExponentsId` cache (the
/// handler normalizes a `$`/`*` to a synthetic dimensionless DE before bind, so
/// the ref always resolves here). `complex = false` (the standalone form).
pub(crate) fn lower_ratio_unit(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyRatioUnit) {
    let dim_exp = ctx
        .id_cache
        .get::<crate::ir::id::DimensionalExponentsId>(early.dimensions);
    let id = ctx.named_units_arena.push(NamedUnit::Ratio(RatioFlavor {
        dim_exp,
        complex: false,
    }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `UNCERTAINTY_MEASURE_WITH_UNIT` (a `measure_with_unit` subtype) into the
/// shared `measure_with_units` arena as [`MeasureWithUnit::UncertaintyMeasureWithUnit`]
/// (ir.toml: `arena = "measure_with_unit"`, `id = "MeasureWithUnitId"`). Registers
/// the `id_cache` key so `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT.uncertainty` refs
/// resolve to `MeasureWithUnitId`. The `value_component` measure kind is not
/// stored — it is derived from `unit`'s `NamedUnit` category on emit. An
/// unresolved `unit_component` drops the entry (matching the legacy no-op).
pub(crate) fn lower_uncertainty_measure_with_unit(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyUncertaintyMeasureWithUnit,
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
        .push(MeasureWithUnit::UncertaintyMeasureWithUnit {
            value,
            unit,
            name: early.name.clone(),
            // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
            description: early.description.clone().unwrap_or_default(),
        });
    ctx.id_cache.insert(entity_id, id);
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

/// Lower `SOLID_ANGLE_UNIT` complex. Only the SI `STERADIAN` form is modelled
/// ((None, Steradian) → `SolidAngleUnit::Steradian`); any other SI prefix/name
/// is unsupported and dropped with a warning (matching the legacy handler).
/// Dual output: the `solid_angle_unit_map` (consumed by `UNCERTAINTY_MEASURE_
/// WITH_UNIT` to classify a solid-angle uncertainty) and the `named_units` arena.
/// `dim_exp` is always `None` — `NAMED_UNIT.dimensions` is the canonical Derived
/// (`*`) for a typed unit (handled by `[derived]`).
pub(crate) fn lower_solid_angle_unit(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlySolidAngleUnit,
) {
    let (None, SiUnitName::Steradian) = (early.prefix, early.name) else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "unsupported SI solid-angle unit (prefix={:?}, name={:?})",
                early.prefix, early.name
            ),
        });
        return;
    };
    let unit = SolidAngleUnit::Steradian;
    ctx.solid_angle_unit_map.insert(entity_id, unit);
    let id = ctx
        .named_units_arena
        .push(NamedUnit::SolidAngle(SolidAngleFlavor {
            unit,
            dim_exp: None,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// The base SI `NamedUnitId` a `MEASURE_WITH_UNIT` references (its
/// `unit_component`). Used by CBU lower to recover `cbu_base` from the preserved
/// conversion-factor MWU (units-CBU-②) — no graph-walk.
pub(crate) fn mwu_unit_of(mwu: &MeasureWithUnit) -> crate::ir::id::NamedUnitId {
    match mwu {
        MeasureWithUnit::Itself(d) => d.unit,
        MeasureWithUnit::Length { unit, .. }
        | MeasureWithUnit::Mass { unit, .. }
        | MeasureWithUnit::PlaneAngle { unit, .. }
        | MeasureWithUnit::Ratio { unit, .. }
        | MeasureWithUnit::UncertaintyMeasureWithUnit { unit, .. } => *unit,
    }
}

/// The scalar `value` of a `MEASURE_WITH_UNIT`. Used by mass/plane-angle CBU
/// lower to identify the unit by conversion factor (units-CBU-②) — read from the
/// preserved factor MWU arena entry, replacing the prior graph-walk.
fn mwu_value_of(mwu: &MeasureWithUnit) -> f64 {
    match mwu {
        MeasureWithUnit::Itself(d) => d.value,
        MeasureWithUnit::Length { value, .. }
        | MeasureWithUnit::Mass { value, .. }
        | MeasureWithUnit::PlaneAngle { value, .. }
        | MeasureWithUnit::Ratio { value, .. }
        | MeasureWithUnit::UncertaintyMeasureWithUnit { value, .. } => *value,
    }
}

/// Relative-tolerance compare for CBU conversion factors (source files vary in
/// printed precision; the canonical constants are exact).
fn factor_eq(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * b.abs()
}

/// Identify a mass unit by its conversion factor to the SI base (kilogram):
/// `0.453_592_37` → Pound, `0.001` → Gram, `1000.0` → Ton. (kg is the fixed SI
/// base, so the factor is a base-free identity — unlike length.)
fn match_mass_by_factor(factor: f64) -> Option<MassUnit> {
    if factor_eq(factor, 0.453_592_37) {
        Some(MassUnit::Pound)
    } else if factor_eq(factor, 0.001) {
        Some(MassUnit::Gram)
    } else if factor_eq(factor, 1000.0) {
        Some(MassUnit::Ton)
    } else {
        None
    }
}

/// Identify a plane-angle unit by conversion factor to SI radian: π/180 →
/// Degree, 1.0 → Radian.
fn match_angle_by_factor(factor: f64) -> Option<AngleUnit> {
    if factor_eq(factor, std::f64::consts::PI / 180.0) {
        Some(AngleUnit::Degree)
    } else if factor_eq(factor, 1.0) {
        Some(AngleUnit::Radian)
    } else {
        None
    }
}

/// CBU mass name → unit (fallback when the factor is unrecognized).
fn match_mass_conversion(upper: &str) -> Option<MassUnit> {
    match upper {
        "POUND" => Some(MassUnit::Pound),
        "GRAM" => Some(MassUnit::Gram),
        "TON" => Some(MassUnit::Ton),
        _ => None,
    }
}

/// CBU plane-angle name → unit (fallback).
fn match_angle_conversion(upper: &str) -> Option<AngleUnit> {
    match upper {
        "DEGREE" | "DEGREES" => Some(AngleUnit::Degree),
        "RADIAN" | "RADIANS" => Some(AngleUnit::Radian),
        _ => None,
    }
}

/// Lower `LENGTH_UNIT` (generated L1 enum). SI vs `CONVERSION_BASED_UNIT`.
pub(crate) fn lower_length(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyLengthUnit) {
    match early {
        EarlyLengthUnit::Si(si) => lower_length_si(ctx, entity_id, si),
        EarlyLengthUnit::Cbu(cbu) => lower_length_cbu(ctx, entity_id, cbu),
    }
}

/// Lower the **SI case** of `LENGTH_UNIT`. `(prefix, name)` → `LengthUnit`
/// (Metre/Millimetre/Centimetre); an unsupported SI length is dropped with a
/// warning. `dim_exp` is `None` — `NAMED_UNIT.dimensions` is canonical Derived
/// (`*`) via the case's `derived` hint.
fn lower_length_si(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyLengthUnitSi) {
    let unit = match (early.prefix, early.name) {
        (None, SiUnitName::Metre) => LengthUnit::Metre,
        (Some(SiPrefix::Milli), SiUnitName::Metre) => LengthUnit::Millimetre,
        (Some(SiPrefix::Centi), SiUnitName::Metre) => LengthUnit::Centimetre,
        _ => {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "unsupported SI length unit (prefix={:?}, name={:?})",
                    early.prefix, early.name
                ),
            });
            return;
        }
    };
    ctx.length_unit_map.insert(entity_id, unit);
    let id = ctx.named_units_arena.push(NamedUnit::Length(LengthFlavor {
        unit,
        cbu_base: None,
        dim_exp: None,
        cbu_factor_mwu_id: None,
    }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the **`CONVERSION_BASED_UNIT` case** of `LENGTH_UNIT`. Identifies the
/// unit by the CBU `name` (length's SI base varies — millimetre vs metre — so
/// the factor is not a base-free identity, unlike mass/plane-angle). The
/// preserved conversion-factor MWU (`conversion_factor` ref) supplies `cbu_base`
/// (its `unit_component`) and is referenced via `cbu_factor_mwu_id` — no
/// graph-walk. `NAMED_UNIT.dimensions` is `*` (Derived) → `dim_exp = None`.
fn lower_length_cbu(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyLengthUnitCbu) {
    let unit = match early.name.to_uppercase().as_str() {
        "INCH" => LengthUnit::Inch,
        "FOOT" => LengthUnit::Foot,
        // Some AP242 / ABC exports wrap an SI unit in a CONVERSION_BASED_UNIT
        // (self-wrap); the name is still authoritative.
        "MILLIMETRE" => LengthUnit::Millimetre,
        "CENTIMETRE" => LengthUnit::Centimetre,
        "METRE" => LengthUnit::Metre,
        other => {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("unsupported CONVERSION_BASED_UNIT length name: {other:?}"),
            });
            return;
        }
    };
    let Some(mwu_id) = ctx
        .id_cache
        .get::<crate::ir::id::MeasureWithUnitId>(early.conversion_factor)
    else {
        // Conversion-factor MWU unresolved (non-Real value etc.) — drop, matching
        // the prior read-side None behaviour.
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: "CONVERSION_BASED_UNIT length conversion_factor MWU unresolved".into(),
        });
        return;
    };
    let cbu_base = Some(mwu_unit_of(&ctx.mwu_arena[mwu_id]));
    ctx.length_unit_map.insert(entity_id, unit);
    let id = ctx.named_units_arena.push(NamedUnit::Length(LengthFlavor {
        unit,
        cbu_base,
        dim_exp: None,
        cbu_factor_mwu_id: Some(mwu_id),
    }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `MASS_UNIT` (generated L1 enum). SI vs `CONVERSION_BASED_UNIT`.
pub(crate) fn lower_mass(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyMassUnit) {
    match early {
        EarlyMassUnit::Si(si) => lower_mass_si(ctx, entity_id, si),
        EarlyMassUnit::Cbu(cbu) => lower_mass_cbu(ctx, entity_id, cbu),
    }
}

/// Lower the **SI case** of `MASS_UNIT`. `(prefix, name)` → `MassUnit`
/// (Gram/Kilogram/Megagram); an unsupported SI mass spelling is dropped with a
/// warning rather than faked as Kilogram (a fake match misrepresents the
/// magnitude, e.g. 1 Mg ≠ 1 kg). `dim_exp` is `None` — `NAMED_UNIT.dimensions`
/// is canonical Derived (`*`) via the case's `derived` hint.
fn lower_mass_si(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyMassUnitSi) {
    let unit = match (early.prefix, early.name) {
        (None, SiUnitName::Gram) => MassUnit::Gram,
        (Some(SiPrefix::Kilo), SiUnitName::Gram) => MassUnit::Kilogram,
        (Some(SiPrefix::Mega), SiUnitName::Gram) => MassUnit::Megagram,
        _ => {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "unsupported SI mass unit (prefix={:?}, name={:?})",
                    early.prefix, early.name
                ),
            });
            return;
        }
    };
    ctx.mass_unit_map.insert(entity_id, unit);
    let id = ctx.named_units_arena.push(NamedUnit::Mass(MassFlavor {
        unit,
        cbu_base: None,
        dim_exp: None,
        cbu_factor_mwu_id: None,
    }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the **`CONVERSION_BASED_UNIT` case** of `MASS_UNIT`. The SI base is the
/// fixed kilogram, so the unit is identified **by the preserved factor MWU's
/// value first, name second**; a non-standard name whose factor matches is
/// normalized (`NsCase::CbuMassFactor`). `cbu_base` is the MWU's `unit_component`;
/// `dim_exp = None` (`NAMED_UNIT.dimensions` is `*`).
fn lower_mass_cbu(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyMassUnitCbu) {
    let by_name = match_mass_conversion(&early.name.to_uppercase());
    let Some(mwu_id) = ctx
        .id_cache
        .get::<crate::ir::id::MeasureWithUnitId>(early.conversion_factor)
    else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: "CONVERSION_BASED_UNIT mass conversion_factor MWU unresolved".into(),
        });
        return;
    };
    let (factor, cbu_base) = {
        let mwu = &ctx.mwu_arena[mwu_id];
        (mwu_value_of(mwu), mwu_unit_of(mwu))
    };
    let Some(unit) = match_mass_by_factor(factor).or(by_name) else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "unsupported CONVERSION_BASED_UNIT mass (name={:?}, factor={factor})",
                early.name
            ),
        });
        return;
    };
    if by_name != Some(unit) {
        let normalized_to = match unit {
            MassUnit::Pound => "POUND",
            MassUnit::Gram => "GRAM",
            MassUnit::Ton => "TON",
            MassUnit::Kilogram => "KILOGRAM",
            MassUnit::Megagram => "MEGAGRAM",
        };
        ctx.ns_push(
            crate::reader::NsCase::CbuMassFactor,
            format!("CONVERSION_BASED_UNIT.name ({:?})", early.name),
            1,
            normalized_to.into(),
        );
    }
    ctx.mass_unit_map.insert(entity_id, unit);
    let id = ctx.named_units_arena.push(NamedUnit::Mass(MassFlavor {
        unit,
        cbu_base: Some(cbu_base),
        dim_exp: None,
        cbu_factor_mwu_id: Some(mwu_id),
    }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `PLANE_ANGLE_UNIT` (generated L1 enum). SI vs `CONVERSION_BASED_UNIT`.
pub(crate) fn lower_plane_angle(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyPlaneAngleUnit,
) {
    match early {
        EarlyPlaneAngleUnit::Si(si) => lower_plane_angle_si(ctx, entity_id, si),
        EarlyPlaneAngleUnit::Cbu(cbu) => lower_plane_angle_cbu(ctx, entity_id, cbu),
    }
}

/// Lower the **SI case** of `PLANE_ANGLE_UNIT`. Only the SI `RADIAN` form is
/// modelled ((None, Radian) → `AngleUnit::Radian`); any other SI prefix/name is
/// unsupported and dropped with a warning. `dim_exp = None` — `NAMED_UNIT.
/// dimensions` is canonical Derived (`*`) via the case's `derived` hint.
fn lower_plane_angle_si(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyPlaneAngleUnitSi) {
    let (None, SiUnitName::Radian) = (early.prefix, early.name) else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "unsupported SI angle unit (prefix={:?}, name={:?})",
                early.prefix, early.name
            ),
        });
        return;
    };
    let unit = AngleUnit::Radian;
    ctx.angle_unit_map.insert(entity_id, unit);
    let id = ctx
        .named_units_arena
        .push(NamedUnit::PlaneAngle(PlaneAngleFlavor {
            unit,
            cbu_base: None,
            dim_exp: None,
            cbu_factor_mwu_id: None,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the **`CONVERSION_BASED_UNIT` case** of `PLANE_ANGLE_UNIT`. SI base is
/// the fixed radian → identify by factor first, name second; non-standard name
/// matching a known factor is normalized (`NsCase::CbuAngleFactor`).
fn lower_plane_angle_cbu(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyPlaneAngleUnitCbu) {
    let by_name = match_angle_conversion(&early.name.to_uppercase());
    let Some(mwu_id) = ctx
        .id_cache
        .get::<crate::ir::id::MeasureWithUnitId>(early.conversion_factor)
    else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: "CONVERSION_BASED_UNIT angle conversion_factor MWU unresolved".into(),
        });
        return;
    };
    let (factor, cbu_base) = {
        let mwu = &ctx.mwu_arena[mwu_id];
        (mwu_value_of(mwu), mwu_unit_of(mwu))
    };
    let Some(unit) = match_angle_by_factor(factor).or(by_name) else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "unsupported CONVERSION_BASED_UNIT angle (name={:?}, factor={factor})",
                early.name
            ),
        });
        return;
    };
    if by_name != Some(unit) {
        let normalized_to = match unit {
            AngleUnit::Degree => "DEGREE",
            AngleUnit::Radian => "RADIAN",
        };
        ctx.ns_push(
            crate::reader::NsCase::CbuAngleFactor,
            format!("CONVERSION_BASED_UNIT.name ({:?})", early.name),
            1,
            normalized_to.into(),
        );
    }
    ctx.angle_unit_map.insert(entity_id, unit);
    let id = ctx
        .named_units_arena
        .push(NamedUnit::PlaneAngle(PlaneAngleFlavor {
            unit,
            cbu_base: Some(cbu_base),
            dim_exp: None,
            cbu_factor_mwu_id: Some(mwu_id),
        }));
    ctx.id_cache.insert(entity_id, id);
}
