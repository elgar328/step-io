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
    EarlyAdvancedBrepShapeRepresentation, EarlyAllAroundShapeAspect, EarlyCameraImage,
    EarlyCameraImage3dWithScale, EarlyCentreOfSymmetry, EarlyCharacterizedItemWithinRepresentation,
    EarlyCharacterizedObjectComplex, EarlyCompositeGroupShapeAspect, EarlyCompositeShapeAspect,
    EarlyCompoundItemDefinition, EarlyCompoundRepresentationItem,
    EarlyConstructiveGeometryRepresentation, EarlyConstructiveGeometryRepresentationRelationship,
    EarlyDatumSystem, EarlyDatumTarget, EarlyDefaultModelGeometricView,
    EarlyDescriptiveRepresentationItem, EarlyDraughtingModel, EarlyGeometricItemSpecificUsage,
    EarlyGeometricallyBoundedSurfaceShapeRepresentation,
    EarlyGeometricallyBoundedWireframeShapeRepresentation, EarlyGlobalUnitAssignedContext,
    EarlyIntegerRepresentationItem, EarlyItemDefinedTransformation,
    EarlyItemIdentifiedRepresentationUsage, EarlyItemIdentifiedRepresentationUsageSelect,
    EarlyManifoldSurfaceShapeRepresentation, EarlyMappedItem, EarlyMeasureValue,
    EarlyMechanicalDesignAndDraughtingRelationship,
    EarlyMechanicalDesignGeometricPresentationRepresentation, EarlyModelGeometricView,
    EarlyParametricRepresentationContext, EarlyPlacedDatumTargetFeature,
    EarlyQualifiedRepresentationItem, EarlyRealRepresentationItem, EarlyRepresentationContext,
    EarlyRepresentationMap, EarlyRepresentationRelationship, EarlyShapeAspect,
    EarlyShapeDimensionRepresentation, EarlyShapeRepresentation,
    EarlyShapeRepresentationRelationship, EarlyShapeRepresentationWithParameters,
    EarlyTessellatedShapeRepresentation, EarlyToleranceZone, EarlyValueRepresentationItem,
};
use crate::entities::tessellation::resolve_tessellated_item_ref;
use crate::ir::assembly::Transform3d;
use crate::ir::error::ConvertError;
use crate::ir::representation_item::{
    MeasureValue, QualifiedRepresentationItem, QualifierRef, RepresentationItem,
    RepresentationItemRef, ValueRepresentationItem,
};
use crate::ir::shape_rep::{
    AdvancedBrepRepr, AllAroundShapeAspect, CameraImage, CentreOfSymmetry,
    CharacterizedItemWithinRepresentation, CharacterizedObject, CharacterizedObjectData,
    CompositeGroupShapeAspect, CompositeShapeAspectKind, CompoundItem, CompoundItemElement,
    CompoundItemKind, CompoundRepresentationItem, ConstructiveGeometryRepr,
    ConstructiveGeometryRepresentationRelationship, DatumSystem, DatumTarget,
    DefaultModelGeometricView, DraughtingModel, DraughtingModelForm, GeometricItemSpecificUsage,
    IiruDefinition, IiruIdentifiedItem, IntegerRepresentationItem,
    ItemIdentifiedRepresentationUsage, ManifoldSurfaceRepr, MappedItem, MappedItemData,
    MappedRepresentationRef, Mdgpr, MechanicalDesignAndDraughtingRelationship, ModelGeometricView,
    NumericRepresentationItem, PlacedDatumTargetFeature, PlainRepr, RealRepresentationItem,
    Representation, RepresentationContextRef, RepresentationMap, RepresentationMapData,
    RepresentationRelationship, RepresentationRelationshipData, ShapeAspect,
    ShapeAspectRelationship, ShapeAspectRelationshipKind, ShapeDimensionRepresentation,
    ShapeRepresentationRelationshipIr, ShapeRepresentationWithParameters, SrwpItem,
    TessellatedShapeRepresentation, ToleranceZone, UnitContext, UnitContextForm, UnitlessContext,
    WireframeRepr,
};
use crate::ir::visualization::VisualizationPool;
use crate::ir::{WireframeContent, WireframeReprKind};
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

