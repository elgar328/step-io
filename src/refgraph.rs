//! [`RefGraph`] — reverse-reference index over a [`StepModel`].
//!
//! The model stores references in the forward direction: an entity points to
//! what it uses. Many read queries want the inverse ("what references this
//! entity?") — the [`scene`](crate::scene) handles need it to walk an
//! assembly or a shape upward. [`StepModel::ref_graph`] builds the index by
//! inverting every entity's outgoing references; the enumeration underneath is
//! generated, so no entity type is missed.

use crate::StepModel;
use crate::generated::model::EntityKey;
use crate::generated::walk::for_each_entity;
use crate::generated::write::Writer;

/// An inverse-reference index: for each entity, the entities that reference it.
///
/// Entities are numbered densely, kind by kind; the referrers of entity `n`
/// are `referrers[starts[n]..starts[n + 1]]`.
pub struct RefGraph {
    /// The first number of each kind of entity, and the total at the end.
    bases: Vec<usize>,
    starts: Vec<usize>,
    referrers: Vec<EntityKey>,
}

impl StepModel {
    /// Build the reverse-reference index for this model.
    ///
    /// O(total references): two walks over the model's references, one to
    /// count them and one to place them. Build once and reuse for many
    /// reverse queries.
    #[must_use]
    pub fn ref_graph(&self) -> RefGraph {
        let mut bases = vec![0; EntityKey::KINDS + 1];
        for_each_entity(self, |key| bases[key.kind_index().0 + 1] += 1);
        for k in 1..bases.len() {
            bases[k] += bases[k - 1];
        }
        let dense = |key: EntityKey| {
            let (kind, i) = key.kind_index();
            bases[kind] + i as usize
        };

        // Count each entity's referrers, then turn the counts into starts.
        let mut starts = vec![0; bases[EntityKey::KINDS] + 1];
        let mut any = None;
        let mut deps = Vec::new();
        for_each_entity(self, |referrer| {
            deps.clear();
            Writer::deps_of(self, referrer, &mut deps);
            for &target in &deps {
                starts[dense(target) + 1] += 1;
                any = Some(referrer);
            }
        });
        for n in 1..starts.len() {
            starts[n] += starts[n - 1];
        }

        // Place the referrers, each entity's in walk order.
        let Some(any) = any else {
            return RefGraph {
                bases,
                starts,
                referrers: Vec::new(),
            };
        };
        // Each entity's start serves as its cursor; once all are placed it
        // holds the next entity's start, so the starts shift back by one.
        let n = starts.len() - 1;
        let mut referrers = vec![any; starts[n]];
        for_each_entity(self, |referrer| {
            deps.clear();
            Writer::deps_of(self, referrer, &mut deps);
            for &target in &deps {
                let at = &mut starts[dense(target)];
                referrers[*at] = referrer;
                *at += 1;
            }
        });
        starts.copy_within(0..n, 1);
        starts[0] = 0;
        RefGraph {
            bases,
            starts,
            referrers,
        }
    }
}

impl RefGraph {
    /// The entities that reference `target` (empty if none). Order is the model's
    /// entity-walk order.
    #[must_use]
    pub fn referrers(&self, target: EntityKey) -> &[EntityKey] {
        let (kind, i) = target.kind_index();
        let n = self.bases[kind] + i as usize;
        if n >= self.bases[kind + 1] {
            return &[];
        }
        &self.referrers[self.starts[n]..self.starts[n + 1]]
    }
}
