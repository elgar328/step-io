//! Shape-representation-domain `lift` fns (the representation relationship
//! cluster). See the [module docs](super) for the lifting contract.

use crate::early::model::{
    EarlyAllAroundShapeAspect, EarlyCentreOfSymmetry, EarlyCharacterizedItemWithinRepresentation,
    EarlyCompositeGroupShapeAspect, EarlyCompositeShapeAspect,
    EarlyConstructiveGeometryRepresentation, EarlyConstructiveGeometryRepresentationRelationship,
    EarlyDatumSystem, EarlyDatumTarget, EarlyDescriptiveRepresentationItem,
    EarlyFeatureForDatumTargetRelationship, EarlyMeasureValue,
    EarlyMechanicalDesignAndDraughtingRelationship, EarlyModelGeometricView,
    EarlyParametricRepresentationContext, EarlyPlacedDatumTargetFeature,
    EarlyQualifiedRepresentationItem, EarlyRealRepresentationItem, EarlyRepresentationContext,
    EarlyRepresentationRelationship, EarlyShapeAspect, EarlyShapeAspectAssociativity,
    EarlyShapeAspectDerivingRelationship, EarlyShapeAspectRelationship,
    EarlyShapeRepresentationRelationship, EarlyTessellatedShapeRepresentation, EarlyToleranceZone,
    EarlyValueRepresentationItem,
};
use crate::ir::representation_item::{MeasureValue, ValueRepresentationItem};
use crate::ir::shape_rep::{
    ConstructiveGeometryRepr, TessellatedShapeRepresentation, UnitlessContext,
};
use crate::parser::entity::Attribute;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

/// Lift one base `REPRESENTATION_RELATIONSHIP` from its (pre-resolved) arena
/// data. The legacy writer emitted `description` as a String (`''` for empty,
/// never `$`), so the faithful-optional L1 field is always `Some`.
pub(crate) fn lift_representation_relationship(
    name: String,
    description: String,
    rep_1: u64,
    rep_2: u64,
) -> EarlyRepresentationRelationship {
    EarlyRepresentationRelationship {
        name,
        description: Some(description),
        rep_1,
        rep_2,
    }
}

/// Lift one `CONSTRUCTIVE_GEOMETRY_REPRESENTATION_RELATIONSHIP` from its
/// (pre-resolved) arena data — faithful pass-through, `Some` description
/// (the legacy writer always emitted a String, never `$`).
pub(crate) fn lift_constructive_geometry_representation_relationship(
    name: String,
    description: String,
    rep_1: u64,
    rep_2: u64,
) -> EarlyConstructiveGeometryRepresentationRelationship {
    EarlyConstructiveGeometryRepresentationRelationship {
        name,
        description: Some(description),
        rep_1,
        rep_2,
    }
}

/// Lift one `MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP` from its
/// (pre-resolved) arena data — faithful pass-through, `Some` description.
pub(crate) fn lift_mechanical_design_and_draughting_relationship(
    name: String,
    description: String,
    rep_1: u64,
    rep_2: u64,
) -> EarlyMechanicalDesignAndDraughtingRelationship {
    EarlyMechanicalDesignAndDraughtingRelationship {
        name,
        description: Some(description),
        rep_1,
        rep_2,
    }
}

/// Lift one `SHAPE_REPRESENTATION_RELATIONSHIP` write input. The legacy
/// writer hard-codes `name`/`description` to `"SRR"`/`"None"` (the arena's
/// faithful values are not consulted on emit — byte-verified legacy
/// behavior); faithful round-trip of these is a separate, deliberate
/// decision.
pub(crate) fn lift_shape_representation_relationship(
    rep_1: u64,
    rep_2: u64,
) -> EarlyShapeRepresentationRelationship {
    EarlyShapeRepresentationRelationship {
        name: "SRR".into(),
        description: Some("None".into()),
        rep_1,
        rep_2,
    }
}

