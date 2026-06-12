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
    EarlyConstructiveGeometryRepresentationRelationship,
    EarlyMechanicalDesignAndDraughtingRelationship, EarlyRepresentationRelationship,
    EarlyShapeRepresentationRelationship,
};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{
    ConstructiveGeometryRepresentationRelationship, MechanicalDesignAndDraughtingRelationship,
    RepresentationRelationship, RepresentationRelationshipData, ShapeRepresentationRelationshipIr,
};
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
