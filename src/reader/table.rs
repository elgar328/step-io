//! The entity table: one small row per input entity (plus the synthetic ones),
//! in id order, with an id → position lookup through a bucket table. It holds
//! what the drop pass and the resolve step need once the raw entities are
//! gone: the type, the arena slot the entity was built into, and where its
//! kept reference slots are.

use crate::generated::model::EntityKey;
use crate::generated::read::{SIMPLE_NAMES, complex_key_of, key_of};

/// Whether an entity is in the generated closure, and as what.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Member {
    /// A simple entity, by its index in the generated `SIMPLE_NAMES`.
    Simple(u16),
    /// A complex entity whose parts the generated part bag covers.
    Complex,
    /// Outside the closure (unmodeled, or not a schema name).
    Out,
    /// Dropped by normalization: a value could not be made standard.
    Rejected,
}

/// "No arena slot" / "no kept reference slots".
pub(crate) const NONE: u32 = u32::MAX;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Row {
    pub(crate) id: u64,
    pub(crate) member: Member,
    /// The interned label the drop pass names this entity by when it is not a
    /// simple closure entity: the type name, or the reason normalization
    /// dropped it. `NONE` for simple closure entities.
    pub(crate) label: u32,
    /// Index in the entity's arena; `NONE` if it is not built.
    pub(crate) arena: u32,
    /// Start of the entity's kept reference slots in the `RefStore`; `NONE`
    /// if it has none.
    pub(crate) refs: u32,
    /// Position in the file (synthetic entities follow the input ones).
    pub(crate) seq: u32,
    /// The line the entity starts on, for a duplicate id error (0 for
    /// synthetic entities); it fills the row's alignment padding.
    pub(crate) line: u32,
}

const _: () = assert!(std::mem::size_of::<Row>() == 32);

/// Sort rows into id order, ties in file order.
pub(crate) fn sort_rows(rows: &mut [Row]) {
    rows.sort_unstable_by_key(|r| (r.id, r.seq));
}

/// In rows sorted by [`sort_rows`], the repeat of an id that comes first in
/// the file, with the row it repeats.
pub(crate) fn first_duplicate(sorted: &[Row]) -> Option<(Row, Row)> {
    sorted
        .windows(2)
        .filter(|w| w[0].id == w[1].id)
        .map(|w| (w[0], w[1]))
        .min_by_key(|(_, r)| r.seq)
}

pub(crate) struct EntityTable {
    rows: Vec<Row>,
    /// Ids are bucketed by `id >> shift`; `starts[b]..starts[b + 1]` are the
    /// positions in bucket `b`. `shift` is the smallest that keeps the table
    /// within `2n + 2` entries, so dense numbering gets one id per bucket.
    shift: u32,
    starts: Vec<u32>,
}