/// Lift one bare `REPRESENTATION_CONTEXT` from its arena data.
pub(crate) fn lift_representation_context(
    identifier: String,
    context_type: String,
) -> EarlyRepresentationContext {
    EarlyRepresentationContext {
        context_identifier: identifier,
        context_type,
    }
}

/// Lift one `CHARACTERIZED_ITEM_WITHIN_REPRESENTATION` (refs pre-resolved —
/// the item via `emit_representation_item_ref`, which may emit it).
pub(crate) fn lift_characterized_item_within_representation(
    name: String,
    description: Option<String>,
    item: u64,
    rep: u64,
) -> EarlyCharacterizedItemWithinRepresentation {
    EarlyCharacterizedItemWithinRepresentation {
        name,
        description,
        item,
        rep,
    }
}

/// `bool` → the L1 LOGICAL slot (the legacy writer emitted `.T.` / `.F.`).
fn bool_to_logical(b: bool) -> crate::ir::geometry::Logical {
    if b {
        crate::ir::geometry::Logical::True
    } else {
        crate::ir::geometry::Logical::False
    }
}

/// Lift one `REAL_REPRESENTATION_ITEM`.
pub(crate) fn lift_real_representation_item(
    name: String,
    the_value: f64,
) -> EarlyRealRepresentationItem {
    EarlyRealRepresentationItem { name, the_value }
}

/// Lift one `DATUM_TARGET` (`of_shape` pre-resolved to the PDS step id;
/// legacy always emitted `description` as a String, never `$`).
pub(crate) fn lift_datum_target(
    dt: crate::ir::shape_rep::DatumTarget,
    of_shape: u64,
) -> EarlyDatumTarget {
    EarlyDatumTarget {
        name: dt.name,
        description: Some(dt.description),
        of_shape,
        product_definitional: bool_to_logical(dt.product_definitional),
        target_id: dt.target_id,
    }
}

/// Lift one `TOLERANCE_ZONE` (refs pre-resolved).
pub(crate) fn lift_tolerance_zone(
    name: String,
    description: String,
    of_shape: u64,
    product_definitional: bool,
    defining_tolerance: Vec<u64>,
    form: u64,
) -> EarlyToleranceZone {
    EarlyToleranceZone {
        name,
        description: Some(description),
        of_shape,
        product_definitional: bool_to_logical(product_definitional),
        defining_tolerance,
        form,
    }
}

/// Lift one plain `SHAPE_ASPECT` from pre-resolved write fields (`of_shape` =
/// the PDS step id the emit loop resolved from `ProductId`). Mirrors the subtype
/// lifts: `description` always `Some` (legacy never emitted `$`).
pub(crate) fn lift_shape_aspect(
    name: String,
    description: String,
    of_shape: u64,
    product_definitional: bool,
) -> EarlyShapeAspect {
    EarlyShapeAspect {
        name,
        description: Some(description),
        of_shape,
        product_definitional: bool_to_logical(product_definitional),
    }
}

/// Lift the plain `SHAPE_ASPECT_RELATIONSHIP` from pre-resolved write fields
/// (both endpoints = the step ids `emit_shape_aspect_ref` produced).
/// `description` always `Some` (legacy never emitted `$`).
pub(crate) fn lift_shape_aspect_relationship(
    name: String,
    description: String,
    relating_step: u64,
    related_step: u64,
) -> EarlyShapeAspectRelationship {
    EarlyShapeAspectRelationship {
        name,
        description: Some(description),
        relating_shape_aspect: relating_step,
        related_shape_aspect: related_step,
    }
}

/// Lift the `SHAPE_ASPECT_ASSOCIATIVITY` kind.
pub(crate) fn lift_shape_aspect_associativity(
    name: String,
    description: String,
    relating_step: u64,
    related_step: u64,
) -> EarlyShapeAspectAssociativity {
    EarlyShapeAspectAssociativity {
        name,
        description: Some(description),
        relating_shape_aspect: relating_step,
        related_shape_aspect: related_step,
    }
}