/// Lower one `INTEGER_REPRESENTATION_ITEM` into the shared numeric arena.
pub(crate) fn lower_integer_representation_item(
    ctx: &mut ReaderContext,
    early: EarlyIntegerRepresentationItem,
) {
    ctx.numeric_representation_items
        .push(NumericRepresentationItem::Integer(
            IntegerRepresentationItem {
                name: early.name,
                the_value: early.the_value,
            },
        ));
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

/// Lower one plain `DRAUGHTING_MODEL` (`Form::Simple`). `representation` subtype
/// — resolves items via the generic ref helper (unresolved skipped); a model
/// that listed items but resolved none is dropped with a warning, an empty `()`
/// SET is kept. `context_of_items` is schema-required; an unresolvable context
/// drops the carrier (schema-strict drop-on-None; the complex-MI forms are
/// lowered by `lower_characterized_object_complex`).
pub(crate) fn lower_draughting_model(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDraughtingModel,
) {
    let Some(context) = ctx.resolve_repr_context(early.context_of_items) else {
        return;
    };
    if let RepresentationContextRef::Unitful(ctx_id) = context {
        ctx.repr_context_map.insert(entity_id, ctx_id);
    }
    let had_item_refs = !early.items.is_empty();
    let items: Vec<_> = early
        .items
        .iter()
        .filter_map(|&r| {
            crate::entities::visualization::styled_item::resolve_representation_item_ref(ctx, r)
        })
        .collect();
    if items.is_empty() && had_item_refs {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: String::from(
                "DRAUGHTING_MODEL dropped: no items resolved to a modelled representation_item",
            ),
        });
        return;
    }
    let repr_id = ctx
        .representations
        .push(Representation::DraughtingModel(DraughtingModel {
            name: early.name,
            items,
            context: Some(context),
            form: DraughtingModelForm::Simple,
        }));
    ctx.id_cache.insert(entity_id, repr_id);
}

/// Lower one complex-MI `DRAUGHTING_MODEL` form (cases A/B/C). bind already
/// exact-matched the part-set into the variant, so the form follows directly.
/// A/B carry a `CHARACTERIZED_OBJECT` facet → a `CharacterizedObject::Itself`
/// carrier arena entry (name reused from the REPRESENTATION part; `(*,*)` so no
/// CO-local data) + a dual `id_cache` insert so a `PROPERTY_DEFINITION` targeting
/// the CO facet resolves. Items resolve via the generic ref helper; an empty
/// resolved set drops the DM (the CO carrier, already pushed, remains).
/// `context_of_items` schema-strict (drop-on-None).
pub(crate) fn lower_characterized_object_complex(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCharacterizedObjectComplex,
) {
    let (name, item_refs, ctx_raw, has_co, has_st) = match early {
        EarlyCharacterizedObjectComplex::Characterized(c) => {
            (c.name, c.items, c.context_of_items, true, false)
        }
        EarlyCharacterizedObjectComplex::CharacterizedShapeTessellated(c) => {
            (c.name, c.items, c.context_of_items, true, true)
        }
        EarlyCharacterizedObjectComplex::ShapeTessellated(c) => {
            (c.name, c.items, c.context_of_items, false, true)
        }
    };
    // The CHARACTERIZED_OBJECT facet (cases A/B) gets a carrier arena entry so a
    // PROPERTY_DEFINITION / CIWR targeting it resolves; case C has none.
    let co_id = has_co.then(|| {
        ctx.characterized_objects
            .push(CharacterizedObject::Itself(CharacterizedObjectData {
                name: name.clone(),
                description: None,
            }))
    });
    let Some(context) = ctx.resolve_repr_context(ctx_raw) else {
        return;
    };
    if let RepresentationContextRef::Unitful(ctx_id) = context {
        ctx.repr_context_map.insert(entity_id, ctx_id);
    }
    let items: Vec<_> = item_refs
        .iter()
        .filter_map(|&r| {
            crate::entities::visualization::styled_item::resolve_representation_item_ref(ctx, r)
        })
        .collect();
    if items.is_empty() {
        return;
    }
    let form = match (co_id, has_st) {
        (Some(id), true) => DraughtingModelForm::CharacterizedShapeTessellated(id),
        (Some(id), false) => DraughtingModelForm::Characterized(id),
        (None, true) => DraughtingModelForm::ShapeTessellated,
        (None, false) => DraughtingModelForm::Simple,
    };
    let repr_id = ctx
        .representations
        .push(Representation::DraughtingModel(DraughtingModel {
            name,
            items,
            context: Some(context),
            form,
        }));
    ctx.id_cache.insert(entity_id, repr_id);
    if let Some(cid) = co_id {
        ctx.id_cache.insert(entity_id, cid);
    }
}

