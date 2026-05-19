//! `form_features` pool — manufacturing feature definitions.
//!
//! Currently holds only `STEP` (AP242 `feature_definition` subtype — the
//! manufacturing-step entity, *not* the P21 file format). Future Phase 4
//! expands this pool with the remaining manufacturing-feature entities.

use super::arena::Arena;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormFeaturesPool {
    pub feature_definitions: Arena<Step>,
}

/// `STEP(name, description)` — AP242 manufacturing step feature. Both
/// fields are inherited from `characterized_object` via `feature_definition`.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub name: String,
    pub description: Option<String>,
}