/// Lift the `SHAPE_ASPECT_DERIVING_RELATIONSHIP` kind.
pub(crate) fn lift_shape_aspect_deriving_relationship(
    name: String,
    description: String,
    relating_step: u64,
    related_step: u64,
) -> EarlyShapeAspectDerivingRelationship {
    EarlyShapeAspectDerivingRelationship {
        name,
        description: Some(description),
        relating_shape_aspect: relating_step,
        related_shape_aspect: related_step,
    }
}

/// Lift the `FEATURE_FOR_DATUM_TARGET_RELATIONSHIP` kind.
pub(crate) fn lift_feature_for_datum_target_relationship(
    name: String,
    description: String,
    relating_step: u64,
    related_step: u64,
) -> EarlyFeatureForDatumTargetRelationship {
    EarlyFeatureForDatumTargetRelationship {
        name,
        description: Some(description),
        relating_shape_aspect: relating_step,
        related_shape_aspect: related_step,
    }
}

/// Lift one `COMPOSITE_GROUP_SHAPE_ASPECT` from pre-resolved write fields.
pub(crate) fn lift_composite_group_shape_aspect(
    name: String,
    description: String,
    of_shape: u64,
    product_definitional: bool,
) -> EarlyCompositeGroupShapeAspect {
    EarlyCompositeGroupShapeAspect {
        name,
        description: Some(description),
        of_shape,
        product_definitional: bool_to_logical(product_definitional),
    }
}

/// Lift one `COMPOSITE_SHAPE_ASPECT`.
pub(crate) fn lift_composite_shape_aspect(
    name: String,
    description: String,
    of_shape: u64,
    product_definitional: bool,
) -> EarlyCompositeShapeAspect {
    EarlyCompositeShapeAspect {
        name,
        description: Some(description),
        of_shape,
        product_definitional: bool_to_logical(product_definitional),
    }
}

/// Lift one `CENTRE_OF_SYMMETRY`.
pub(crate) fn lift_centre_of_symmetry(
    name: String,
    description: String,
    of_shape: u64,
    product_definitional: bool,
) -> EarlyCentreOfSymmetry {
    EarlyCentreOfSymmetry {
        name,
        description: Some(description),
        of_shape,
        product_definitional: bool_to_logical(product_definitional),
    }
}

/// Lift one `ALL_AROUND_SHAPE_ASPECT`.
pub(crate) fn lift_all_around_shape_aspect(
    name: String,
    description: String,
    of_shape: u64,
    product_definitional: bool,
) -> EarlyAllAroundShapeAspect {
    EarlyAllAroundShapeAspect {
        name,
        description: Some(description),
        of_shape,
        product_definitional: bool_to_logical(product_definitional),
    }
}

/// Lift one `PLACED_DATUM_TARGET_FEATURE` (`of_shape` pre-resolved).
pub(crate) fn lift_placed_datum_target_feature(
    p: crate::ir::shape_rep::PlacedDatumTargetFeature,
    of_shape: u64,
) -> EarlyPlacedDatumTargetFeature {
    EarlyPlacedDatumTargetFeature {
        name: p.name,
        description: Some(p.description),
        of_shape,
        product_definitional: bool_to_logical(p.product_definitional),
        target_id: p.target_id,
    }
}

/// Lift one `DATUM_SYSTEM` (refs pre-resolved).
pub(crate) fn lift_datum_system(
    name: String,
    description: String,
    of_shape: u64,
    product_definitional: bool,
    constituents: Vec<u64>,
) -> EarlyDatumSystem {
    EarlyDatumSystem {
        name,
        description: Some(description),
        of_shape,
        product_definitional: bool_to_logical(product_definitional),
        constituents,
    }
}

/// Lift one `DESCRIPTIVE_REPRESENTATION_ITEM`.
pub(crate) fn lift_descriptive_representation_item(
    name: String,
    description: String,
) -> EarlyDescriptiveRepresentationItem {
    EarlyDescriptiveRepresentationItem { name, description }
}