impl EntityTable {
    /// Index rows sorted by [`sort_rows`] with distinct ids.
    pub(crate) fn new(rows: Vec<Row>) -> Self {
        let n = rows.len();
        let (shift, starts) = match rows.last() {
            None => (0, Vec::new()),
            Some(last) => {
                let limit = 2 * n as u64;
                let mut shift = 0;
                while (last.id >> shift) > limit {
                    shift += 1;
                }
                let end = last.id >> shift;
                let mut starts = Vec::with_capacity(usize::try_from(end + 2).unwrap_or(0));
                let mut p = 0;
                for b in 0..=end + 1 {
                    while p < n && (rows[p].id >> shift) < b {
                        p += 1;
                    }
                    starts.push(u32::try_from(p).unwrap_or(u32::MAX));
                }
                (shift, starts)
            }
        };
        EntityTable {
            rows,
            shift,
            starts,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.rows.len()
    }

    pub(crate) fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub(crate) fn rows_mut(&mut self) -> &mut [Row] {
        &mut self.rows
    }

    /// The position of the entity with this id, if there is one.
    pub(crate) fn pos(&self, id: u64) -> Option<usize> {
        // Compared in u64 before narrowing: an id past the last bucket must
        // not wrap into a valid one on a 32-bit target.
        let b = id >> self.shift;
        if b >= self.starts.len().saturating_sub(1) as u64 {
            return None;
        }
        let b = usize::try_from(b).ok()?;
        let (lo, hi) = (self.starts[b] as usize, self.starts[b + 1] as usize);
        self.rows[lo..hi]
            .binary_search_by_key(&id, |r| r.id)
            .ok()
            .map(|i| lo + i)
    }

    /// The model key of the entity at `pos`, if it is built.
    pub(crate) fn key(&self, pos: usize) -> Option<EntityKey> {
        let row = &self.rows[pos];
        if row.arena == NONE {
            return None;
        }
        let i = row.arena;
        match row.member {
            Member::Simple(ix) => key_of(ix, i),
            Member::Complex => complex_key_of(i),
            Member::Out | Member::Rejected => None,
        }
    }
}

/// Labels interned by the reader: type names of entities outside the simple
/// closure, normalization drop reasons, and drop-pass root labels.
#[derive(Default)]
pub(crate) struct Labels {
    names: Vec<Box<str>>,
    index: std::collections::HashMap<Box<str>, u32>,
}

impl Labels {
    pub(crate) fn intern(&mut self, s: &str) -> u32 {
        if let Some(&i) = self.index.get(s) {
            return i;
        }
        let i = u32::try_from(self.names.len()).unwrap_or(NONE);
        self.names.push(s.into());
        self.index.insert(s.into(), i);
        i
    }

    pub(crate) fn get(&self, i: u32) -> &str {
        self.names.get(i as usize).map_or("", |s| s)
    }

    /// How the drop pass names the entity in `row`.
    pub(crate) fn of(&self, row: &Row) -> &str {
        match row.member {
            Member::Simple(ix) => SIMPLE_NAMES.get(usize::from(ix)).copied().unwrap_or(""),
            _ => self.get(row.label),
        }
    }
}

/// The generated read's id → model key map: the key of a built entity follows
/// from its type and arena slot.
pub(crate) struct IdMap<'a> {
    table: &'a EntityTable,
}

impl<'a> IdMap<'a> {
    pub(crate) fn new(table: &'a EntityTable) -> Self {
        IdMap { table }
    }

    pub(crate) fn get(&self, id: u64) -> Option<EntityKey> {
        self.table.key(self.table.pos(id)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(ids: &[u64]) -> Vec<Row> {
        ids.iter()
            .zip(0..)
            .map(|(&id, seq)| Row {
                id,
                member: Member::Out,
                label: NONE,
                arena: NONE,
                refs: NONE,
                seq,
                line: 0,
            })
            .collect()
    }

    fn check(ids: &[u64], absent: &[u64]) {
        let table = EntityTable::new(rows(ids));
        assert_eq!(table.len(), ids.len());
        for (p, &id) in ids.iter().enumerate() {
            assert_eq!(table.pos(id), Some(p), "id {id} in {ids:?}");
            assert_eq!(table.rows()[p].id, id);
        }
        for &id in absent {
            assert_eq!(table.pos(id), None, "id {id} in {ids:?}");
        }
        assert!(table.starts.len() <= 2 * ids.len() + 2);
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
    fn empty_table_has_no_ids() {
        check(&[], &[0, 1, u64::MAX]);
    }

    #[test]
    fn reports_the_first_repeat_in_the_file() {
        // #7 repeats at file positions 3 and 4, #2 at position 2.
        let mut r = rows(&[7, 2, 2, 7, 7]);
        sort_rows(&mut r);
        let (first, dup) = first_duplicate(&r).unwrap();
        assert_eq!((dup.id, dup.seq, first.seq), (2, 2, 1));
        let mut r = rows(&[1, 2, 3]);
        sort_rows(&mut r);
        assert!(first_duplicate(&r).is_none());
    }
}