/// Lower one plain `SHAPE_REPRESENTATION`. Captures the first
/// `AXIS2_PLACEMENT_3D` from `items` as the coordinate frame (for the SDR
/// indirection pass) and dual-writes into the unified `representations` arena.
/// `context_of_items` is schema-required; an unresolvable context drops the
/// carrier (schema-strict — None-context is corpus-absent, see plan 0-FAIL
/// proof).
pub(crate) fn lower_shape_representation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyShapeRepresentation,
) {
    let Some(context) = ctx.resolve_repr_context(early.context_of_items) else {
        return;
    };
    if let RepresentationContextRef::Unitful(ctx_id) = context {
        ctx.repr_context_map.insert(entity_id, ctx_id);
    }
    let frame = early
        .items
        .iter()
        .find_map(|r| ctx.placement_map.get(r).copied());
    if let Some(placement_id) = frame {
        ctx.plain_sr_frame_map.insert(entity_id, placement_id);
    }
    let repr_id = ctx.representations.push(Representation::Plain(PlainRepr {
        name: early.name,
        context: Some(context),
        frame,
    }));
    ctx.id_cache.insert(entity_id, repr_id);
}

/// Lower one `ADVANCED_BREP_SHAPE_REPRESENTATION`. Resolves every `items` ref
/// into a typed `RepresentationItemRef` (solids + an axis frame, or `MAPPED_ITEM`s
/// for an assembly ABSR), preserving source order, and derives the legacy
/// `absr_solid_map` / `absr_ref_frame_map` side maps. An empty resolved set is
/// kept (with a warning) — unlike CGR, which drops. `context_of_items` is
/// schema-required; an unresolvable context drops the carrier (schema-strict —
/// None-context is corpus-absent, see plan 0-FAIL proof).
pub(crate) fn lower_advanced_brep_shape_representation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyAdvancedBrepShapeRepresentation,
) {
    let Some(context) = ctx.resolve_repr_context(early.context_of_items) else {
        return;
    };
    if let RepresentationContextRef::Unitful(ctx_id) = context {
        ctx.repr_context_map.insert(entity_id, ctx_id);
    }
    let resolved: Vec<RepresentationItemRef> = early
        .items
        .iter()
        .filter_map(|&r| {
            crate::entities::visualization::styled_item::resolve_representation_item_ref(ctx, r)
        })
        .collect();

    let solid_ids: Vec<_> = resolved
        .iter()
        .filter_map(|it| match it {
            RepresentationItemRef::Solid(id) => Some(*id),
            _ => None,
        })
        .collect();
    if let Some(placement_id) = resolved.iter().find_map(|it| match it {
        RepresentationItemRef::Placement3d(id) => Some(*id),
        _ => None,
    }) {
        ctx.absr_ref_frame_map.insert(entity_id, placement_id);
    }
    if !solid_ids.is_empty() {
        ctx.absr_solid_map.insert(entity_id, solid_ids);
    }
    if resolved.is_empty() {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: String::from("ADVANCED_BREP_SHAPE_REPRESENTATION with no resolvable items"),
        });
    }

    let repr_id = ctx
        .representations
        .push(Representation::AdvancedBrep(AdvancedBrepRepr {
            name: early.name,
            context: Some(context),
            items: resolved,
        }));
    ctx.id_cache.insert(entity_id, repr_id);
}

