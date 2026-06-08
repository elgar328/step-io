//! Type-keyed step-id cache.
//!
//! The writer assigns every emitted entity a STEP `#N` id and later looks it
//! up when another entity references it. Previously this was ~100 separate
//! `Vec<u64>` fields on `WriteBuffer`, each hand-declared, hand-sized and
//! hand-indexed. This replaces the arena-1:1 ones with a single store keyed by
//! the arena-id *type*: adding a new entity needs no new field.
//!
//! Each arena keeps a dense `Vec<u64>` indexed by [`ArenaId::index`]. `0` is
//! the unset sentinel — `WriteBuffer::fresh` starts at `1`, so a real STEP id
//! is never `0`.

use std::any::TypeId;
use std::collections::HashMap;

use crate::ir::arena::ArenaId;

/// Maps each arena-id type to a dense `Vec<u64>` of emitted STEP ids.
#[derive(Default)]
pub(crate) struct StepIdCache {
    by_type: HashMap<TypeId, Vec<u64>>,
}

impl StepIdCache {
    /// Emitted STEP id for `id`, or `0` if none was recorded.
    pub(crate) fn get<K: ArenaId>(&self, id: K) -> u64 {
        self.by_type
            .get(&TypeId::of::<K>())
            .and_then(|v| v.get(id.index()))
            .copied()
            .unwrap_or(0)
    }

    /// Record the emitted STEP id for `id`, growing the backing vec as needed.
    pub(crate) fn set<K: ArenaId>(&mut self, id: K, step: u64) {
        let v = self.by_type.entry(TypeId::of::<K>()).or_default();
        if v.len() <= id.index() {
            v.resize(id.index() + 1, 0);
        }
        v[id.index()] = step;
    }
}