/// Lift one `QUALIFIED_REPRESENTATION_ITEM` (qualifiers pre-resolved via
/// `emit_select`).
pub(crate) fn lift_qualified_representation_item(
    name: String,
    qualifiers: Vec<u64>,
) -> EarlyQualifiedRepresentationItem {
    EarlyQualifiedRepresentationItem { name, qualifiers }
}

/// Lift one `MODEL_GEOMETRIC_VIEW` (refs pre-resolved).
pub(crate) fn lift_model_geometric_view(
    name: String,
    description: Option<String>,
    item: u64,
    rep: u64,
) -> EarlyModelGeometricView {
    EarlyModelGeometricView {
        name,
        description,
        item,
        rep,
    }
}

/// Lift one parametric `(GRC PRC REP_CONTEXT)` context. `coordinate_space_
/// dimension` is `Some(..)` by construction here (the writer only routes the
/// parametric/complex form to this path), so unwrap is total.
pub(crate) fn lift_parametric_representation_context(
    uc: UnitlessContext,
) -> EarlyParametricRepresentationContext {
    EarlyParametricRepresentationContext {
        coordinate_space_dimension: uc
            .coordinate_space_dimension
            .expect("parametric context carries a coordinate_space_dimension"),
        context_identifier: uc.identifier,
        context_type: uc.context_type,
    }
}

/// Bridge L2 generic `MeasureValue` → the synth-generated typed
/// `EarlyMeasureValue` (inverse of `measure_value_to_l2`). `type_name` selects
/// the member; numeric members are real (an `Integer` value casts to f64, as
/// the L1 has no integer member — see Phase 3j). The `unreachable!` only fires
/// on a non-standard `type_name`, which `lower` never produces.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn measure_value_to_early(m: &MeasureValue) -> EarlyMeasureValue {
    use EarlyMeasureValue as E;
    let (type_name, value) = match m {
        MeasureValue::Text { value, .. } => return E::DescriptiveMeasure(value.clone()),
        MeasureValue::Real { type_name, value } => (type_name.as_str(), *value),
        MeasureValue::Integer { type_name, value } => (type_name.as_str(), *value as f64),
    };
    macro_rules! pick {
        ($($t:literal => $V:ident),+ $(,)?) => {
            match type_name {
                $($t => E::$V(value),)+
                other => unreachable!("non-standard measure_value type_name: {other}"),
            }
        };
    }
    pick! {
        "ABSORBED_DOSE_MEASURE" => AbsorbedDoseMeasure,
        "ACCELERATION_MEASURE" => AccelerationMeasure,
        "AMOUNT_OF_SUBSTANCE_MEASURE" => AmountOfSubstanceMeasure,
        "AREA_MEASURE" => AreaMeasure,
        "CAPACITANCE_MEASURE" => CapacitanceMeasure,
        "CELSIUS_TEMPERATURE_MEASURE" => CelsiusTemperatureMeasure,
        "CONDUCTANCE_MEASURE" => ConductanceMeasure,
        "CONTEXT_DEPENDENT_MEASURE" => ContextDependentMeasure,
        "COUNT_MEASURE" => CountMeasure,
        "DOSE_EQUIVALENT_MEASURE" => DoseEquivalentMeasure,
        "ELECTRIC_CHARGE_MEASURE" => ElectricChargeMeasure,
        "ELECTRIC_CURRENT_MEASURE" => ElectricCurrentMeasure,
        "ELECTRIC_POTENTIAL_MEASURE" => ElectricPotentialMeasure,
        "ENERGY_MEASURE" => EnergyMeasure,
        "FORCE_MEASURE" => ForceMeasure,
        "FREQUENCY_MEASURE" => FrequencyMeasure,
        "ILLUMINANCE_MEASURE" => IlluminanceMeasure,
        "INDUCTANCE_MEASURE" => InductanceMeasure,
        "LENGTH_MEASURE" => LengthMeasure,
        "LUMINOUS_FLUX_MEASURE" => LuminousFluxMeasure,
        "LUMINOUS_INTENSITY_MEASURE" => LuminousIntensityMeasure,
        "MAGNETIC_FLUX_DENSITY_MEASURE" => MagneticFluxDensityMeasure,
        "MAGNETIC_FLUX_MEASURE" => MagneticFluxMeasure,
        "MASS_MEASURE" => MassMeasure,
        "NON_NEGATIVE_LENGTH_MEASURE" => NonNegativeLengthMeasure,
        "NUMERIC_MEASURE" => NumericMeasure,
        "PARAMETER_VALUE" => ParameterValue,
        "PLANE_ANGLE_MEASURE" => PlaneAngleMeasure,
        "POSITIVE_LENGTH_MEASURE" => PositiveLengthMeasure,
        "POSITIVE_PLANE_ANGLE_MEASURE" => PositivePlaneAngleMeasure,
        "POSITIVE_RATIO_MEASURE" => PositiveRatioMeasure,
        "POWER_MEASURE" => PowerMeasure,
        "PRESSURE_MEASURE" => PressureMeasure,
        "RADIOACTIVITY_MEASURE" => RadioactivityMeasure,
        "RATIO_MEASURE" => RatioMeasure,
        "RESISTANCE_MEASURE" => ResistanceMeasure,
        "SOLID_ANGLE_MEASURE" => SolidAngleMeasure,
        "THERMODYNAMIC_TEMPERATURE_MEASURE" => ThermodynamicTemperatureMeasure,
        "TIME_MEASURE" => TimeMeasure,
        "VELOCITY_MEASURE" => VelocityMeasure,
        "VOLUME_MEASURE" => VolumeMeasure,
    }
}

