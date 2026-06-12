//! Shape-representation-domain `lift` fns (the representation relationship
//! cluster). See the [module docs](super) for the lifting contract.

use crate::early::model::{
    EarlyCharacterizedItemWithinRepresentation,
    EarlyConstructiveGeometryRepresentationRelationship, EarlyDatumTarget,
    EarlyMechanicalDesignAndDraughtingRelationship, EarlyRealRepresentationItem,
    EarlyRepresentationContext, EarlyRepresentationRelationship,
    EarlyShapeRepresentationRelationship, EarlyToleranceZone,
};

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
