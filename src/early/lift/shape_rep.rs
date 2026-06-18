//! Shape-representation-domain `lift` fns (the representation relationship
//! cluster). See the [module docs](super) for the lifting contract.

use crate::early::model::{
    EarlyAdvancedBrepShapeRepresentation, EarlyAllAroundShapeAspect, EarlyCameraImage,
    EarlyCameraImage3dWithScale, EarlyCentreOfSymmetry, EarlyCharacterizedItemWithinRepresentation,
    EarlyCompositeGroupShapeAspect, EarlyCompositeShapeAspect, EarlyCompoundItemDefinition,
    EarlyCompoundRepresentationItem, EarlyConstructiveGeometryRepresentation,
    EarlyConstructiveGeometryRepresentationRelationship, EarlyDatumSystem, EarlyDatumTarget,
    EarlyDefaultModelGeometricView, EarlyDescriptiveRepresentationItem, EarlyDraughtingModel,
    EarlyFeatureForDatumTargetRelationship, EarlyGeometricItemSpecificUsage,
    EarlyGeometricallyBoundedSurfaceShapeRepresentation,
    EarlyGeometricallyBoundedWireframeShapeRepresentation, EarlyGlobalUnitAssignedContext,
    EarlyGlobalUnitAssignedContextFull, EarlyGlobalUnitAssignedContextNoUncertainty,
    EarlyIntegerRepresentationItem, EarlyItemDefinedTransformation,
    EarlyItemIdentifiedRepresentationUsage, EarlyItemIdentifiedRepresentationUsageSelect,
    EarlyManifoldSurfaceShapeRepresentation, EarlyMappedItem, EarlyMeasureValue,
    EarlyMechanicalDesignAndDraughtingRelationship,
    EarlyMechanicalDesignGeometricPresentationRepresentation, EarlyModelGeometricView,
    EarlyParametricRepresentationContext, EarlyPlacedDatumTargetFeature,
    EarlyQualifiedRepresentationItem, EarlyRealRepresentationItem, EarlyRepresentationContext,
    EarlyRepresentationMap, EarlyRepresentationRelationship, EarlyShapeAspect,
    EarlyShapeAspectAssociativity, EarlyShapeAspectDerivingRelationship,
    EarlyShapeAspectRelationship, EarlyShapeDimensionRepresentation, EarlyShapeRepresentation,
    EarlyShapeRepresentationRelationship, EarlyShapeRepresentationWithParameters,
    EarlyTessellatedShapeRepresentation, EarlyToleranceZone, EarlyValueRepresentationItem,
};
use crate::entities::SimpleEntityHandler;
use crate::entities::shape_rep::descriptive_representation_item::DescriptiveRepresentationItemHandler;
use crate::ir::representation_item::{MeasureValue, ValueRepresentationItem};
use crate::ir::shape_rep::{
    CompoundItem, CompoundItemKind, CompoundRepresentationItem, ConstructiveGeometryRepr,
    DimensionItem, IiruDefinition, IiruIdentifiedItem, ItemIdentifiedRepresentationUsage, Mdgpr,
    ShapeDimensionRepresentation, ShapeRepresentationWithParameters, SrwpItem,
    TessellatedShapeRepresentation, UnitContext, UnitContextForm, UnitlessContext,
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

/// Lift one `INTEGER_REPRESENTATION_ITEM`.
pub(crate) fn lift_integer_representation_item(
    name: String,
    the_value: i64,
) -> EarlyIntegerRepresentationItem {
    EarlyIntegerRepresentationItem { name, the_value }
}

/// Lift one plain `DRAUGHTING_MODEL` (`Form::Simple`) from pre-resolved step ids.
pub(crate) fn lift_draughting_model(
    name: String,
    items: Vec<u64>,
    context_of_items: u64,
) -> EarlyDraughtingModel {
    EarlyDraughtingModel {
        name,
        items,
        context_of_items,
    }
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

/// Lift one plain `SHAPE_REPRESENTATION` from pre-resolved step ids. Shared by
/// the arena write path (Plain arm) and the product-driven paths (Group SR /
/// Fusion-CATIA indirect outer SR), which differ in input shape — hence the
/// raw-pieces signature.
pub(crate) fn lift_shape_representation(
    name: String,
    items: Vec<u64>,
    context_of_items: u64,
) -> EarlyShapeRepresentation {
    EarlyShapeRepresentation {
        name,
        items,
        context_of_items,
    }
}

/// Lift one `ADVANCED_BREP_SHAPE_REPRESENTATION` from pre-resolved step ids.
/// Shared by the arena write path (`advanced_brep_early`) and the product-driven
/// path; raw-pieces signature for the same reason as `lift_shape_representation`.
pub(crate) fn lift_advanced_brep_shape_representation(
    name: String,
    items: Vec<u64>,
    context_of_items: u64,
) -> EarlyAdvancedBrepShapeRepresentation {
    EarlyAdvancedBrepShapeRepresentation {
        name,
        items,
        context_of_items,
    }
}

/// Lift one `MANIFOLD_SURFACE_SHAPE_REPRESENTATION` from pre-resolved step ids
/// (frame axis + SBSM refs). Raw-pieces signature — shared by the arena arm and
/// the product-driven handler, which differ in input shape.
pub(crate) fn lift_manifold_surface_shape_representation(
    name: String,
    items: Vec<u64>,
    context_of_items: u64,
) -> EarlyManifoldSurfaceShapeRepresentation {
    EarlyManifoldSurfaceShapeRepresentation {
        name,
        items,
        context_of_items,
    }
}

/// Lift one `GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION` from
/// pre-resolved step ids (frame axis + GCS/GS ref).
pub(crate) fn lift_geometrically_bounded_surface_shape_representation(
    name: String,
    items: Vec<u64>,
    context_of_items: u64,
) -> EarlyGeometricallyBoundedSurfaceShapeRepresentation {
    EarlyGeometricallyBoundedSurfaceShapeRepresentation {
        name,
        items,
        context_of_items,
    }
}

/// Lift one `GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION` from
/// pre-resolved step ids (frame axis + GCS/GS ref).
pub(crate) fn lift_geometrically_bounded_wireframe_shape_representation(
    name: String,
    items: Vec<u64>,
    context_of_items: u64,
) -> EarlyGeometricallyBoundedWireframeShapeRepresentation {
    EarlyGeometricallyBoundedWireframeShapeRepresentation {
        name,
        items,
        context_of_items,
    }
}

/// Lift one `MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION`. `items`
/// are `STYLED_ITEM` arena ids → cached step ids (infallible); `context_of_items`
/// reuses `repr_context_attr`, always an `EntityRef` here (lower drops any
/// carrier whose context did not resolve).
pub(crate) fn lift_mechanical_design_geometric_presentation_representation(
    buf: &WriteBuffer,
    mdgpr: Mdgpr,
) -> EarlyMechanicalDesignGeometricPresentationRepresentation {
    let items = mdgpr.items.iter().map(|&id| buf.step_id(id)).collect();
    let Attribute::EntityRef(context_of_items) = buf.repr_context_attr(mdgpr.context) else {
        unreachable!("MDGPR context is guaranteed resolved by lower → EntityRef")
    };
    EarlyMechanicalDesignGeometricPresentationRepresentation {
        name: mdgpr.name,
        items,
        context_of_items,
    }
}

/// Lift one `SHAPE_DIMENSION_REPRESENTATION`. `items` are `DimensionItem`
/// (Item → fallible repr-item emitter / Descriptive → descriptive emitter);
/// `context_of_items` reuses `repr_context_attr` (always `EntityRef` — lower
/// drops any carrier whose context did not resolve).
pub(crate) fn lift_shape_dimension_representation(
    buf: &mut WriteBuffer,
    sdr: ShapeDimensionRepresentation,
) -> Result<EarlyShapeDimensionRepresentation, WriteError> {
    let mut items = Vec::with_capacity(sdr.items.len());
    for item in sdr.items {
        let step = match item {
            DimensionItem::Item(r) => buf.emit_representation_item_ref(r)?,
            DimensionItem::Descriptive(d) => buf.emit_descriptive_item(d),
        };
        items.push(step);
    }
    let Attribute::EntityRef(context_of_items) = buf.repr_context_attr(sdr.context) else {
        unreachable!("SDR context is guaranteed resolved by lower → EntityRef")
    };
    Ok(EarlyShapeDimensionRepresentation {
        name: sdr.name,
        items,
        context_of_items,
    })
}

/// Lift one `SHAPE_REPRESENTATION_WITH_PARAMETERS`. `items` are the 4-way
/// `SrwpItem` SELECT, each emitted to its step id; `context_of_items` reuses
/// `repr_context_attr` (always `EntityRef` — lower drops unresolved context).
pub(crate) fn lift_shape_representation_with_parameters(
    buf: &mut WriteBuffer,
    srwp: ShapeRepresentationWithParameters,
) -> Result<EarlyShapeRepresentationWithParameters, WriteError> {
    let mut items = Vec::with_capacity(srwp.items.len());
    for item in srwp.items {
        let step = match item {
            SrwpItem::Direction(id) => buf.emit_direction(id)?,
            SrwpItem::Placement(id) => buf.emit_axis2_placement_3d(id)?,
            SrwpItem::Descriptive(d) => DescriptiveRepresentationItemHandler::write(buf, d)?,
            SrwpItem::MeasureItem(id) => buf.step_id(id),
        };
        items.push(step);
    }
    let Attribute::EntityRef(context_of_items) = buf.repr_context_attr(srwp.context) else {
        unreachable!("SRWP context is guaranteed resolved by lower → EntityRef")
    };
    Ok(EarlyShapeRepresentationWithParameters {
        name: srwp.name,
        items,
        context_of_items,
    })
}

/// Lift one `COMPOUND_REPRESENTATION_ITEM`. Each child emits to its step id
/// (Descriptive → handler write / Item → repr-item emitter); the Set/List kind
/// selects the synth SELECT variant.
pub(crate) fn lift_compound_representation_item(
    buf: &mut WriteBuffer,
    cri: CompoundRepresentationItem,
) -> Result<EarlyCompoundRepresentationItem, WriteError> {
    let mut steps = Vec::with_capacity(cri.item_element.items.len());
    for item in cri.item_element.items {
        let step = match item {
            CompoundItem::Descriptive(d) => DescriptiveRepresentationItemHandler::write(buf, d)?,
            CompoundItem::Item(r) => buf.emit_representation_item_ref(r)?,
        };
        steps.push(step);
    }
    let item_element = match cri.item_element.kind {
        CompoundItemKind::Set => EarlyCompoundItemDefinition::SetRepresentationItem(steps),
        CompoundItemKind::List => EarlyCompoundItemDefinition::ListRepresentationItem(steps),
    };
    Ok(EarlyCompoundRepresentationItem {
        name: cri.name,
        item_element,
    })
}

/// Lift one `ITEM_IDENTIFIED_REPRESENTATION_USAGE`. `definition` (5-way) /
/// `used_representation` resolve to cached step ids; `identified_item` emits to
/// the synth SELECT (single ref → `EntityRef`, Set/List → typed aggregate).
pub(crate) fn lift_item_identified_representation_usage(
    buf: &mut WriteBuffer,
    iiru: ItemIdentifiedRepresentationUsage,
) -> Result<EarlyItemIdentifiedRepresentationUsage, WriteError> {
    let definition = match iiru.definition {
        IiruDefinition::ShapeAspect(id) => buf.step_id(id),
        IiruDefinition::Datum(id) => buf.step_id(id),
        IiruDefinition::DatumFeature(id) => buf.step_id(id),
        IiruDefinition::DimensionalSize(id) => buf.step_id(id),
        IiruDefinition::GeometricTolerance(id) => buf.step_id(id),
    };
    let used_representation = buf.step_id(iiru.used_representation);
    let identified_item = match iiru.identified_item {
        IiruIdentifiedItem::Item(r) => EarlyItemIdentifiedRepresentationUsageSelect::EntityRef(
            buf.emit_representation_item_ref(r)?,
        ),
        IiruIdentifiedItem::Compound { kind, items } => {
            let mut steps = Vec::with_capacity(items.len());
            for r in items {
                steps.push(buf.emit_representation_item_ref(r)?);
            }
            match kind {
                CompoundItemKind::Set => {
                    EarlyItemIdentifiedRepresentationUsageSelect::SetRepresentationItem(steps)
                }
                CompoundItemKind::List => {
                    EarlyItemIdentifiedRepresentationUsageSelect::ListRepresentationItem(steps)
                }
            }
        }
    };
    Ok(EarlyItemIdentifiedRepresentationUsage {
        name: iiru.name,
        description: iiru.description,
        definition,
        used_representation,
        identified_item,
    })
}

/// Lift one `CAMERA_IMAGE`. `mapping_source` / `mapping_target` are the child
/// output step ids (the handler resolves the camera-usage step id and emits the
/// planar extent first).
pub(crate) fn lift_camera_image(
    name: String,
    mapping_source: u64,
    mapping_target: u64,
) -> EarlyCameraImage {
    EarlyCameraImage {
        name,
        mapping_source,
        mapping_target,
    }
}

/// Lift one `CAMERA_IMAGE_3D_WITH_SCALE` (AND-combined complex). Same child step
/// ids as the plain `CAMERA_IMAGE`.
pub(crate) fn lift_camera_image_3d_with_scale(
    name: String,
    mapping_source: u64,
    mapping_target: u64,
) -> EarlyCameraImage3dWithScale {
    EarlyCameraImage3dWithScale {
        mapping_source,
        mapping_target,
        name,
    }
}

/// Lift one `REPRESENTATION_MAP` (`Itself`). `mapping_origin` / `mapped_representation`
/// are the child output step ids (the handler emits the origin item and resolves
/// the mapped representation step id first).
pub(crate) fn lift_representation_map(
    mapping_origin: u64,
    mapped_representation: u64,
) -> EarlyRepresentationMap {
    EarlyRepresentationMap {
        mapping_origin,
        mapped_representation,
    }
}

/// Lift one `MAPPED_ITEM` (`Itself`). `mapping_source` / `mapping_target` are the
/// child output step ids.
pub(crate) fn lift_mapped_item(
    name: String,
    mapping_source: u64,
    mapping_target: u64,
) -> EarlyMappedItem {
    EarlyMappedItem {
        name,
        mapping_source,
        mapping_target,
    }
}

/// Lift one `ITEM_DEFINED_TRANSFORMATION`. `transform_item_1` / `transform_item_2`
/// are the child placement output step ids. `name` / `description` are not modelled
/// by `Transform3d`, so they are re-emitted as `''` (matching the legacy handler).
pub(crate) fn lift_item_defined_transformation(
    transform_item_1: u64,
    transform_item_2: u64,
) -> EarlyItemDefinedTransformation {
    EarlyItemDefinedTransformation {
        name: String::new(),
        description: Some(String::new()),
        transform_item_1,
        transform_item_2,
    }
}

/// Lift one `DEFAULT_MODEL_GEOMETRIC_VIEW`. `item` / `rep` / `of_shape` are child
/// output step ids. Descriptions are always re-emitted (`Some`, possibly empty),
/// matching the legacy handler which wrote them unconditionally as strings.
#[allow(clippy::too_many_arguments)]
pub(crate) fn lift_default_model_geometric_view(
    co_name: String,
    co_description: String,
    item: u64,
    rep: u64,
    sa_name: String,
    sa_description: String,
    of_shape: u64,
) -> EarlyDefaultModelGeometricView {
    EarlyDefaultModelGeometricView {
        name: co_name,
        description: Some(co_description),
        item,
        rep,
        name_2: sa_name,
        description_2: Some(sa_description),
        of_shape,
    }
}

/// Lift the complex `GLOBAL_UNIT_ASSIGNED_CONTEXT`. The unit / uncertainty arena
/// entries were already emitted by `emit_units_pool_if_set`, so each ref
/// resolves to its step id. A non-empty `uncertainty` selects the `Full` case
/// (adds the `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT` part). Only called for
/// [`UnitContextForm::Complex`]; the simple form emits via its own hand path.
pub(crate) fn lift_global_unit_assigned_context(
    buf: &WriteBuffer,
    units: &UnitContext,
) -> EarlyGlobalUnitAssignedContext {
    let UnitContextForm::Complex {
        coordinate_space_dimension,
        repr_identifier,
        repr_type,
    } = &units.form
    else {
        unreachable!("lift_global_unit_assigned_context called on a non-Complex form")
    };
    let unit_steps: Vec<u64> = units.units.iter().map(|id| buf.step_id(id)).collect();
    if units.uncertainty.is_empty() {
        EarlyGlobalUnitAssignedContext::NoUncertainty(EarlyGlobalUnitAssignedContextNoUncertainty {
            coordinate_space_dimension: *coordinate_space_dimension,
            units: unit_steps,
            context_identifier: repr_identifier.clone(),
            context_type: repr_type.clone(),
        })
    } else {
        let uncertainty: Vec<u64> = units.uncertainty.iter().map(|id| buf.step_id(id)).collect();
        EarlyGlobalUnitAssignedContext::Full(EarlyGlobalUnitAssignedContextFull {
            coordinate_space_dimension: *coordinate_space_dimension,
            uncertainty,
            units: unit_steps,
            context_identifier: repr_identifier.clone(),
            context_type: repr_type.clone(),
        })
    }
}

/// Lift `GEOMETRIC_ITEM_SPECIFIC_USAGE`. The handler write pre-resolves the
/// three refs to emitted step ids (`emit_shape_aspect_ref` / `step_id` /
/// `emit_representation_item_ref`); `identified_item` re-wraps as the single
/// `EntityRef` member (GISU never carries the SET/LIST forms).
pub(crate) fn lift_geometric_item_specific_usage(
    name: String,
    description: Option<String>,
    def_step: u64,
    used_step: u64,
    item_step: u64,
) -> EarlyGeometricItemSpecificUsage {
    EarlyGeometricItemSpecificUsage {
        name,
        description,
        definition: def_step,
        used_representation: used_step,
        identified_item: EarlyItemIdentifiedRepresentationUsageSelect::EntityRef(item_step),
    }
}