/// Lower one `MANIFOLD_SURFACE_SHAPE_REPRESENTATION`. `items` carry one or more
/// `SHELL_BASED_SURFACE_MODEL`s (shells flattened via `sbsm_shells_map`, GRI ids
/// kept in `sbsm_id_map` for the writer's arena routing) plus an optional
/// `AXIS2_PLACEMENT_3D` frame. Derives the legacy `mssr_ref_frame_map` /
/// `mssr_shells_map` side maps and dual-writes the unified arena. An empty
/// resolved set is kept (no warning, mirroring the legacy reader).
/// `context_of_items` is schema-required; an unresolvable context drops the
/// carrier (schema-strict — None-context is corpus-absent, see plan 0-FAIL).
pub(crate) fn lower_manifold_surface_shape_representation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyManifoldSurfaceShapeRepresentation,
) {
    let Some(context) = ctx.resolve_repr_context(early.context_of_items) else {
        return;
    };
    if let RepresentationContextRef::Unitful(ctx_id) = context {
        ctx.repr_context_map.insert(entity_id, ctx_id);
    }
    let ref_frame = early
        .items
        .iter()
        .find_map(|r| ctx.placement_map.get(r).copied());
    if let Some(placement_id) = ref_frame {
        ctx.mssr_ref_frame_map.insert(entity_id, placement_id);
    }
    let shells: Vec<crate::ir::ShellId> = early
        .items
        .iter()
        .filter_map(|r| ctx.sbsm_shells_map.get(r))
        .flat_map(|shells| shells.iter().copied())
        .collect();
    ctx.mssr_shells_map.insert(entity_id, shells.clone());
    let sbsm_ids: Vec<crate::ir::id::GeometricRepresentationItemId> = early
        .items
        .iter()
        .filter_map(|r| ctx.sbsm_id_map.get(r).copied())
        .collect();
    let repr_id = ctx
        .representations
        .push(Representation::ManifoldSurface(ManifoldSurfaceRepr {
            name: early.name,
            context: Some(context),
            ref_frame,
            shells,
            sbsm_ids,
        }));
    ctx.id_cache.insert(entity_id, repr_id);
}

/// Shared body for `GEOMETRICALLY_BOUNDED_{WIREFRAME,SURFACE}_SHAPE_
/// REPRESENTATION` (differ only by `repr_kind`). `items` carry a
/// `GEOMETRIC_(CURVE_)SET` (curves + points flattened via `curve_set_map`, with
/// a bare `CURVE` ref also accepted) plus an optional `AXIS2_PLACEMENT_3D`
/// frame; GRI ids are kept in `curve_set_id_map` for the writer's arena routing.
/// Derives the legacy `wireframe_ref_frame_map` / `wireframe_data_map` side maps
/// and dual-writes the unified arena. Empty set kept (no warning).
/// `context_of_items` schema-required; unresolvable context drops the carrier.
fn lower_wireframe_representation_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    name: String,
    items: &[u64],
    context_of_items: u64,
    repr_kind: WireframeReprKind,
) {
    let Some(context) = ctx.resolve_repr_context(context_of_items) else {
        return;
    };
    if let RepresentationContextRef::Unitful(ctx_id) = context {
        ctx.repr_context_map.insert(entity_id, ctx_id);
    }
    let ref_frame = items.iter().find_map(|r| ctx.placement_map.get(r).copied());
    if let Some(placement_id) = ref_frame {
        ctx.wireframe_ref_frame_map.insert(entity_id, placement_id);
    }
    let mut curves = Vec::new();
    let mut points = Vec::new();
    for r in items {
        if let Some((c, p)) = ctx.curve_set_map.get(r) {
            curves.extend_from_slice(c);
            points.extend_from_slice(p);
        } else if let Some(cid) = ctx.id_cache.get::<crate::ir::id::CurveId>(*r) {
            curves.push(cid);
        }
    }
    let wireframe = WireframeContent {
        curves,
        points,
        repr_kind,
    };
    ctx.wireframe_data_map.insert(entity_id, wireframe.clone());
    let gcs_ids: Vec<crate::ir::id::GeometricRepresentationItemId> = items
        .iter()
        .filter_map(|r| ctx.curve_set_id_map.get(r).copied())
        .collect();
    let repr_id = ctx
        .representations
        .push(Representation::Wireframe(WireframeRepr {
            name,
            context: Some(context),
            ref_frame,
            content: wireframe,
            gcs_ids,
        }));
    ctx.id_cache.insert(entity_id, repr_id);
}

