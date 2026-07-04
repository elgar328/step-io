//! [`RefGraph`] — reverse-reference index over a [`StepModel`].
//!
//! The model stores references in the forward direction: an entity points to
//! what it uses. Many read queries want the inverse ("what references this
//! entity?") — the [`scene`](crate::scene) handles need it to walk an
//! assembly or a shape upward. [`StepModel::ref_graph`] builds the index by
//! inverting every entity's outgoing references; the enumeration underneath is
//! generated, so no entity type is missed.

use std::collections::HashMap;

use crate::StepModel;
use crate::generated::model::EntityKey;
use crate::generated::walk::for_each_entity;
use crate::generated::write::Writer;

/// An inverse-reference index: for each entity, the entities that reference it.
pub struct RefGraph {
    referrers: HashMap<EntityKey, Vec<EntityKey>>,
}

impl StepModel {
    /// Build the reverse-reference index for this model.
    ///
    /// O(total references); builds the whole index in one pass. Build once and
    /// reuse for many reverse queries.
    #[must_use]
    pub fn ref_graph(&self) -> RefGraph {
        let writer = Writer::new(self);
        let mut referrers: HashMap<EntityKey, Vec<EntityKey>> = HashMap::new();
        let mut deps = Vec::new();
        for_each_entity(self, |referrer| {
            deps.clear();
            writer.deps_of(referrer, &mut deps);
            for &target in &deps {
                referrers.entry(target).or_default().push(referrer);
            }
        });
        RefGraph { referrers }
    }
}

impl RefGraph {
    /// The entities that reference `target` (empty if none). Order is the model's
    /// entity-walk order.
    #[must_use]
    pub fn referrers(&self, target: EntityKey) -> &[EntityKey] {
        self.referrers.get(&target).map_or(&[], Vec::as_slice)
    }
}
