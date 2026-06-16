//! Shape-representation-domain `lower` fns (the representation relationship
//! cluster: base `REPRESENTATION_RELATIONSHIP` + `SHAPE_REPRESENTATION_
//! RELATIONSHIP`). See the [module docs](super) for the lowering contract.
//!
//! Neither entity records a typed correspondence: nothing registers in
//! `id_cache` (the arena's only consumer is the writer's emit loop), and the
//! SDR indirection cache (`srr_equiv_map`) stays a dispatch-side raw-id map —
//! its values are step ids (not L2 arena ids) probed by post-passes against
//! the equally raw-keyed geometry payload maps.

use crate::early::model::{
    EarlyAllAroundShapeAspect, EarlyCentreOfSymmetry, EarlyCharacterizedItemWithinRepresentation,
    EarlyCompositeGroupShapeAspect, EarlyCompositeShapeAspect, EarlyCompoundItemDefinition,
    EarlyCompoundRepresentationItem, EarlyConstructiveGeometryRepresentation,
    EarlyConstructiveGeometryRepresentationRelationship, EarlyDatumSystem, EarlyDatumTarget,
    EarlyDescriptiveRepresentationItem, EarlyMeasureValue,
    EarlyMechanicalDesignAndDraughtingRelationship,
    EarlyMechanicalDesignGeometricPresentationRepresentation, EarlyModelGeometricView,
    EarlyParametricRepresentationContext, EarlyPlacedDatumTargetFeature,
    EarlyQualifiedRepresentationItem, EarlyRealRepresentationItem, EarlyRepresentationContext,
    EarlyRepresentationRelationship, EarlyShapeAspect, EarlyShapeDimensionRepresentation,
    EarlyShapeRepresentationRelationship, EarlyShapeRepresentationWithParameters,
    EarlyTessellatedShapeRepresentation, EarlyToleranceZone, EarlyValueRepresentationItem,
};
use crate::entities::tessellation::resolve_tessellated_item_ref;
use crate::ir::error::ConvertError;
use crate::ir::representation_item::{
    MeasureValue, QualifiedRepresentationItem, QualifierRef, RepresentationItem,
    ValueRepresentationItem,
};
use crate::ir::shape_rep::{
    AllAroundShapeAspect, CentreOfSymmetry, CharacterizedItemWithinRepresentation,
    CharacterizedObject, CharacterizedObjectData, CompositeGroupShapeAspect,
    CompositeShapeAspectKind, CompoundItem, CompoundItemElement, CompoundItemKind,
    CompoundRepresentationItem, ConstructiveGeometryRepr,
    ConstructiveGeometryRepresentationRelationship, DatumSystem, DatumTarget, Mdgpr,
    MechanicalDesignAndDraughtingRelationship, ModelGeometricView, NumericRepresentationItem,
    PlacedDatumTargetFeature, RealRepresentationItem, Representation, RepresentationContextRef,
    RepresentationRelationship, RepresentationRelationshipData, ShapeAspect,
    ShapeAspectRelationship, ShapeAspectRelationshipKind, ShapeDimensionRepresentation,
    ShapeRepresentationRelationshipIr, ShapeRepresentationWithParameters, SrwpItem,
    TessellatedShapeRepresentation, ToleranceZone, UnitlessContext,
};
use crate::ir::visualization::VisualizationPool;
use crate::reader::ReaderContext;

/// Lower one base `REPRESENTATION_RELATIONSHIP` (`Itself` carrier): resolve
/// both reps and push the faithful arena entry. An unmodelled rep surfaces as
/// a warning and skips the entry (legacy leniency, per-side detail).
pub(crate) fn lower_representation_relationship(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyRepresentationRelationship,
) {
    let rep_1_ref = early.rep_1;
    let rep_2_ref = early.rep_2;
    let Some(rep_1) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(rep_1_ref)
    else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "REPRESENTATION_RELATIONSHIP #{entity_id}.rep_1 #{rep_1_ref} did not \
                 resolve to a modelled REPRESENTATION subtype — skipping"
            ),
        });
        return;
    };
    let Some(rep_2) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(rep_2_ref)
    else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "REPRESENTATION_RELATIONSHIP #{entity_id}.rep_2 #{rep_2_ref} did not \
                 resolve to a modelled REPRESENTATION subtype — skipping"
            ),
        });
        return;
    };
    ctx.representation_relationships
        .push(RepresentationRelationship::Itself(
            RepresentationRelationshipData {
                name: early.name,
                // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
                description: early.description.unwrap_or_default(),
                rep_1,
                rep_2,
            },
        ));
}

