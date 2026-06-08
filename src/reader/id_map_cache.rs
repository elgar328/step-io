//! Type-keyed cache of `file entity id (#N) → arena id` mappings for the
//! reader, replacing the former hand-declared `*_id_map: HashMap<u64, XId>`
//! fields on [`ReaderContext`](super::ReaderContext).
//!
//! This is the reader-side mirror of the writer's
//! [`StepIdCache`](crate::writer::buffer::step_id_cache::StepIdCache), but the
//! direction is reversed: the writer stores `arena_id → step_id (u64)`, whereas
//! the reader stores `file_id (u64) → arena_id`. The arena id is kept as its
//! `u32` index (every arena id is an `XId(pub u32)`) and reconstructed on lookup
//! via [`ArenaId::from_index`].
//!
//! Only the "simple" maps — those whose value is a single arena-id newtype with
//! a value type unique among the simple maps — live here. Maps whose value is an
//! enum / tuple / `Vec` / struct / raw `u64`, or that share an arena-id type
//! with another map (which would collide in one `TypeId` bucket), keep their own
//! named fields on `ReaderContext`.

use std::any::TypeId;
use std::collections::HashMap;

use crate::ir::arena::ArenaId;
use crate::ir::error::ConvertError;

/// `file_id (u64) → arena index (u32)`, bucketed by the arena-id value type.
#[derive(Default)]
pub(crate) struct IdMapCache {
    by_type: HashMap<TypeId, HashMap<u64, u32>>,
}

impl IdMapCache {
    /// Record that file entity `file_id` resolved to arena id `id`.
    // `index()` is u32-domain by construction (every arena id is `XId(pub u32)`
    // and `Arena::push` guards the u32 range), so the cast cannot truncate.
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn insert<K: ArenaId>(&mut self, file_id: u64, id: K) {
        self.by_type
            .entry(TypeId::of::<K>())
            .or_default()
            .insert(file_id, id.index() as u32);
    }

    /// Look up the arena id of type `K` registered for `file_id`.
    pub(crate) fn get<K: ArenaId>(&self, file_id: u64) -> Option<K> {
        self.by_type
            .get(&TypeId::of::<K>())
            .and_then(|m| m.get(&file_id))
            .map(|&idx| K::from_index(idx))
    }

    /// `true` when `file_id` is registered under arena-id type `K`.
    // Used once the geometry batch migrates `is_geometry_registered`.
    #[allow(dead_code)]
    pub(crate) fn contains<K: ArenaId>(&self, file_id: u64) -> bool {
        self.by_type
            .get(&TypeId::of::<K>())
            .is_some_and(|m| m.contains_key(&file_id))
    }

    /// `true` when no file id is registered under arena-id type `K`.
    pub(crate) fn is_empty<K: ArenaId>(&self) -> bool {
        self.by_type
            .get(&TypeId::of::<K>())
            .is_none_or(HashMap::is_empty)
    }

    /// Resolve `to` to its arena id of type `K`, or a `MissingReference` error.
    /// Mirrors the free function [`resolve_in_map`](super::resolve_in_map).
    // Used once the geometry batch migrates the `resolve_X` wrappers.
    #[allow(dead_code)]
    pub(crate) fn resolve<K: ArenaId>(
        &self,
        from: u64,
        to: u64,
        field_name: &'static str,
    ) -> Result<K, ConvertError> {
        self.get::<K>(to).ok_or(ConvertError::MissingReference {
            from,
            to,
            field_name,
        })
    }
}
