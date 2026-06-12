//! Shape-representation-domain `lift` fns (the representation relationship
//! cluster). See the [module docs](super) for the lifting contract.

use crate::early::model::{EarlyRepresentationRelationship, EarlyShapeRepresentationRelationship};

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
