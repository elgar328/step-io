//! PMI-domain `lower` fns (leaf qualifiers + datum-free form tolerances).
//! See the [module docs](super) for the lowering contract.

use crate::early::model::{
    EarlyCylindricityTolerance, EarlyFlatnessTolerance, EarlyRoundnessTolerance,
    EarlyStraightnessTolerance, EarlySurfaceProfileTolerance, EarlyToleranceZoneForm,
    EarlyTypeQualifier, EarlyValueFormatTypeQualifier,
};
use crate::ir::pmi::{
    GeometricTolerance, PmiPool, ToleranceZoneForm, TypeQualifier, ValueFormatTypeQualifier,
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