/// Lower one `GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION`.
pub(crate) fn lower_geometrically_bounded_surface_shape_representation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyGeometricallyBoundedSurfaceShapeRepresentation,
) {
    lower_wireframe_representation_body(
        ctx,
        entity_id,
        early.name,
        &early.items,
        early.context_of_items,
        WireframeReprKind::Surface,
    );
}

/// Lower one `GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION`.
pub(crate) fn lower_geometrically_bounded_wireframe_shape_representation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyGeometricallyBoundedWireframeShapeRepresentation,
) {
    lower_wireframe_representation_body(
        ctx,
        entity_id,
        early.name,
        &early.items,
        early.context_of_items,
        WireframeReprKind::Wireframe,
    );
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

/// Resolve the IIRU `definition` SELECT (all-entity → bare ref) to the typed
/// [`IiruDefinition`] via the 5 modelled members; unmodelled members → `None`.
fn resolve_iiru_definition(ctx: &ReaderContext, ref_id: u64) -> Option<IiruDefinition> {
    if let Some(id) = ctx.id_cache.get::<crate::ir::ShapeAspectId>(ref_id) {
        return Some(IiruDefinition::ShapeAspect(id));
    }
    if let Some(id) = ctx.id_cache.get::<crate::ir::DatumFeatureId>(ref_id) {
        return Some(IiruDefinition::DatumFeature(id));
    }
    if let Some(id) = ctx.id_cache.get::<crate::ir::DatumId>(ref_id) {
        return Some(IiruDefinition::Datum(id));
    }
    if let Some(id) = ctx.id_cache.get::<crate::ir::DimensionalSizeId>(ref_id) {
        return Some(IiruDefinition::DimensionalSize(id));
    }
    if let Some(id) = ctx.id_cache.get::<crate::ir::GeometricToleranceId>(ref_id) {
        return Some(IiruDefinition::GeometricTolerance(id));
    }
    None
}

/// Map the synth `identified_item` SELECT to [`IiruIdentifiedItem`]: a single
/// ref → `Item`; a `SET`/`LIST_REPRESENTATION_ITEM` → `Compound` (per-child
/// resolved, unresolved skipped). An entirely-unresolved member → `None`.
fn lower_iiru_identified_item(
    ctx: &ReaderContext,
    sel: EarlyItemIdentifiedRepresentationUsageSelect,
) -> Option<IiruIdentifiedItem> {
    let (kind, refs) = match sel {
        EarlyItemIdentifiedRepresentationUsageSelect::EntityRef(r) => {
            return crate::entities::visualization::styled_item::resolve_representation_item_ref(
                ctx, r,
            )
            .map(IiruIdentifiedItem::Item);
        }
        EarlyItemIdentifiedRepresentationUsageSelect::SetRepresentationItem(refs) => {
            (CompoundItemKind::Set, refs)
        }
        EarlyItemIdentifiedRepresentationUsageSelect::ListRepresentationItem(refs) => {
            (CompoundItemKind::List, refs)
        }
    };
    let mut items = Vec::with_capacity(refs.len());
    for r in refs {
        if let Some(item) =
            crate::entities::visualization::styled_item::resolve_representation_item_ref(ctx, r)
        {
            items.push(item);
        }
    }
    if items.is_empty() {
        return None;
    }
    Some(IiruIdentifiedItem::Compound { kind, items })
}

/// Lower one `ITEM_IDENTIFIED_REPRESENTATION_USAGE`. `definition` (5-way) /
/// `used_representation` / `identified_item` each drop the carrier when
/// unresolved. Orphan round-trip — no `id_cache` entry.
pub(crate) fn lower_item_identified_representation_usage(
    ctx: &mut ReaderContext,
    _entity_id: u64,
    early: EarlyItemIdentifiedRepresentationUsage,
) {
    let Some(definition) = resolve_iiru_definition(ctx, early.definition) else {
        return;
    };
    let Some(used_representation) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(early.used_representation)
    else {
        return;
    };
    let Some(identified_item) = lower_iiru_identified_item(ctx, early.identified_item) else {
        return;
    };
    ctx.item_identified_representation_usages
        .push(ItemIdentifiedRepresentationUsage {
            name: early.name,
            description: early.description,
            definition,
            used_representation,
            identified_item,
        });
}

/// Resolve a `camera_image` body from `name` + `mapping_source` / `mapping_target`
/// step refs. `None` when either ref does not resolve (the carrier is dropped,
/// symmetric on re-read). Shared by the plain and `_3d_with_scale` lowers.
fn resolve_camera_image_body(
    ctx: &ReaderContext,
    name: String,
    source_ref: u64,
    target_ref: u64,
) -> Option<CameraImage> {
    let mapping_source = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationMapId>(source_ref)?;
    let mapping_target = ctx
        .id_cache
        .get::<crate::ir::id::PlanarExtentId>(target_ref)?;
    Some(CameraImage {
        name,
        mapping_source,
        mapping_target,
    })
}

/// Lower one `CAMERA_IMAGE` (`mapped_item` subtype). `mapping_source` narrows to
/// a `RepresentationMapId`, `mapping_target` to a `PlanarExtentId`; an unresolved
/// ref drops the carrier. Pushed to the `mapped_items` arena.
pub(crate) fn lower_camera_image(ctx: &mut ReaderContext, entity_id: u64, early: EarlyCameraImage) {
    let Some(body) =
        resolve_camera_image_body(ctx, early.name, early.mapping_source, early.mapping_target)
    else {
        return;
    };
    let mi_id = ctx.mapped_items.push(MappedItem::CameraImage(body));
    ctx.id_cache.insert(entity_id, mi_id);
}

/// Lower one `CAMERA_IMAGE_3D_WITH_SCALE` (AND-combined complex). Same body
/// resolution as the plain `CAMERA_IMAGE`; stored as the `CameraImage3dWithScale`
/// variant.
pub(crate) fn lower_camera_image_3d_with_scale(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCameraImage3dWithScale,
) {
    let Some(body) =
        resolve_camera_image_body(ctx, early.name, early.mapping_source, early.mapping_target)
    else {
        return;
    };
    let mi_id = ctx
        .mapped_items
        .push(MappedItem::CameraImage3dWithScale(body));
    ctx.id_cache.insert(entity_id, mi_id);
}

/// Lower one `REPRESENTATION_MAP` (`Itself`). `mapping_origin` resolves through
/// the shared representation-item resolver (unmodelled → drop); `mapped_representation`
/// probes the plain-representation arena then the presentation arena (else drop).
pub(crate) fn lower_representation_map(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyRepresentationMap,
) {
    let Some(mapping_origin) =
        crate::entities::visualization::styled_item::resolve_representation_item_ref(
            ctx,
            early.mapping_origin,
        )
    else {
        return;
    };
    let mapped_representation = if let Some(rid) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(early.mapped_representation)
    {
        MappedRepresentationRef::Representation(rid)
    } else if let Some(pid) = ctx
        .id_cache
        .get::<crate::ir::id::PresentationRepresentationId>(early.mapped_representation)
    {
        MappedRepresentationRef::Presentation(pid)
    } else {
        return;
    };
    let id = ctx
        .representation_maps
        .push(RepresentationMap::Itself(RepresentationMapData {
            mapping_origin,
            mapped_representation,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `MAPPED_ITEM` (`Itself`). `mapping_source` narrows to a
/// `RepresentationMapId`; `mapping_target` resolves through the shared
/// representation-item resolver. An unresolved ref drops the carrier.
pub(crate) fn lower_mapped_item(ctx: &mut ReaderContext, entity_id: u64, early: EarlyMappedItem) {
    let Some(mapping_source) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationMapId>(early.mapping_source)
    else {
        return;
    };
    let Some(mapping_target) =
        crate::entities::visualization::styled_item::resolve_representation_item_ref(
            ctx,
            early.mapping_target,
        )
    else {
        return;
    };
    let mi_id = ctx.mapped_items.push(MappedItem::Itself(MappedItemData {
        name: early.name,
        mapping_source,
        mapping_target,
    }));
    ctx.id_cache.insert(entity_id, mi_id);
}

/// Lower one `ITEM_DEFINED_TRANSFORMATION`. `transform_item_1` / `transform_item_2`
/// resolve through `resolve_placement` (an unresolved ref errors → drop + warn).
/// The `Transform3d` is stored in the `transform_map` side-map keyed by the entity
/// id; assembly consumers (NAUO / CDSR) fetch it there. `name` / `description` are
/// not modelled by `Transform3d` (the writer re-emits `''`).
pub(crate) fn lower_item_defined_transformation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyItemDefinedTransformation,
) -> Result<(), ConvertError> {
    let source = ctx.resolve_placement(entity_id, early.transform_item_1, "transform_item_1")?;
    let target = ctx.resolve_placement(entity_id, early.transform_item_2, "transform_item_2")?;
    ctx.transform_map
        .insert(entity_id, Transform3d { source, target });
    Ok(())
}

/// Lower one `DEFAULT_MODEL_GEOMETRIC_VIEW` (2-supertype flatten; leaf, no
/// `id_cache`). `item` is a `CAMERA_MODEL`, `rep` a `DRAUGHTING_MODEL`, `of_shape`
/// a `PRODUCT_DEFINITION_SHAPE` resolved to its product. Any unresolved member
/// drops the view. `description` defaults to `""` (re-emitted as `''`).
pub(crate) fn lower_default_model_geometric_view(
    ctx: &mut ReaderContext,
    early: &EarlyDefaultModelGeometricView,
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
    let Some(target) = ctx.product_of_pds(early.of_shape) else {
        return;
    };
    ctx.default_model_geometric_views
        .push(DefaultModelGeometricView {
            co_name: early.name.clone(),
            co_description: early.description.clone().unwrap_or_default(),
            sa_name: early.name_2.clone(),
            sa_description: early.description_2.clone().unwrap_or_default(),
            item,
            rep,
            target,
        });
}

/// Lower the complex `GLOBAL_UNIT_ASSIGNED_CONTEXT` (the
/// `(GEOMETRIC_REPRESENTATION_CONTEXT … REPRESENTATION_CONTEXT)` MI). Resolves
/// `units` / `uncertainty` refs to arena ids, preserves the GEO dimension and
/// the two `representation_context` strings in [`UnitContextForm::Complex`], and
/// registers the `context_id_map` entry consumed by representation converters.
pub(crate) fn lower_global_unit_assigned_context(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyGlobalUnitAssignedContext,
) {
    let (dimension, unit_refs, uncertainty_refs, repr_identifier, repr_type) = match early {
        EarlyGlobalUnitAssignedContext::Full(f) => (
            f.coordinate_space_dimension,
            f.units.as_slice(),
            f.uncertainty.as_slice(),
            f.context_identifier.clone(),
            f.context_type.clone(),
        ),
        EarlyGlobalUnitAssignedContext::NoUncertainty(n) => (
            n.coordinate_space_dimension,
            n.units.as_slice(),
            &[][..],
            n.context_identifier.clone(),
            n.context_type.clone(),
        ),
    };
    let units = resolve_unit_refs(ctx, entity_id, unit_refs);
    let uncertainty = resolve_uncertainty_refs(ctx, entity_id, uncertainty_refs);
    let ctx_id = ctx.units.push(UnitContext {
        units,
        uncertainty,
        form: UnitContextForm::Complex {
            coordinate_space_dimension: dimension,
            repr_identifier,
            repr_type,
        },
    });
    ctx.context_id_map.insert(entity_id, ctx_id);
}

/// Resolve a `GLOBAL_UNIT_ASSIGNED_CONTEXT.units` ref list (`SET OF unit`, any
/// kind) into `NamedUnitId`s in source order. An unresolved ref (a unit step-io
/// did not model) is surfaced and skipped (no synthesis).
pub(crate) fn resolve_unit_refs(
    ctx: &mut ReaderContext,
    entity_id: u64,
    unit_refs: &[u64],
) -> Vec<crate::ir::id::NamedUnitId> {
    let mut units = Vec::with_capacity(unit_refs.len());
    for r in unit_refs {
        if let Some(nu_id) = ctx.id_cache.get::<crate::ir::id::NamedUnitId>(*r) {
            units.push(nu_id);
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("GLOBAL_UNIT_ASSIGNED_CONTEXT.units ref #{r} unresolved"),
            });
        }
    }
    units
}

/// Resolve a `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT.uncertainty` ref list (each a
/// `UNCERTAINTY_MEASURE_WITH_UNIT`, lowered into the shared `measure_with_units`
/// arena) into `MeasureWithUnitId`s in source order. An unresolved ref (value
/// dropped on bind, or dangling) is surfaced and skipped.
fn resolve_uncertainty_refs(
    ctx: &mut ReaderContext,
    entity_id: u64,
    uncertainty_refs: &[u64],
) -> Vec<crate::ir::id::MeasureWithUnitId> {
    let mut out = Vec::with_capacity(uncertainty_refs.len());
    for r in uncertainty_refs {
        if let Some(id) = ctx.id_cache.get::<crate::ir::id::MeasureWithUnitId>(*r) {
            out.push(id);
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT.uncertainty ref #{r} unresolved"
                ),
            });
        }
    }
    out
}

