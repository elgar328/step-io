//! PMI-domain `lower` fns (leaf qualifiers + datum-free form tolerances).
//! See the [module docs](super) for the lowering contract.

use crate::early::model::{
    EarlyAngularLocation, EarlyAngularSize, EarlyAngularityTolerance,
    EarlyAnnotationCurveOccurrence, EarlyAnnotationOccurrence, EarlyAnnotationPlane,
    EarlyAnnotationSymbolOccurrence, EarlyAnnotationTextOccurrence, EarlyCircularRunoutTolerance,
    EarlyConcentricityTolerance, EarlyCylindricityTolerance, EarlyDatum, EarlyDatumFeature,
    EarlyDimensionalLocation, EarlyDimensionalSize, EarlyDirectedDimensionalLocation,
    EarlyDraughtingAnnotationOccurrence, EarlyDraughtingCallout, EarlyFlatnessTolerance,
    EarlyGeometricToleranceRelationship, EarlyLeaderCurve, EarlyLeaderDirectedCallout,
    EarlyLeaderTerminator, EarlyMeasureQualification, EarlyParallelismTolerance,
    EarlyPerpendicularityTolerance, EarlyRoundnessTolerance, EarlyStraightnessTolerance,
    EarlySurfaceProfileTolerance, EarlySymmetryTolerance, EarlyToleranceZoneForm,
    EarlyTotalRunoutTolerance, EarlyTypeQualifier, EarlyValueFormatTypeQualifier,
};
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::pmi::{
    AngularLocationData, AnnotationCurveOccurrence, AnnotationOccurrence, AnnotationPlane,
    AnnotationSymbolOccurrence, AnnotationTextOccurrence, Datum, DatumFeature, DimensionalLocation,
    DimensionalLocationData, DimensionalSize, DimensionalSizeKind, DraughtingAnnotationOccurrence,
    DraughtingCallout, DraughtingCalloutData, GeometricTolerance, GeometricToleranceRelationship,
    GeometricToleranceWithDatumReference, LeaderCurve, LeaderTerminator, MeasureQualification,
    PlainAnnotationCurveOccurrence, PlainAnnotationOccurrence, PmiPool, ToleranceZoneForm,
    TypeQualifier, ValueFormatTypeQualifier, ValueQualifier,
};
use crate::reader::ReaderContext;