/// Lift one `VALUE_REPRESENTATION_ITEM` → its L1 form.
pub(crate) fn lift_value_representation_item(
    vri: &ValueRepresentationItem,
) -> EarlyValueRepresentationItem {
    EarlyValueRepresentationItem {
        name: vri.name.clone(),
        value_component: measure_value_to_early(&vri.value_component),
    }
}

/// Lift one `TESSELLATED_SHAPE_REPRESENTATION`. `items` emit through the shared
/// (infallible) tessellated-item emitter; `context_of_items` reuses the shared
/// `repr_context_attr` mapping, which always yields an `EntityRef` here because
/// `lower` admits only the `Unitful` context.
pub(crate) fn lift_tessellated_shape_representation(
    buf: &WriteBuffer,
    tsr: TessellatedShapeRepresentation,
) -> EarlyTessellatedShapeRepresentation {
    let items = tsr
        .items
        .iter()
        .map(|&r| buf.emit_tessellated_item_ref(r))
        .collect();
    let Attribute::EntityRef(context_of_items) = buf.repr_context_attr(tsr.context) else {
        unreachable!("TSR context_of_items is always a Unitful GUAC → EntityRef")
    };
    EarlyTessellatedShapeRepresentation {
        name: tsr.name,
        items,
        context_of_items,
    }
}

/// Lift one `CONSTRUCTIVE_GEOMETRY_REPRESENTATION`. `items` emit through the
/// shared (fallible) representation-item emitter; `context_of_items` reuses the
/// shared `repr_context_attr` mapping, which always yields an `EntityRef` here
/// because `lower` drops any carrier whose context did not resolve.
pub(crate) fn lift_constructive_geometry_representation(
    buf: &mut WriteBuffer,
    cgr: ConstructiveGeometryRepr,
) -> Result<EarlyConstructiveGeometryRepresentation, WriteError> {
    let mut items = Vec::with_capacity(cgr.items.len());
    for item in cgr.items {
        items.push(buf.emit_representation_item_ref(item)?);
    }
    let Attribute::EntityRef(context_of_items) = buf.repr_context_attr(cgr.context) else {
        unreachable!("CGR context is guaranteed resolved by lower → EntityRef")
    };
    Ok(EarlyConstructiveGeometryRepresentation {
        name: cgr.name,
        items,
        context_of_items,
    })
}