/// Lower one `CONSTRUCTIVE_GEOMETRY_REPRESENTATION_RELATIONSHIP`: resolve
/// both reps and push the faithful arena entry. An unresolved ref silently
/// drops the carrier (legacy leniency — symmetric on re-read, no warning).
pub(crate) fn lower_constructive_geometry_representation_relationship(
    ctx: &mut ReaderContext,
    early: EarlyConstructiveGeometryRepresentationRelationship,
) {
    let Some(rep_1) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(early.rep_1)
    else {
        return;
    };
    let Some(rep_2) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(early.rep_2)
    else {
        return;
    };
    ctx.representation_relationships.push(
        RepresentationRelationship::ConstructiveGeometryRepresentationRelationship(
            ConstructiveGeometryRepresentationRelationship {
                name: early.name,
                // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
                description: early.description.unwrap_or_default(),
                rep_1,
                rep_2,
            },
        ),
    );
}

/// Lower one `MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP`: resolve both
/// reps and push the faithful arena entry. An unmodelled rep surfaces as a
/// warning and skips the entry (legacy leniency, per-side detail).
pub(crate) fn lower_mechanical_design_and_draughting_relationship(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyMechanicalDesignAndDraughtingRelationship,
) {
    let rep_1_ref = early.rep_1;
    let rep_2_ref = early.rep_2;
    let Some(rep_1) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(rep_1_ref)
    else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "MDDR #{entity_id}.rep_1 #{rep_1_ref} did not resolve to a \
                 modelled REPRESENTATION subtype — skipping"
            ),
        });
        return;
    };
    let Some(rep_2) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(rep_2_ref)
    else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "MDDR #{entity_id}.rep_2 #{rep_2_ref} did not resolve to a \
                 modelled REPRESENTATION subtype — skipping"
            ),
        });
        return;
    };
    ctx.representation_relationships.push(
        RepresentationRelationship::MechanicalDesignAndDraughtingRelationship(
            MechanicalDesignAndDraughtingRelationship {
                name: early.name,
                // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
                description: early.description.unwrap_or_default(),
                rep_1,
                rep_2,
            },
        ),
    );
}

/// Lower one `SHAPE_REPRESENTATION_RELATIONSHIP`: record the SDR indirection
/// equivalence (when exactly one side is a known geometry-carrying rep), then
/// resolve both reps and push the faithful arena entry. Unmodelled reps
/// surface as one combined warning and skip the entry (legacy leniency).
pub(crate) fn lower_shape_representation_relationship(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyShapeRepresentationRelationship,
) {
    let rep_1 = early.rep_1;
    let rep_2 = early.rep_2;

    // Keep the SDR indirection cache populated so the Fusion 360 /
    // CATIA `SDR → plain SR → SRR → ABSR/MSSR` chain still resolves
    // through `srr_equiv_map`. Arena push runs alongside so all SRRs
    // (1-to-many fan-out, multi-hop, ABSR↔MSSR direct) round-trip.
    let r1_target = ctx.absr_solid_map.contains_key(&rep_1)
        || ctx.mssr_shells_map.contains_key(&rep_1)
        || ctx.wireframe_data_map.contains_key(&rep_1);
    let r2_target = ctx.absr_solid_map.contains_key(&rep_2)
        || ctx.mssr_shells_map.contains_key(&rep_2)
        || ctx.wireframe_data_map.contains_key(&rep_2);
    match (r1_target, r2_target) {
        (true, false) => {
            ctx.srr_equiv_map.insert(rep_2, rep_1);
        }
        (false, true) => {
            ctx.srr_equiv_map.insert(rep_1, rep_2);
        }
        _ => {}
    }

    let (Some(r1_id), Some(r2_id)) = (
        ctx.id_cache.get::<crate::ir::id::RepresentationId>(rep_1),
        ctx.id_cache.get::<crate::ir::id::RepresentationId>(rep_2),
    ) else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "SHAPE_REPRESENTATION_RELATIONSHIP #{entity_id} references unmodelled representation(s) (#{rep_1} or #{rep_2})"
            ),
        });
        return;
    };
    ctx.representation_relationships.push(
        RepresentationRelationship::ShapeRepresentationRelationship(
            ShapeRepresentationRelationshipIr {
                name: early.name,
                // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
                description: early.description.unwrap_or_default(),
                rep_1: r1_id,
                rep_2: r2_id,
            },
        ),
    );
}