/// Lower one `TOLERANCE_ZONE_FORM`.
pub(crate) fn lower_tolerance_zone_form(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyToleranceZoneForm,
) {
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .tolerance_zone_forms
        .push(ToleranceZoneForm { name: early.name });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `TYPE_QUALIFIER`.
pub(crate) fn lower_type_qualifier(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyTypeQualifier,
) {
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .type_qualifiers
        .push(TypeQualifier { name: early.name });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `VALUE_FORMAT_TYPE_QUALIFIER`.
pub(crate) fn lower_value_format_type_qualifier(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyValueFormatTypeQualifier,
) {
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .value_format_type_qualifiers
        .push(ValueFormatTypeQualifier {
            format_type: early.format_type,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Shared datum-free form-tolerance lowering: resolve magnitude (2-path) +
/// toleranced target, both unresolved = silent drop (symmetric on re-read).
/// The L1 OPTIONAL magnitude also drops on `$` (the legacy required read
/// errored there; no corpus file carries one).
#[allow(clippy::type_complexity)]
fn lower_form_tolerance_common(
    ctx: &ReaderContext,
    name: String,
    description: Option<String>,
    magnitude: Option<u64>,
    toleranced_shape_aspect: u64,
) -> Option<crate::ir::pmi::GeometricToleranceData> {
    let magnitude_ref = magnitude?;
    let magnitude = crate::entities::pmi::resolve_tolerance_magnitude(ctx, magnitude_ref)?;
    let toleranced_shape_aspect =
        crate::entities::pmi::resolve_geometric_tolerance_target(ctx, toleranced_shape_aspect)?;
    Some(crate::ir::pmi::GeometricToleranceData {
        name,
        // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
        description: description.unwrap_or_default(),
        magnitude,
        toleranced_shape_aspect,
        modifiers: Vec::new(),
        unit_size: None,
        defined_area_unit: None,
    })
}

/// Lower one `FLATNESS_TOLERANCE` (datum-free form).
pub(crate) fn lower_flatness_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyFlatnessTolerance,
) {
    let Some(data) = lower_form_tolerance_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
    ) else {
        return;
    };
    crate::entities::pmi::push_geometric_tolerance(
        ctx,
        entity_id,
        GeometricTolerance::Flatness(data),
    );
}

/// Lower one `SURFACE_PROFILE_TOLERANCE` (datum-free form).
pub(crate) fn lower_surface_profile_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlySurfaceProfileTolerance,
) {
    let Some(data) = lower_form_tolerance_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
    ) else {
        return;
    };
    crate::entities::pmi::push_geometric_tolerance(
        ctx,
        entity_id,
        GeometricTolerance::SurfaceProfile(data),
    );
}

/// Shared lowering for the simple `GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE`
/// subtypes (`angularity` / `circular_runout` / `concentricity` / `parallelism`
/// / `perpendicularity` / `symmetry` / `total_runout`): resolve `magnitude` /
/// `shape_aspect` / `datum_system` via the shared builder, then push under
/// `kind`. A `$` magnitude (corpus-absent — the legacy read required it) drops
/// the entity.
#[allow(clippy::too_many_arguments)]
fn lower_gt_with_datum_reference(
    ctx: &mut ReaderContext,
    entity_id: u64,
    name: String,
    description: Option<String>,
    magnitude: Option<u64>,
    toleranced_shape_aspect: u64,
    datum_system: &[u64],
    kind: fn(
        crate::ir::pmi::GeometricToleranceWithDatumReferenceData,
    ) -> GeometricToleranceWithDatumReference,
) {
    let Some(mag) = magnitude else {
        return;
    };
    let Some(data) = crate::entities::pmi::build_gt_with_datum_reference_data(
        ctx,
        name,
        description.unwrap_or_default(),
        mag,
        toleranced_shape_aspect,
        datum_system,
        Vec::new(),
        None,
    ) else {
        return;
    };
    crate::entities::pmi::push_gt_with_datum_reference(ctx, entity_id, kind(data));
}

/// Lower one `ANGULARITY_TOLERANCE`.
pub(crate) fn lower_angularity_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAngularityTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::Angularity,
    );
}

/// Lower one `CIRCULAR_RUNOUT_TOLERANCE`.
pub(crate) fn lower_circular_runout_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCircularRunoutTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::CircularRunout,
    );
}

/// Lower one `CONCENTRICITY_TOLERANCE`.
pub(crate) fn lower_concentricity_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyConcentricityTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::Concentricity,
    );
}

/// Lower one `PARALLELISM_TOLERANCE`.
pub(crate) fn lower_parallelism_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyParallelismTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::Parallelism,
    );
}

/// Lower one `PERPENDICULARITY_TOLERANCE`.
pub(crate) fn lower_perpendicularity_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPerpendicularityTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::Perpendicularity,
    );
}

/// Lower one `SYMMETRY_TOLERANCE`.
pub(crate) fn lower_symmetry_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlySymmetryTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::Symmetry,
    );
}

/// Lower one `TOTAL_RUNOUT_TOLERANCE`.
pub(crate) fn lower_total_runout_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyTotalRunoutTolerance,
) {
    lower_gt_with_datum_reference(
        ctx,
        entity_id,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
        &early.datum_system,
        GeometricToleranceWithDatumReference::TotalRunout,
    );
}

/// Lower one `STRAIGHTNESS_TOLERANCE` (datum-free form).
pub(crate) fn lower_straightness_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyStraightnessTolerance,
) {
    let Some(data) = lower_form_tolerance_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
    ) else {
        return;
    };
    crate::entities::pmi::push_geometric_tolerance(
        ctx,
        entity_id,
        GeometricTolerance::Straightness(data),
    );
}

/// Lower one `ROUNDNESS_TOLERANCE` (datum-free form).
pub(crate) fn lower_roundness_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyRoundnessTolerance,
) {
    let Some(data) = lower_form_tolerance_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
    ) else {
        return;
    };
    crate::entities::pmi::push_geometric_tolerance(
        ctx,
        entity_id,
        GeometricTolerance::Roundness(data),
    );
}

