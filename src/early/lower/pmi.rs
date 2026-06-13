//! PMI-domain `lower` fns (leaf qualifiers + datum-free form tolerances).
//! See the [module docs](super) for the lowering contract.

use crate::early::model::{
    EarlyAngularLocation, EarlyAngularSize, EarlyCylindricityTolerance, EarlyDatum,
    EarlyDimensionalLocation, EarlyDimensionalSize, EarlyDirectedDimensionalLocation,
    EarlyFlatnessTolerance, EarlyGeometricToleranceRelationship, EarlyMeasureQualification,
    EarlyRoundnessTolerance, EarlyStraightnessTolerance, EarlySurfaceProfileTolerance,
    EarlyToleranceZoneForm, EarlyTypeQualifier, EarlyValueFormatTypeQualifier,
};
use crate::ir::pmi::{
    AngularLocationData, Datum, DimensionalLocation, DimensionalLocationData, DimensionalSize,
    DimensionalSizeKind, GeometricTolerance, GeometricToleranceRelationship, MeasureQualification,
    PmiPool, ToleranceZoneForm, TypeQualifier, ValueFormatTypeQualifier, ValueQualifier,
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