/// Lower one bare `REPRESENTATION_CONTEXT` (unitless — no dimension count).
pub(crate) fn lower_representation_context(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyRepresentationContext,
) {
    let id = ctx.unitless_contexts.push(UnitlessContext {
        identifier: early.context_identifier,
        context_type: early.context_type,
        coordinate_space_dimension: None,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `CHARACTERIZED_ITEM_WITHIN_REPRESENTATION` (unresolved item /
/// rep = silent drop, legacy leniency). Registers in `id_cache` so
/// `PROPERTY_DEFINITION.definition` can resolve a CIWR target.
pub(crate) fn lower_characterized_item_within_representation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCharacterizedItemWithinRepresentation,
) {
    let Some(item) = crate::entities::visualization::styled_item::resolve_representation_item_ref(
        ctx, early.item,
    ) else {
        return;
    };
    let Some(rep) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(early.rep)
    else {
        return;
    };
    let co_id =
        ctx.characterized_objects
            .push(CharacterizedObject::CharacterizedItemWithinRepresentation(
                CharacterizedItemWithinRepresentation {
                    inherited: CharacterizedObjectData {
                        name: early.name,
                        description: early.description,
                    },
                    item,
                    rep,
                },
            ));
    ctx.id_cache.insert(entity_id, co_id);
}

/// L1 LOGICAL → the L2 `bool` the legacy `read_bool` produced. `.U.` drops
/// the entity (the legacy read errored there; no corpus file carries one).
fn logical_to_bool(l: crate::ir::geometry::Logical) -> Option<bool> {
    match l {
        crate::ir::geometry::Logical::True => Some(true),
        crate::ir::geometry::Logical::False => Some(false),
        crate::ir::geometry::Logical::Unknown => None,
    }
}

/// Lower one `REAL_REPRESENTATION_ITEM` (no `id_cache` registration — the
/// arena's only consumer is the writer's emit loop).
pub(crate) fn lower_real_representation_item(
    ctx: &mut ReaderContext,
    early: EarlyRealRepresentationItem,
) {
    ctx.numeric_representation_items
        .push(NumericRepresentationItem::Real(RealRepresentationItem {
            name: early.name,
            the_value: early.the_value,
        }));
}

/// Lower one `DATUM_TARGET` (unresolved `of_shape` product = silent drop).
pub(crate) fn lower_datum_target(ctx: &mut ReaderContext, entity_id: u64, early: EarlyDatumTarget) {
    let Some(product_definitional) = logical_to_bool(early.product_definitional) else {
        return;
    };
    let Some(target) = ctx.product_of_pds(early.of_shape) else {
        return;
    };
    let id = ctx.datum_targets.push(DatumTarget {
        name: early.name,
        // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
        description: early.description.unwrap_or_default(),
        target,
        product_definitional,
        target_id: early.target_id,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `TOLERANCE_ZONE` (unresolved `of_shape` product / form = silent
/// drop; unresolved `defining_tolerance` members skip individually).
pub(crate) fn lower_tolerance_zone(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyToleranceZone,
) {
    let Some(product_definitional) = logical_to_bool(early.product_definitional) else {
        return;
    };
    let Some(target) = ctx.product_of_pds(early.of_shape) else {
        return;
    };
    let Some(form) = ctx
        .id_cache
        .get::<crate::ir::ToleranceZoneFormId>(early.form)
    else {
        return;
    };
    let mut defining_tolerance = Vec::with_capacity(early.defining_tolerance.len());
    for r in early.defining_tolerance {
        if let Some(gtr) = crate::entities::pmi::resolve_geometric_tolerance_ref(ctx, r) {
            defining_tolerance.push(gtr);
        }
    }
    let id = ctx.tolerance_zones.push(ToleranceZone {
        name: early.name,
        // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
        description: early.description.unwrap_or_default(),
        target,
        product_definitional,
        defining_tolerance,
        form,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Shared lowering for the `SHAPE_ASPECT_RELATIONSHIP` family (base + the
/// `Associativity` / `DerivingRelationship` / `FeatureForDatumTarget` kinds).
/// Both endpoints resolve through `resolve_shape_aspect_ref`; either unresolved
/// drops the relationship (symmetric on re-read). No `id_cache` registration —
/// the arena's only consumer is the writer emit loop.
pub(crate) fn lower_shape_aspect_relationship(
    ctx: &mut ReaderContext,
    name: String,
    description: Option<String>,
    relating_ref: u64,
    related_ref: u64,
    kind: ShapeAspectRelationshipKind,
) {
    use crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref;
    let Some(relating_shape_aspect) = resolve_shape_aspect_ref(ctx, relating_ref) else {
        return;
    };
    let Some(related_shape_aspect) = resolve_shape_aspect_ref(ctx, related_ref) else {
        return;
    };
    ctx.shape_aspect_relationships
        .push(ShapeAspectRelationship {
            name,
            // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
            description: description.unwrap_or_default(),
            relating_shape_aspect,
            related_shape_aspect,
            kind,
        });
}

/// Shared `SHAPE_ASPECT`-subtype lowering prelude: bool conversion (`U`
/// drops) + the typed `of_shape` probe + the `$`→"" description collapse.
fn lower_sa_subtype_common(
    ctx: &ReaderContext,
    of_shape: u64,
    product_definitional: crate::ir::geometry::Logical,
    description: Option<String>,
) -> Option<(crate::ir::ProductId, bool, String)> {
    let pd = logical_to_bool(product_definitional)?;
    let target = ctx.product_of_pds(of_shape)?;
    Some((target, pd, description.unwrap_or_default()))
}

/// Lower one plain `SHAPE_ASPECT`. Same 4-attr shape as the subtypes, but with
/// the extra non-standard `of_shape = PRODUCT_DEFINITION` fallback (C3D), so the
/// dual-path resolve is written inline (it needs `&mut` for `ns_push`) rather
/// than via `lower_sa_subtype_common`. L1 keeps `of_shape` a plain ref; whether
/// the target is a standard PDS or the non-standard PD is decided here.
pub(crate) fn lower_shape_aspect(ctx: &mut ReaderContext, entity_id: u64, early: EarlyShapeAspect) {
    let Some(product_definitional) = logical_to_bool(early.product_definitional) else {
        return;
    };
    // SHAPE_ASPECT.of_shape → PRODUCT_DEFINITION_SHAPE → ProductId (typed
    // one-probe). NsCase::ShapeAspectOfShapePd (C3D): of_shape references a
    // PRODUCT_DEFINITION directly → resolve via the product; writer re-emits
    // the standard PDS form.
    let target = if let Some(pid) = ctx.product_of_pds(early.of_shape) {
        pid
    } else if ctx
        .id_cache
        .get::<crate::ir::ProductDefinitionId>(early.of_shape)
        .is_some()
    {
        ctx.ns_push(
            crate::reader::NsCase::ShapeAspectOfShapePd,
            "SHAPE_ASPECT.of_shape".into(),
            1,
            "PRODUCT_DEFINITION_SHAPE (of_shape was a PRODUCT_DEFINITION)".into(),
        );
        let Some(pid) = ctx.product_of_pdef(early.of_shape) else {
            return;
        };
        pid
    } else {
        return; // unresolved (genuinely non-product target)
    };
    let id = ctx.shape_aspects.push(ShapeAspect {
        name: early.name,
        // Legacy read_string_or_unset collapsed `$` to "" (L2 String).
        description: early.description.unwrap_or_default(),
        target,
        product_definitional,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `COMPOSITE_GROUP_SHAPE_ASPECT` (shared 4-attr subtype shape).
pub(crate) fn lower_composite_group_shape_aspect(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCompositeGroupShapeAspect,
) {
    let Some((target, product_definitional, description)) = lower_sa_subtype_common(
        ctx,
        early.of_shape,
        early.product_definitional,
        early.description,
    ) else {
        return;
    };
    let id = ctx
        .composite_group_shape_aspects
        .push(CompositeGroupShapeAspect {
            name: early.name,
            description,
            target,
            product_definitional,
            kind: CompositeShapeAspectKind::Group,
            datum_feature: false,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `COMPOSITE_SHAPE_ASPECT` (same arena, `Composite` kind).
pub(crate) fn lower_composite_shape_aspect(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCompositeShapeAspect,
) {
    let Some((target, product_definitional, description)) = lower_sa_subtype_common(
        ctx,
        early.of_shape,
        early.product_definitional,
        early.description,
    ) else {
        return;
    };
    let id = ctx
        .composite_group_shape_aspects
        .push(CompositeGroupShapeAspect {
            name: early.name,
            description,
            target,
            product_definitional,
            kind: CompositeShapeAspectKind::Composite,
            datum_feature: false,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `CENTRE_OF_SYMMETRY`.
pub(crate) fn lower_centre_of_symmetry(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCentreOfSymmetry,
) {
    let Some((target, product_definitional, description)) = lower_sa_subtype_common(
        ctx,
        early.of_shape,
        early.product_definitional,
        early.description,
    ) else {
        return;
    };
    let id = ctx.centre_of_symmetries.push(CentreOfSymmetry {
        name: early.name,
        description,
        target,
        product_definitional,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `ALL_AROUND_SHAPE_ASPECT`.
pub(crate) fn lower_all_around_shape_aspect(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAllAroundShapeAspect,
) {
    let Some((target, product_definitional, description)) = lower_sa_subtype_common(
        ctx,
        early.of_shape,
        early.product_definitional,
        early.description,
    ) else {
        return;
    };
    let id = ctx.all_around_shape_aspects.push(AllAroundShapeAspect {
        name: early.name,
        description,
        target,
        product_definitional,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `PLACED_DATUM_TARGET_FEATURE` (same shared subtype prelude).
pub(crate) fn lower_placed_datum_target_feature(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPlacedDatumTargetFeature,
) {
    let Some((target, product_definitional, description)) = lower_sa_subtype_common(
        ctx,
        early.of_shape,
        early.product_definitional,
        early.description,
    ) else {
        return;
    };
    let id = ctx
        .placed_datum_target_features
        .push(PlacedDatumTargetFeature {
            name: early.name,
            description,
            target,
            product_definitional,
            target_id: early.target_id,
        });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DATUM_SYSTEM` (unresolved constituents skip individually).
pub(crate) fn lower_datum_system(ctx: &mut ReaderContext, entity_id: u64, early: EarlyDatumSystem) {
    let Some((target, product_definitional, description)) = lower_sa_subtype_common(
        ctx,
        early.of_shape,
        early.product_definitional,
        early.description,
    ) else {
        return;
    };
    let mut constituents = Vec::with_capacity(early.constituents.len());
    for r in early.constituents {
        if let Some(id) = ctx.id_cache.get::<crate::ir::GeneralDatumReferenceId>(r) {
            constituents.push(id);
        }
    }
    let id = ctx.datum_systems.push(DatumSystem {
        name: early.name,
        description,
        target,
        product_definitional,
        constituents,
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DESCRIPTIVE_REPRESENTATION_ITEM` (dispatch-side value map —
/// consumed transiently by the property handlers).
pub(crate) fn lower_descriptive_representation_item(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDescriptiveRepresentationItem,
) {
    ctx.descriptive_item_map.insert(
        entity_id,
        crate::ir::shape_rep::DescriptiveItem {
            name: early.name,
            description: early.description,
        },
    );
}

/// Lower one `QUALIFIED_REPRESENTATION_ITEM`. Members resolved by the
/// `StepSelect` derive; unmodelled members drop silently (corpus 0).
pub(crate) fn lower_qualified_representation_item(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyQualifiedRepresentationItem,
) {
    let mut qualifiers = Vec::with_capacity(early.qualifiers.len());
    for r in early.qualifiers {
        if let Some(q) = QualifierRef::resolve_select(ctx, r) {
            qualifiers.push(q);
        }
    }
    let id = ctx
        .representation_items
        .push(RepresentationItem::QualifiedRepresentationItem(
            QualifiedRepresentationItem {
                name: early.name,
                qualifiers,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `MODEL_GEOMETRIC_VIEW` (unresolved camera / rep = silent drop).
/// Registers in `id_cache` so `PROPERTY_DEFINITION.definition` can resolve
/// an MGV target (mirrors CIWR).
pub(crate) fn lower_model_geometric_view(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyModelGeometricView,
) {
    let Some(item) = ctx.id_cache.get::<crate::ir::id::CameraModelId>(early.item) else {
        return;
    };
    let Some(rep) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(early.rep)
    else {
        return;
    };
    let co_id = ctx
        .characterized_objects
        .push(CharacterizedObject::ModelGeometricView(
            ModelGeometricView {
                inherited: CharacterizedObjectData {
                    name: early.name,
                    description: early.description,
                },
                item,
                rep,
            },
        ));
    ctx.id_cache.insert(entity_id, co_id);
}

/// Lower one `(GEOMETRIC_REPRESENTATION_CONTEXT PARAMETRIC_REPRESENTATION_CONTEXT
/// REPRESENTATION_CONTEXT)` complex into the `unitless_contexts` arena. The GRC
/// `coordinate_space_dimension` is `Some(..)` here (the writer's
/// `emit_unitless_contexts` pass keys the parametric-vs-plain form on this).
pub(crate) fn lower_parametric_representation_context(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyParametricRepresentationContext,
) {
    let id = ctx.unitless_contexts.push(UnitlessContext {
        identifier: early.context_identifier.clone(),
        context_type: early.context_type.clone(),
        coordinate_space_dimension: Some(early.coordinate_space_dimension),
    });
    ctx.id_cache.insert(entity_id, id);
}

/// Bridge the synth-generated 43-typed `EarlyMeasureValue` (L1) → the generic
/// L2 `MeasureValue` (`type_name` preserved). Every numeric member is real
/// (NUMBER members are real by type — see Phase 3j); `descriptive_measure` is
/// text. The `early_measure_value_round_trips` test exercises all 43 arms.
pub(crate) fn measure_value_to_l2(m: EarlyMeasureValue) -> MeasureValue {
    use EarlyMeasureValue as E;
    macro_rules! r {
        ($v:expr, $t:literal) => {
            MeasureValue::Real {
                type_name: $t.into(),
                value: $v,
            }
        };
    }
    match m {
        E::AbsorbedDoseMeasure(v) => r!(v, "ABSORBED_DOSE_MEASURE"),
        E::AccelerationMeasure(v) => r!(v, "ACCELERATION_MEASURE"),
        E::AmountOfSubstanceMeasure(v) => r!(v, "AMOUNT_OF_SUBSTANCE_MEASURE"),
        E::AreaMeasure(v) => r!(v, "AREA_MEASURE"),
        E::CapacitanceMeasure(v) => r!(v, "CAPACITANCE_MEASURE"),
        E::CelsiusTemperatureMeasure(v) => r!(v, "CELSIUS_TEMPERATURE_MEASURE"),
        E::ConductanceMeasure(v) => r!(v, "CONDUCTANCE_MEASURE"),
        E::ContextDependentMeasure(v) => r!(v, "CONTEXT_DEPENDENT_MEASURE"),
        E::CountMeasure(v) => r!(v, "COUNT_MEASURE"),
        E::DescriptiveMeasure(s) => MeasureValue::Text {
            type_name: "DESCRIPTIVE_MEASURE".into(),
            value: s,
        },
        E::DoseEquivalentMeasure(v) => r!(v, "DOSE_EQUIVALENT_MEASURE"),
        E::ElectricChargeMeasure(v) => r!(v, "ELECTRIC_CHARGE_MEASURE"),
        E::ElectricCurrentMeasure(v) => r!(v, "ELECTRIC_CURRENT_MEASURE"),
        E::ElectricPotentialMeasure(v) => r!(v, "ELECTRIC_POTENTIAL_MEASURE"),
        E::EnergyMeasure(v) => r!(v, "ENERGY_MEASURE"),
        E::ForceMeasure(v) => r!(v, "FORCE_MEASURE"),
        E::FrequencyMeasure(v) => r!(v, "FREQUENCY_MEASURE"),
        E::IlluminanceMeasure(v) => r!(v, "ILLUMINANCE_MEASURE"),
        E::InductanceMeasure(v) => r!(v, "INDUCTANCE_MEASURE"),
        E::LengthMeasure(v) => r!(v, "LENGTH_MEASURE"),
        E::LuminousFluxMeasure(v) => r!(v, "LUMINOUS_FLUX_MEASURE"),
        E::LuminousIntensityMeasure(v) => r!(v, "LUMINOUS_INTENSITY_MEASURE"),
        E::MagneticFluxDensityMeasure(v) => r!(v, "MAGNETIC_FLUX_DENSITY_MEASURE"),
        E::MagneticFluxMeasure(v) => r!(v, "MAGNETIC_FLUX_MEASURE"),
        E::MassMeasure(v) => r!(v, "MASS_MEASURE"),
        E::NonNegativeLengthMeasure(v) => r!(v, "NON_NEGATIVE_LENGTH_MEASURE"),
        E::NumericMeasure(v) => r!(v, "NUMERIC_MEASURE"),
        E::ParameterValue(v) => r!(v, "PARAMETER_VALUE"),
        E::PlaneAngleMeasure(v) => r!(v, "PLANE_ANGLE_MEASURE"),
        E::PositiveLengthMeasure(v) => r!(v, "POSITIVE_LENGTH_MEASURE"),
        E::PositivePlaneAngleMeasure(v) => r!(v, "POSITIVE_PLANE_ANGLE_MEASURE"),
        E::PositiveRatioMeasure(v) => r!(v, "POSITIVE_RATIO_MEASURE"),
        E::PowerMeasure(v) => r!(v, "POWER_MEASURE"),
        E::PressureMeasure(v) => r!(v, "PRESSURE_MEASURE"),
        E::RadioactivityMeasure(v) => r!(v, "RADIOACTIVITY_MEASURE"),
        E::RatioMeasure(v) => r!(v, "RATIO_MEASURE"),
        E::ResistanceMeasure(v) => r!(v, "RESISTANCE_MEASURE"),
        E::SolidAngleMeasure(v) => r!(v, "SOLID_ANGLE_MEASURE"),
        E::ThermodynamicTemperatureMeasure(v) => r!(v, "THERMODYNAMIC_TEMPERATURE_MEASURE"),
        E::TimeMeasure(v) => r!(v, "TIME_MEASURE"),
        E::VelocityMeasure(v) => r!(v, "VELOCITY_MEASURE"),
        E::VolumeMeasure(v) => r!(v, "VOLUME_MEASURE"),
    }
}

/// Lower one `VALUE_REPRESENTATION_ITEM` (synth `measure_value` SELECT). The
/// generated bind already dropped non-standard measure tags (strict).
pub(crate) fn lower_value_representation_item(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyValueRepresentationItem,
) {
    let value_component = measure_value_to_l2(early.value_component.clone());
    let id = ctx
        .representation_items
        .push(RepresentationItem::ValueRepresentationItem(
            ValueRepresentationItem {
                name: early.name.clone(),
                value_component,
            },
        ));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `TESSELLATED_SHAPE_REPRESENTATION`. The EXPRESS WHERE rule requires
/// `context_of_items` to be a `GLOBAL_UNIT_ASSIGNED_CONTEXT`; only the `Unitful`
/// variant is accepted (other contexts drop the carrier, symmetric on re-read).
/// Items unresolved by `resolve_tessellated_item_ref` are filtered; an empty
/// resolved set drops the carrier.
pub(crate) fn lower_tessellated_shape_representation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyTessellatedShapeRepresentation,
) {
    let context = ctx.resolve_repr_context(early.context_of_items);
    let Some(RepresentationContextRef::Unitful(ctx_id)) = context else {
        return;
    };
    ctx.repr_context_map.insert(entity_id, ctx_id);

    let items: Vec<_> = early
        .items
        .iter()
        .filter_map(|&r| resolve_tessellated_item_ref(ctx, r))
        .collect();
    if items.is_empty() {
        return;
    }

    let repr_id = ctx
        .representations
        .push(Representation::TessellatedShapeRepresentation(
            TessellatedShapeRepresentation {
                name: early.name,
                items,
                context,
            },
        ));
    ctx.id_cache.insert(entity_id, repr_id);
}

/// Lower one `CONSTRUCTIVE_GEOMETRY_REPRESENTATION`. Items are narrowed by the
/// shared representation-item resolver (unresolved skipped; empty set drops the
/// carrier). `context_of_items` is schema-required; an unresolvable context
/// (neither Unitful nor Unitless) drops the carrier (schema-strict — no
/// non-standard `$` round-trip in the corpus, see plan 0-FAIL proof).
pub(crate) fn lower_constructive_geometry_representation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyConstructiveGeometryRepresentation,
) {
    let Some(context) = ctx.resolve_repr_context(early.context_of_items) else {
        return;
    };
    if let RepresentationContextRef::Unitful(ctx_id) = context {
        ctx.repr_context_map.insert(entity_id, ctx_id);
    }
    let items: Vec<_> = early
        .items
        .iter()
        .filter_map(|&r| {
            crate::entities::visualization::styled_item::resolve_representation_item_ref(ctx, r)
        })
        .collect();
    if items.is_empty() {
        return;
    }
    let repr_id = ctx
        .representations
        .push(Representation::ConstructiveGeometry(
            ConstructiveGeometryRepr {
                name: early.name,
                items,
                context: Some(context),
            },
        ));
    ctx.id_cache.insert(entity_id, repr_id);
}

/// Lower one `MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION`. `items`
/// are narrowed to `STYLED_ITEM` (unresolved skipped; an empty set is kept,
/// unlike CGR). `context_of_items` is schema-required; an unresolvable context
/// drops the carrier (schema-strict — None-context is corpus-absent, see plan).
/// Dual-write: the `visualization.mdgprs` copy + the unified `representations`
/// arena (the latter drives emit; `id_cache` maps to the `RepresentationId`).
pub(crate) fn lower_mechanical_design_geometric_presentation_representation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyMechanicalDesignGeometricPresentationRepresentation,
) {
    let Some(context) = ctx.resolve_repr_context(early.context_of_items) else {
        return;
    };
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Some(id) = ctx.id_cache.get::<crate::ir::id::StyledItemId>(r) {
            items.push(id);
        }
    }
    let mdgpr = Mdgpr {
        name: early.name,
        items,
        context: Some(context),
    };
    ctx.visualization
        .get_or_insert_with(VisualizationPool::default)
        .mdgprs
        .push(mdgpr.clone());
    let repr_id = ctx.representations.push(Representation::Mdgpr(mdgpr));
    ctx.id_cache.insert(entity_id, repr_id);
}

/// Lower one `SHAPE_DIMENSION_REPRESENTATION`. `context_of_items` is schema-
/// required; an unresolvable context drops the carrier (schema-strict). Item
/// resolution is **deferred** (the complex `MEASURE_REPRESENTATION_ITEM` arena is
/// not yet populated): push empty `items` and stash the raw refs in
/// `sdr_raw_items`, which `resolve_deferred_sdr_items` rebuilds post-read.
pub(crate) fn lower_shape_dimension_representation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyShapeDimensionRepresentation,
) {
    let Some(context) = ctx.resolve_repr_context(early.context_of_items) else {
        return;
    };
    if let RepresentationContextRef::Unitful(ctx_id) = context {
        ctx.repr_context_map.insert(entity_id, ctx_id);
    }
    let repr_id = ctx
        .representations
        .push(Representation::ShapeDimensionRepresentation(
            ShapeDimensionRepresentation {
                name: early.name,
                context: Some(context),
                items: Vec::new(),
            },
        ));
    ctx.id_cache.insert(entity_id, repr_id);
    ctx.sdr_raw_items.insert(repr_id, early.items);
}

/// Lower one `SHAPE_REPRESENTATION_WITH_PARAMETERS`. `context_of_items` required
/// → unresolvable drops the carrier. `items` narrow to the 4-way `SrwpItem`
/// SELECT (direction / placement / descriptive / measure-repr-item); unresolved
/// members skip, and an empty resolved set drops the carrier.
pub(crate) fn lower_shape_representation_with_parameters(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyShapeRepresentationWithParameters,
) {
    let Some(context) = ctx.resolve_repr_context(early.context_of_items) else {
        return;
    };
    if let RepresentationContextRef::Unitful(ctx_id) = context {
        ctx.repr_context_map.insert(entity_id, ctx_id);
    }
    let mut items = Vec::with_capacity(early.items.len());
    for r in early.items {
        if let Some(did) = ctx.id_cache.get::<crate::ir::id::DirectionId>(r) {
            items.push(SrwpItem::Direction(did));
        } else if let Some(&pid) = ctx.placement_map.get(&r) {
            items.push(SrwpItem::Placement(pid));
        } else if let Some(d) = ctx.descriptive_item_map.get(&r).cloned() {
            items.push(SrwpItem::Descriptive(d));
        } else if let Some(id) = ctx.id_cache.get::<crate::ir::id::RepresentationItemId>(r) {
            // MEASURE_REPRESENTATION_ITEM lives in the representation_item arena.
            // Guard on the variant: repr_item_id_map also holds QRI / VRI.
            if matches!(
                ctx.representation_items[id],
                RepresentationItem::MeasureRepresentationItem(_)
            ) {
                items.push(SrwpItem::MeasureItem(id));
            }
        }
    }
    if items.is_empty() {
        return;
    }
    let repr_id = ctx
        .representations
        .push(Representation::ShapeRepresentationWithParameters(
            ShapeRepresentationWithParameters {
                name: early.name,
                items,
                context: Some(context),
            },
        ));
    ctx.id_cache.insert(entity_id, repr_id);
}

/// Lower one `COMPOUND_REPRESENTATION_ITEM`. The synth `item_element` SELECT
/// carries the Set/List kind + raw child refs; each child resolves to a
/// `Descriptive` (via `descriptive_item_map`) or a generic representation
/// `Item` (unresolved skipped). An empty resolved set drops the carrier.
/// Orphan round-trip — no `id_cache` entry.
pub(crate) fn lower_compound_representation_item(
    ctx: &mut ReaderContext,
    _entity_id: u64,
    early: EarlyCompoundRepresentationItem,
) {
    let (kind, raw) = match early.item_element {
        EarlyCompoundItemDefinition::SetRepresentationItem(refs) => (CompoundItemKind::Set, refs),
        EarlyCompoundItemDefinition::ListRepresentationItem(refs) => (CompoundItemKind::List, refs),
    };
    let mut items = Vec::with_capacity(raw.len());
    for r in raw {
        if let Some(d) = ctx.descriptive_item_map.get(&r).cloned() {
            items.push(CompoundItem::Descriptive(d));
        } else if let Some(item_ref) =
            crate::entities::visualization::styled_item::resolve_representation_item_ref(ctx, r)
        {
            items.push(CompoundItem::Item(item_ref));
        }
    }
    if items.is_empty() {
        return;
    }
    ctx.compound_representation_items
        .push(CompoundRepresentationItem {
            name: early.name,
            item_element: CompoundItemElement { kind, items },
        });
}