/// Lower one `CYLINDRICITY_TOLERANCE` (datum-free form).
pub(crate) fn lower_cylindricity_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCylindricityTolerance,
) {
    let Some(data) = lower_form_tolerance_common(
        ctx,
        early.name,
        early.description,
        early.magnitude,
        early.toleranced_shape_aspect,
    ) else {
        return;
    };
    crate::entities::pmi::push_geometric_tolerance(
        ctx,
        entity_id,
        GeometricTolerance::Cylindricity(data),
    );
}

/// L1 LOGICAL → the legacy `read_bool`'s `bool` (`.U.` drops).
fn logical_to_bool(l: crate::ir::geometry::Logical) -> Option<bool> {
    match l {
        crate::ir::geometry::Logical::True => Some(true),
        crate::ir::geometry::Logical::False => Some(false),
        crate::ir::geometry::Logical::Unknown => None,
    }
}

/// Lower one `DATUM` (unresolved `of_shape` product / `.U.` = silent drop).
pub(crate) fn lower_datum(ctx: &mut ReaderContext, entity_id: u64, early: EarlyDatum) {
    let Some(product_definitional) = logical_to_bool(early.product_definitional) else {
        return;
    };
    let Some(target) = ctx.product_of_pds(early.of_shape) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .datums
        .push(Datum {
            name: early.name,
            // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
            description: early.description.unwrap_or_default(),
            target,
            product_definitional,
            identification: early.identification,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one plain `DIMENSIONAL_SIZE` (unresolved `applies_to` = silent drop).
pub(crate) fn lower_dimensional_size(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDimensionalSize,
) {
    let Some(applies_to) =
        crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref(
            ctx,
            early.applies_to,
        )
    else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .dimensional_sizes
        .push(DimensionalSize {
            applies_to,
            name: early.name,
            kind: DimensionalSizeKind::Plain,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `GEOMETRIC_TOLERANCE_RELATIONSHIP` (either end unresolved =
/// silent drop).
pub(crate) fn lower_geometric_tolerance_relationship(
    ctx: &mut ReaderContext,
    early: EarlyGeometricToleranceRelationship,
) {
    let Some(relating) = crate::entities::pmi::resolve_geometric_tolerance_ref(
        ctx,
        early.relating_geometric_tolerance,
    ) else {
        return;
    };
    let Some(related) = crate::entities::pmi::resolve_geometric_tolerance_ref(
        ctx,
        early.related_geometric_tolerance,
    ) else {
        return;
    };
    ctx.pmi
        .get_or_insert_with(PmiPool::default)
        .geometric_tolerance_relationships
        .push(GeometricToleranceRelationship {
            name: early.name,
            description: early.description,
            relating,
            related,
        });
}

/// Lower one `MEASURE_QUALIFICATION` (unresolved measure = silent drop;
/// unresolved qualifier members skip individually).
pub(crate) fn lower_measure_qualification(
    ctx: &mut ReaderContext,
    early: EarlyMeasureQualification,
) {
    let Some(qualified_measure) = ctx
        .id_cache
        .get::<crate::ir::id::MeasureWithUnitId>(early.qualified_measure)
    else {
        return;
    };
    let mut qualifiers = Vec::with_capacity(early.qualifiers.len());
    for r in early.qualifiers {
        if let Some(q) = ValueQualifier::resolve_select(ctx, r) {
            qualifiers.push(q);
        }
    }
    ctx.pmi
        .get_or_insert_with(PmiPool::default)
        .measure_qualifications
        .push(MeasureQualification {
            name: early.name,
            description: early.description,
            qualified_measure,
            qualifiers,
        });
}

/// Shared `DIMENSIONAL_LOCATION` endpoint resolve (either unresolved =
/// silent drop) + the `$`→"" description collapse.
fn lower_dl_common(
    ctx: &ReaderContext,
    name: String,
    description: Option<String>,
    relating: u64,
    related: u64,
) -> Option<DimensionalLocationData> {
    let relating_shape_aspect =
        crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref(
            ctx, relating,
        )?;
    let related_shape_aspect =
        crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref(
            ctx, related,
        )?;
    Some(DimensionalLocationData {
        name,
        description: description.unwrap_or_default(),
        relating_shape_aspect,
        related_shape_aspect,
    })
}

/// Lower one plain `DIMENSIONAL_LOCATION`.
pub(crate) fn lower_dimensional_location(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDimensionalLocation,
) {
    let Some(data) = lower_dl_common(
        ctx,
        early.name,
        early.description,
        early.relating_shape_aspect,
        early.related_shape_aspect,
    ) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .dimensional_locations
        .push(DimensionalLocation::Plain(data));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DIRECTED_DIMENSIONAL_LOCATION` (same 4-attr body).
pub(crate) fn lower_directed_dimensional_location(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDirectedDimensionalLocation,
) {
    let Some(data) = lower_dl_common(
        ctx,
        early.name,
        early.description,
        early.relating_shape_aspect,
        early.related_shape_aspect,
    ) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .dimensional_locations
        .push(DimensionalLocation::Directed(data));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `ANGULAR_LOCATION` (adds the `angle_relator` enum, converted
/// by the bind's enum hint — unknown tokens error there, like the legacy
/// read).
pub(crate) fn lower_angular_location(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAngularLocation,
) {
    let Some(data) = lower_dl_common(
        ctx,
        early.name,
        early.description,
        early.relating_shape_aspect,
        early.related_shape_aspect,
    ) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .dimensional_locations
        .push(DimensionalLocation::Angular(AngularLocationData {
            name: data.name,
            description: data.description,
            relating_shape_aspect: data.relating_shape_aspect,
            related_shape_aspect: data.related_shape_aspect,
            angle_selection: early.angle_selection,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `ANGULAR_SIZE` (`DIMENSIONAL_SIZE` plus `angle_relator`).
pub(crate) fn lower_angular_size(ctx: &mut ReaderContext, entity_id: u64, early: EarlyAngularSize) {
    let Some(applies_to) =
        crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref(
            ctx,
            early.applies_to,
        )
    else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .dimensional_sizes
        .push(DimensionalSize {
            applies_to,
            name: early.name,
            kind: DimensionalSizeKind::Angular(early.angle_selection),
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one plain `DATUM_FEATURE` (`Itself` variant of the family; the
/// complex DSWDF keeps its own handler).
pub(crate) fn lower_datum_feature(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDatumFeature,
) {
    let Some(product_definitional) = logical_to_bool(early.product_definitional) else {
        return;
    };
    let Some(target) = ctx.product_of_pds(early.of_shape) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .datum_features
        .push(DatumFeature::Itself(crate::ir::DatumFeatureData {
            name: early.name,
            // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
            description: early.description.unwrap_or_default(),
            target,
            product_definitional,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one plain `DRAUGHTING_CALLOUT` (unresolved content members skip
/// individually via the shared contents resolver).
pub(crate) fn lower_draughting_callout(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDraughtingCallout,
) {
    let contents = crate::entities::pmi::read_draughting_callout_contents(ctx, &early.contents);
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .draughting_callouts
        .push(DraughtingCallout::Plain(DraughtingCalloutData {
            name: early.name,
            contents,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `LEADER_DIRECTED_CALLOUT` (same body, `LeaderDirected` variant).
pub(crate) fn lower_leader_directed_callout(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyLeaderDirectedCallout,
) {
    let contents = crate::entities::pmi::read_draughting_callout_contents(ctx, &early.contents);
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .draughting_callouts
        .push(DraughtingCallout::LeaderDirected(DraughtingCalloutData {
            name: early.name,
            contents,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the styled `ANNOTATION_TEXT_OCCURRENCE` complex. `styles` keeps only
/// refs resolving to a `PresentationStyleAssignment` (others skipped); `item`
/// resolves through `resolve_representation_item_ref` — an unresolved item drops
/// the whole occurrence (symmetric on re-read). Verbatim port of the legacy read.
pub(crate) fn lower_annotation_text_occurrence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAnnotationTextOccurrence,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::AnnotationTextOccurrence(
            AnnotationTextOccurrence {
                name: early.name.clone(),
                styles,
                item,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the styled `LEADER_CURVE` complex. `styles` keeps only resolved
/// `PresentationStyleAssignment` refs; `item` must resolve to a `CurveId`
/// (unresolved drops the occurrence). Verbatim port of the legacy read.
pub(crate) fn lower_leader_curve(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyLeaderCurve,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = ctx.id_cache.get::<crate::ir::id::CurveId>(early.item) else {
        return; // item unresolved — drop the occurrence
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_curve_occurrences
        .push(AnnotationCurveOccurrence::LeaderCurve(LeaderCurve {
            name: early.name.clone(),
            styles,
            item,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the styled `LEADER_TERMINATOR` complex. Like `LEADER_CURVE` but `item`
/// resolves through `resolve_representation_item_ref` and `annotated_curve` must
/// resolve to an `AnnotationCurveOccurrenceId` (a `LEADER_CURVE`); either
/// unresolved drops the occurrence. Verbatim port of the legacy read.
pub(crate) fn lower_leader_terminator(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyLeaderTerminator,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let Some(annotated_curve) = ctx
        .id_cache
        .get::<crate::ir::id::AnnotationCurveOccurrenceId>(early.annotated_curve)
    else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::LeaderTerminator(LeaderTerminator {
            name: early.name.clone(),
            styles,
            item,
            annotated_curve,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the plain `ANNOTATION_OCCURRENCE` supertype. `styles` keeps only
/// resolved `PresentationStyleAssignment` refs; `item` resolves through
/// `resolve_representation_item_ref` (unresolved drops the occurrence,
/// symmetric on re-read). Verbatim port of the legacy read.
pub(crate) fn lower_annotation_occurrence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAnnotationOccurrence,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::Plain(PlainAnnotationOccurrence {
            name: early.name.clone(),
            styles,
            item,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `DRAUGHTING_ANNOTATION_OCCURRENCE` (same shape as the supertype).
/// Verbatim port of the legacy read.
pub(crate) fn lower_draughting_annotation_occurrence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyDraughtingAnnotationOccurrence,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::DraughtingAnnotationOccurrence(
            DraughtingAnnotationOccurrence {
                name: early.name.clone(),
                styles,
                item,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `ANNOTATION_SYMBOL_OCCURRENCE` (same shape as the supertype).
/// Verbatim port of the legacy read.
pub(crate) fn lower_annotation_symbol_occurrence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAnnotationSymbolOccurrence,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::AnnotationSymbolOccurrence(
            AnnotationSymbolOccurrence {
                name: early.name.clone(),
                styles,
                item,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower the plain `ANNOTATION_CURVE_OCCURRENCE` supertype (not `LEADER_CURVE`).
/// `item` resolves through `resolve_representation_item_ref` (a plain `CURVE` or
/// e.g. `GEOMETRIC_CURVE_SET`); unresolved drops the occurrence. Verbatim port
/// of the legacy read.
pub(crate) fn lower_annotation_curve_occurrence(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAnnotationCurveOccurrence,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_curve_occurrences
        .push(AnnotationCurveOccurrence::Plain(
            PlainAnnotationCurveOccurrence {
                name: early.name.clone(),
                styles,
                item,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower `ANNOTATION_PLANE`. Same styled shape as the other occurrences; the
/// 4th attr `elements` (an `annotation_plane_element` list) is not modelled, so
/// `early.elements` is ignored here and re-emitted as `$` on write. `item`
/// resolves through `resolve_representation_item_ref` (unresolved drops the
/// occurrence, symmetric on re-read). Verbatim port of the legacy read.
pub(crate) fn lower_annotation_plane(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAnnotationPlane,
) {
    let mut styles = Vec::with_capacity(early.styles.len());
    for &r in &early.styles {
        if let Some(psa_id) = ctx
            .id_cache
            .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
        {
            styles.push(psa_id);
        }
    }
    let Some(item) = resolve_representation_item_ref(ctx, early.item) else {
        return;
    };
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .annotation_occurrences
        .push(AnnotationOccurrence::AnnotationPlane(AnnotationPlane {
            name: early.name.clone(),
            styles,
            item,
        }));
    ctx.id_cache.insert(entity_id, id);
}