/// Lower the standard (non-`$`) `GEOMETRIC_ITEM_SPECIFIC_USAGE`: `definition`
/// → `ShapeAspectRef`, `identified_item` SELECT → a single `RepresentationItemRef`
/// (GISU only ever carries the `EntityRef` member; SET/LIST drop, matching the
/// hand read's single-ref `read_entity_ref`), `used_representation` → arena id.
/// Any unresolved member drops the carrier. The non-standard `used_representation=$`
/// (`GisuUnsetUsedRep`) is intercepted + deferred in the handler before bind.
pub(crate) fn lower_geometric_item_specific_usage(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyGeometricItemSpecificUsage,
) {
    use crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref;
    use crate::entities::visualization::styled_item::resolve_representation_item_ref;

    let Some(definition) = resolve_shape_aspect_ref(ctx, early.definition) else {
        return;
    };
    let identified_item = match early.identified_item {
        EarlyItemIdentifiedRepresentationUsageSelect::EntityRef(r) => {
            let Some(item) = resolve_representation_item_ref(ctx, r) else {
                return;
            };
            item
        }
        // GISU narrows identified_item to a single representation_item; the
        // SET/LIST members never occur (and the hand read errored on them).
        EarlyItemIdentifiedRepresentationUsageSelect::SetRepresentationItem(_)
        | EarlyItemIdentifiedRepresentationUsageSelect::ListRepresentationItem(_) => return,
    };
    let Some(used_representation) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(early.used_representation)
    else {
        return;
    };
    let id = ctx
        .geometric_item_specific_usages
        .push(GeometricItemSpecificUsage {
            name: early.name,
            description: early.description,
            definition,
            used_representation,
            identified_item,
        });
    ctx.id_cache.insert(entity_id, id);
}
