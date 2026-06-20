//! `EarlyGraph` — a typed read facade over the raw [`EntityGraph`](crate::parser).
//!
//! L2-building code (handlers, `lower`) cross-walks OTHER entities through this
//! facade's typed accessors (each folds `get` + name-guard + the generated
//! strict `bind` into one call returning an owned `Early*`) instead of touching
//! `RawEntity` directly. No leniency is added — accessors use the same strict
//! `bind`, so the L1 invariant holds. Non-standard diagnostic readers that must
//! inspect malformed raw input keep `&EntityGraph` directly (they are not trait
//! `read` fns); a `raw()` escape is added when the first such caller migrates.
//!
//! Accessors are added as cross-walk sites migrate — only the entities actually
//! cross-walked get one (not the whole 322-entity schema).

use crate::ir::error::ConvertError;
use crate::parser::entity::{EntityGraph, RawEntity};

use super::bind;
use super::model::EarlyRepresentation;

/// Typed front door to the L1 bind layer over a borrowed raw graph. `Copy` —
/// constructed on demand at each cross-walk (`EarlyGraph::new(graph)`).
#[derive(Clone, Copy)]
pub(crate) struct EarlyGraph<'a> {
    raw: &'a EntityGraph,
}

impl<'a> EarlyGraph<'a> {
    pub(crate) fn new(raw: &'a EntityGraph) -> Self {
        Self { raw }
    }

    /// Whether an entity id is present in the graph (dangling-reference probe).
    pub(crate) fn exists(self, id: u64) -> bool {
        self.raw.get(id).is_some()
    }

    /// Type name of a `Simple` entity, or `None` if absent / `Complex`. Used to
    /// discriminate a shared entity name (e.g. `REPRESENTATION` vs a subtype)
    /// before binding.
    pub(crate) fn type_name(self, id: u64) -> Option<&'a str> {
        match self.raw.get(id)? {
            RawEntity::Simple { name, .. } => Some(name.as_str()),
            RawEntity::Complex { .. } => None,
        }
    }

    /// Bind a bare `REPRESENTATION` (exact name). `None` when absent or not
    /// exactly `REPRESENTATION`; `Some(Err)` propagates a strict-bind defect
    /// (incl. `unset_required_field` for the c3d `context_of_items=$` drop).
    pub(crate) fn representation(
        self,
        id: u64,
    ) -> Option<Result<EarlyRepresentation, ConvertError>> {
        let RawEntity::Simple {
            name, attributes, ..
        } = self.raw.get(id)?
        else {
            return None;
        };
        if name != "REPRESENTATION" {
            return None;
        }
        Some(bind::bind_representation(id, attributes))
    }
}
