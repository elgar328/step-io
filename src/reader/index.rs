//! Position index over the normalized entity graph, shared by `drop_pass`
//! and the generated read: entities are numbered `0..n` in id order, each
//! with its generated type index resolved once, and an id is found through a
//! bucket table instead of a tree walk.

use std::collections::BTreeMap;

use crate::generated::model::EntityKey;
use crate::generated::read::{is_complex_unit, simple_index};
use crate::parser::RawEntity;

/// Whether an entity is in the generated closure, and as what.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Member {
    /// A simple entity, by its index in the generated `SIMPLE_NAMES`.
    Simple(u16),
    /// A complex entity whose parts the generated part bag covers.
    Complex,
    /// Outside the closure (unmodeled, or not a schema name).
    Out,
}

pub(crate) struct EntityIndex<'g> {
    /// Entity ids by position, ascending.
    ids: Vec<u64>,
    entities: Vec<(&'g RawEntity, Member)>,
    /// Ids are bucketed by `id >> shift`; `starts[b]..starts[b + 1]` are the
    /// positions in bucket `b`. `shift` is the smallest that keeps the table
    /// within `2n + 2` entries, so dense numbering gets one id per bucket.
    shift: u32,
    starts: Vec<usize>,
}

impl<'g> EntityIndex<'g> {
    pub(crate) fn new(graph: &'g BTreeMap<u64, RawEntity>) -> Self {
        let n = graph.len();
        let mut ids = Vec::with_capacity(n);
        let mut entities = Vec::with_capacity(n);
        for (&id, ent) in graph {
            let member = match ent {
                RawEntity::Simple { name, .. } => {
                    simple_index(name).map_or(Member::Out, Member::Simple)
                }
                RawEntity::Complex { parts, .. } if is_complex_unit(parts) => Member::Complex,
                RawEntity::Complex { .. } => Member::Out,
            };
            ids.push(id);
            entities.push((ent, member));
        }
        let (shift, starts) = match ids.last() {
            None => (0, Vec::new()),
            Some(&max) => {
                let limit = 2 * n as u64;
                let mut shift = 0;
                while (max >> shift) > limit {
                    shift += 1;
                }
                let last = max >> shift;
                let mut starts = Vec::with_capacity(usize::try_from(last + 2).unwrap_or(0));
                let mut p = 0;
                for b in 0..=last + 1 {
                    while p < n && (ids[p] >> shift) < b {
                        p += 1;
                    }
                    starts.push(p);
                }
                (shift, starts)
            }
        };
        EntityIndex {
            ids,
            entities,
            shift,
            starts,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.ids.len()
    }

    pub(crate) fn id(&self, pos: usize) -> u64 {
        self.ids[pos]
    }

    pub(crate) fn entity(&self, pos: usize) -> &'g RawEntity {
        self.entities[pos].0
    }

    pub(crate) fn member(&self, pos: usize) -> Member {
        self.entities[pos].1
    }

    /// The position of the entity with this id, if the graph has one.
    pub(crate) fn pos(&self, id: u64) -> Option<usize> {
        // Compared in u64 before narrowing: an id past the last bucket must
        // not wrap into a valid one on a 32-bit target.
        let b = id >> self.shift;
        if b >= self.starts.len().saturating_sub(1) as u64 {
            return None;
        }
        let b = usize::try_from(b).ok()?;
        let (lo, hi) = (self.starts[b], self.starts[b + 1]);
        self.ids[lo..hi].binary_search(&id).ok().map(|i| lo + i)
    }
}

/// The generated read's id → model key map, stored by position.
pub(crate) struct IdMap<'a> {
    index: &'a EntityIndex<'a>,
    keys: Vec<Option<EntityKey>>,
}

impl<'a> IdMap<'a> {
    pub(crate) fn new(index: &'a EntityIndex<'a>) -> Self {
        IdMap {
            index,
            keys: vec![None; index.len()],
        }
    }

    // Same shape as `BTreeMap::get`, so the generated call sites read as before.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(crate) fn get(&self, id: &u64) -> Option<&EntityKey> {
        self.keys[self.index.pos(*id)?].as_ref()
    }

    /// Only ids of the indexed graph are inserted (the read builds kept
    /// entities only).
    pub(crate) fn insert(&mut self, id: u64, key: EntityKey) {
        let pos = self.index.pos(id);
        debug_assert!(pos.is_some(), "IdMap::insert: #{id} is not in the index");
        if let Some(p) = pos {
            self.keys[p] = Some(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::lexer::Span;

    fn graph(ids: &[u64]) -> BTreeMap<u64, RawEntity> {
        ids.iter()
            .map(|&id| {
                let e = RawEntity::Simple {
                    id,
                    name: "CARTESIAN_POINT".to_owned(),
                    attributes: Vec::new(),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                };
                (id, e)
            })
            .collect()
    }

    fn check(ids: &[u64], absent: &[u64]) {
        let g = graph(ids);
        let index = EntityIndex::new(&g);
        assert_eq!(index.len(), ids.len());
        for (p, &id) in ids.iter().enumerate() {
            assert_eq!(index.pos(id), Some(p), "id {id} in {ids:?}");
            assert_eq!(index.id(p), id);
        }
        for &id in absent {
            assert_eq!(index.pos(id), None, "id {id} in {ids:?}");
        }
        assert!(index.starts.len() <= 2 * ids.len() + 2);
    }

    #[test]
    fn finds_every_id_and_nothing_else() {
        // Dense, stepped by ten, one outlier, and appended synthetic ids.
        check(&[1, 2, 3, 4, 5], &[0, 6, 100]);
        check(&[10, 20, 30, 40, 50], &[0, 9, 11, 25, 51, 60]);
        check(&[1, 2, 3, 5_000_000], &[4, 4_999_999, 5_000_001]);
        check(&[10, 20, 30, 31, 32], &[29, 33]);
        check(&[u64::MAX], &[0, u64::MAX - 1]);
        check(&[0, u64::MAX], &[1, u64::MAX / 2]);
    }

    #[test]
    fn empty_graph_has_no_ids() {
        check(&[], &[0, 1, u64::MAX]);
    }

    #[test]
    fn classifies_members_once() {
        let g = graph(&[1]);
        let index = EntityIndex::new(&g);
        assert!(matches!(index.member(0), Member::Simple(_)));
    }
}
