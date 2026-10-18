//! [`read`] — STEP source to [`StepModel`] plus a provenance [`Report`].
//!
//! [`read`] streams the source: it takes the entities from the
//! [`parser`](crate::parser) one at a time, heals non-standard input in place
//! where a safe rewrite exists, and builds each entity into the typed model
//! right away with the generated readers, keeping only a row in the entity
//! table and the entity's reference slots. Once the whole file is read, the
//! drop pass decides what cannot enter the model, with a [`DropReason`], and
//! the kept references are resolved. The [`Report`] accounts for everything
//! that came in — kept, normalized, or dropped and why — and carries what the
//! parser repaired on the way in.

use std::collections::BTreeMap;

use crate::generated::generic_normalize;
use crate::generated::model::StepModel;
use crate::generated::read::{
    COMPLEX_PART_NAMES, RefSlot, SIMPLE_NAMES, build_complex_entity, build_entity,
    complex_part_index, complex_ref_field_slots, complex_ref_slots, finalize_arenas,
    ref_field_slots, ref_slots, resolve_complex_entity, resolve_entity, shrink_arenas,
    simple_index,
};
use crate::parser::lexer::Span;
use crate::parser::p21::Parser;
use crate::parser::{Attribute, Error, ParseWarning, RawEntity, RawEntityPart};

mod entity_normalize;
mod refs;
pub(crate) mod table;

use entity_normalize::Synth;
use refs::RefStore;
use table::{EntityTable, IdMap, Labels, Member, NONE, Row, first_duplicate, sort_rows};

/// Why an entity was dropped before it could enter the model.
/// `NonstandardValue` = one of the entity's own attribute *values* is nonstandard
/// and could not be normalized (a required reference is missing, an integer field
/// holds a fractional value). `NonstandardReference` = one of its *reference*
/// slots points at a target the schema does not admit. `Unimplemented` = the
/// entity type is outside the generated closure (not yet modeled, or an
/// unknown/non-schema name). `Cascade` = a referrer whose target was
/// dropped/dangling. `Unclassified` = the generated read could not consume the
/// entity's own attributes (off-kind scalar, bad enum token, short arity, …) and
/// no policy layer (normalize / `drop_pass`) classified it — a frontier signal:
/// investigate and absorb via normalize or `nonstd_ref`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DropKind {
    NonstandardValue,
    Unimplemented,
    NonstandardReference,
    Cascade,
    Unclassified,
}

/// A drop reason: kind + a human label (`<TYPE>` / `<ENT>: '<slot>' references
/// <TYPE>` / a slot-rule name / `via-<root>`). Aggregated as instance counts per
/// reason.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DropReason {
    pub kind: DropKind,
    pub key: String,
}

/// Input-to-model provenance. Every input entity plus every synthetic entity
/// added by normalization is either kept (`validated`) or dropped with a reason
/// (`drops`): `validated + sum(drops) == n_in + n_synth`. `norm` = rewrite (fix)
/// notes from normalization; `parse_warnings` = what the parser repaired
/// before that. The source header (including the identified schema)
/// lives on the model: [`StepModel::header`].
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Report {
    /// Input entity count (data section, before normalization).
    pub n_in: usize,
    /// Synthetic entities added by per-entity normalization (add-only).
    pub n_synth: usize,
    /// Kept entity count (entities that entered the model).
    pub validated: usize,
    /// Dropped entities, per-entity: input id + reason (nonstandard-value +
    /// unimplemented + cascade + nonstandard-reference), in id order.
    /// `dropped.len()` is the total drop count.
    pub dropped: Vec<(u64, DropReason)>,
    /// Non-standard rewrite notes (entities fixed in place), in the order the
    /// entities appear in the file.
    pub norm: Vec<&'static str>,
    /// What the parser repaired or skipped on the way in (blank attribute
    /// slots, an undecodable string escape, …) — fixed at the syntax level,
    /// before normalization. Each prints as a one-line message.
    pub parse_warnings: Vec<ParseWarning>,
}

fn to_u32(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(NONE)
}

/// How the drop pass names a complex entity: its part names, sorted.
fn complex_name(parts: &[RawEntityPart]) -> String {
    let mut n: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
    n.sort_unstable();
    format!("({})", n.join("+"))
}

/// A synthetic entity added by normalization, until it is numbered.
struct Synthetic {
    /// The id of the entity whose fix references it.
    creator: u64,
    row: Row,
    build: Result<(), String>,
}

/// The reader's state while the entities stream past: everything kept of
/// them once each is built and released.
#[derive(Default)]
struct Scan {
    model: StepModel,
    rows: Vec<Row>,
    refs: RefStore,
    labels: Labels,
    norm: Vec<&'static str>,
    build_fail: Vec<(u64, String)>,
    synthetic: Vec<Synthetic>,
    max_id: Option<u64>,
}

impl Scan {
    /// Normalize one entity, build it into the model, and keep its row and
    /// reference slots.
    fn entity(&mut self, mut ent: RawEntity) {
        let id = ent.id();
        self.max_id = Some(self.max_id.map_or(id, |m| m.max(id)));
        let mut row = Row {
            id,
            member: Member::Out,
            label: NONE,
            arena: NONE,
            refs: NONE,
            seq: to_u32(self.rows.len()),
        };
        // A synthetic entity is added before the generic rules run: it stays
        // in the model even if they then drop the entity that references it.
        let synth = entity_normalize::normalize_entity(&mut ent, &mut self.norm).map(|s| {
            let Synth::Colour { slot } = s;
            let k = to_u32(self.synthetic.len());
            self.add_colour(id);
            (slot, k)
        });
        if let Err(reason) = generic_normalize::normalize_entity(&mut ent, &mut self.norm) {
            row.member = Member::Rejected;
            row.label = self.labels.intern(reason);
            self.rows.push(row);
            return;
        }
        match ent {
            RawEntity::Simple {
                name, attributes, ..
            } => match simple_index(&name) {
                Some(ix) => {
                    row.member = Member::Simple(ix);
                    match build_entity(&mut self.model, ix, &attributes) {
                        Ok(i) => row.arena = to_u32(i),
                        Err(e) => self.build_fail.push((id, e)),
                    }
                    row.refs = self
                        .refs
                        .push_simple(&attributes, ref_field_slots(ix), synth);
                }
                None => row.label = self.labels.intern(&name),
            },
            RawEntity::Complex { parts, .. } => {
                row.label = self.labels.intern(&complex_name(&parts));
                let pixs: Option<Vec<u16>> =
                    parts.iter().map(|p| complex_part_index(&p.name)).collect();
                if let Some(pixs) = pixs.filter(|p| !p.is_empty()) {
                    row.member = Member::Complex;
                    match build_complex_entity(&mut self.model, &parts) {
                        Ok(i) => row.arena = to_u32(i),
                        Err(e) => self.build_fail.push((id, e)),
                    }
                    row.refs = self
                        .refs
                        .push_complex(&parts, &pixs, complex_ref_field_slots);
                }
            }
        }
        self.rows.push(row);
    }

    /// Add a bare `COLOUR()` for the entity `creator` references it from.
    fn add_colour(&mut self, creator: u64) {
        let mut row = Row {
            id: 0,
            member: Member::Out,
            label: NONE,
            arena: NONE,
            refs: NONE,
            seq: 0,
        };
        let mut build = Ok(());
        match simple_index("COLOUR") {
            Some(ix) => {
                row.member = Member::Simple(ix);
                match build_entity(&mut self.model, ix, &[]) {
                    Ok(i) => row.arena = to_u32(i),
                    Err(e) => build = Err(e),
                }
            }
            None => row.label = self.labels.intern("COLOUR"),
        }
        self.synthetic.push(Synthetic {
            creator,
            row,
            build,
        });
    }

    /// Once the entities are read: sort the rows into the entity table,
    /// report a duplicate id, and number the synthetic entities.
    /// `end` = how the rest of the source parsed; a duplicate id among the
    /// entities read wins over its error, as the first error in the file.
    fn finish(
        mut self,
        src: &[u8],
        end: Result<Vec<ParseWarning>, Error>,
    ) -> Result<Scanned, Error> {
        let mut rows = std::mem::take(&mut self.rows);
        sort_rows(&mut rows);
        if let Some(dup) = first_duplicate(&rows) {
            return Err(Error::DuplicateEntityId {
                id: dup.id,
                span: span_of(src, dup.seq),
            });
        }
        let parse_warnings = end?;
        let n_in = rows.len();

        // Synthetic ids follow the largest input id, skipping ids in use (and
        // 0), in the order of the entities that reference them.
        let mut by_creator: Vec<usize> = (0..self.synthetic.len()).collect();
        by_creator.sort_unstable_by_key(|&k| self.synthetic[k].creator);
        let mut next = self.max_id.map_or(1, |m| m.saturating_add(1));
        let mut ids = vec![0; self.synthetic.len()];
        for k in by_creator {
            while next == 0 || rows.binary_search_by_key(&next, |r| r.id).is_ok() {
                next = next.wrapping_add(1);
            }
            ids[k] = next;
            next = next.wrapping_add(1);
        }
        let n_synth = self.synthetic.len();
        for (k, s) in self.synthetic.into_iter().enumerate() {
            let id = ids[k];
            if let Err(e) = s.build {
                self.build_fail.push((id, e));
            }
            rows.push(Row {
                id,
                seq: to_u32(n_in + k),
                ..s.row
            });
        }
        if n_synth > 0 {
            sort_rows(&mut rows);
        }
        self.refs.set_synth_ids(ids);
        Ok(Scanned {
            model: self.model,
            table: EntityTable::new(rows),
            refs: self.refs,
            labels: self.labels,
            norm: self.norm,
            build_fail: self.build_fail,
            n_in,
            n_synth,
            parse_warnings,
        })
    }
}

/// The span of the `seq`-th entity of the source, for an error about it
/// (parses the source again up to it).
fn span_of(src: &[u8], seq: u32) -> Span {
    let mut p = Parser::from_bytes(src);
    let found = p.read_prelude().ok().and_then(|_| {
        (0..=seq)
            .map(|_| p.next_entity().ok().flatten())
            .last()
            .flatten()
    });
    found.map_or(
        Span {
            start: 0,
            end: 0,
            line: 0,
            column: 0,
        },
        |e| e.span(),
    )
}

/// The streamed source, ready for the drop pass and the resolve step.
struct Scanned {
    model: StepModel,
    table: EntityTable,
    refs: RefStore,
    labels: Labels,
    norm: Vec<&'static str>,
    build_fail: Vec<(u64, String)>,
    n_in: usize,
    n_synth: usize,
    parse_warnings: Vec<ParseWarning>,
}

/// Slot-aware nonstandard-ref check: does any ref slot of the entity in `row`
/// point at an in-closure target whose type the slot's SELECT does not admit?
/// Returns the first `<ENT>: '<slot>' references <TYPE>` label. Out-of-closure
/// / dangling targets are NOT flagged here — those are cascade/unimplemented.
fn nonstd_ref(
    table: &EntityTable,
    refs: &RefStore,
    labels: &Labels,
    row: &Row,
    buf: &mut Vec<u64>,
) -> Option<String> {
    let mut found = None;
    refs.slot_refs(row.refs, buf, |pix, idx, ids| {
        let (ename, slots): (&str, &[RefSlot]) = match row.member {
            Member::Simple(ix) => (
                SIMPLE_NAMES.get(usize::from(ix)).copied().unwrap_or(""),
                ref_slots(ix),
            ),
            Member::Complex => (
                COMPLEX_PART_NAMES
                    .get(usize::from(pix))
                    .copied()
                    .unwrap_or(""),
                complex_ref_slots(pix),
            ),
            Member::Out | Member::Rejected => return false,
        };
        let Some(rs) = slots.iter().find(|rs| rs.idx == idx as usize) else {
            return false;
        };
        for &r in ids {
            let Some(tp) = table.pos(r) else {
                continue;
            };
            let target = &table.rows()[tp];
            let admitted = match target.member {
                // Not in the closure: cascade / unimplemented handles it.
                Member::Out | Member::Rejected => continue,
                Member::Complex => rs.complex_ok,
                Member::Simple(t) => rs.allowed.binary_search(&t).is_ok(),
            };
            if !admitted {
                found = Some(format!(
                    "{ename}: '{}' references {}",
                    rs.name,
                    labels.of(target)
                ));
                return true;
            }
        }
        false
    });
    found
}

/// Decide which entities enter the model, recording WHY each other one is
/// dropped. An entity survives iff it is a closure type, all its transitive
/// refs stay kept, and no ref slot holds a type the slot does not admit
/// (fixpoint). Returns the kept flags (by table position) and how many are
/// set.
///
/// `known_bad` = entities the generated read already failed on (own-attr);
/// they are pre-dropped roots here so their referrers cascade. Their drop
/// reason is owned by the caller's read-failure accumulator, so they are NOT
/// re-recorded in `dropped` (avoids double-counting in the provenance
/// accounting). Entities normalization dropped are roots the same way, under
/// their normalization reason.
fn drop_pass(
    table: &EntityTable,
    refs: &RefStore,
    labels: &mut Labels,
    known_bad: &[bool],
) -> (Vec<bool>, usize, Vec<(u64, DropReason)>) {
    let n = table.len();
    let rows = table.rows();
    let mut dropped: Vec<(u64, DropReason)> = Vec::new();
    let mut keep = vec![false; n];
    // Root labels of dropped entities, by position.
    let mut root_of: BTreeMap<usize, u32> = BTreeMap::new();
    let unclassified = labels.intern("unclassified");
    for (pos, kept) in keep.iter_mut().enumerate() {
        let row = &rows[pos];
        if known_bad[pos] {
            root_of.insert(pos, unclassified);
            continue;
        }
        match row.member {
            Member::Simple(_) | Member::Complex => *kept = true,
            Member::Rejected => {
                root_of.insert(pos, row.label);
            }
            Member::Out => {
                root_of.insert(pos, row.label);
                dropped.push((
                    row.id,
                    DropReason {
                        kind: DropKind::Unimplemented,
                        key: labels.get(row.label).to_string(),
                    },
                ));
            }
        }
    }
    let mut ids: Vec<u64> = Vec::new();
    let mut buf: Vec<u64> = Vec::new();
    loop {
        let mut removed = false;
        // In id order; an entity only ever drops itself, so checking its flag
        // at its turn is the same as snapshotting the kept set up front.
        for pos in 0..n {
            if !keep[pos] {
                continue;
            }
            let row = &rows[pos];
            refs.refs(row.refs, &mut ids);
            let missing = ids.iter().find_map(|&r| match table.pos(r) {
                Some(p) if keep[p] => None,
                p => Some(p),
            });
            if let Some(target) = missing {
                keep[pos] = false;
                let root = target.and_then(|p| root_of.get(&p).copied());
                let key = format!("via-{}", root.map_or("dangling", |l| labels.get(l)));
                let root = root.unwrap_or_else(|| labels.intern("dangling"));
                dropped.push((
                    row.id,
                    DropReason {
                        kind: DropKind::Cascade,
                        key,
                    },
                ));
                root_of.insert(pos, root);
                removed = true;
                continue;
            }
            if let Some(reason) = nonstd_ref(table, refs, labels, row, &mut buf) {
                keep[pos] = false;
                root_of.insert(pos, labels.intern(&reason));
                dropped.push((
                    row.id,
                    DropReason {
                        kind: DropKind::NonstandardReference,
                        key: reason,
                    },
                ));
                removed = true;
            }
        }
        if !removed {
            break;
        }
    }
    let kept = keep.iter().filter(|&&k| k).count();
    (keep, kept, dropped)
}

/// Keep in the arenas only the entities in `kept`, in id order, and point
/// the table rows at their new arena slots (`NONE` for the removed ones).
fn finalize(model: &mut StepModel, table: &mut EntityTable, kept: &[bool]) {
    let mut order: Vec<Vec<u32>> = vec![Vec::new(); SIMPLE_NAMES.len()];
    let mut complex: Vec<u32> = Vec::new();
    for (row, &k) in table.rows_mut().iter_mut().zip(kept) {
        if row.arena == NONE {
            continue;
        }
        if !k {
            row.arena = NONE;
            continue;
        }
        let list = match row.member {
            Member::Simple(ix) => order.get_mut(usize::from(ix)),
            Member::Complex => Some(&mut complex),
            Member::Out | Member::Rejected => None,
        };
        row.arena = list.map_or(NONE, |l| {
            l.push(row.arena);
            to_u32(l.len() - 1)
        });
    }
    finalize_arenas(model, &order, &complex);
}

/// Resolve the references of every kept, built entity, in id order. Returns
/// the failures, by table position.
fn resolve(
    model: &mut StepModel,
    table: &EntityTable,
    refs: &RefStore,
    kept: &[bool],
) -> Vec<(usize, String)> {
    let idmap = IdMap::new(table);
    let mut fails = Vec::new();
    let mut attrs: Vec<Attribute> = Vec::new();
    let mut parts: Vec<RawEntityPart> = Vec::new();
    for (pos, row) in table.rows().iter().enumerate() {
        if !kept[pos] || row.arena == NONE || row.refs == NONE {
            continue;
        }
        let i = row.arena as usize;
        let r = match row.member {
            Member::Simple(ix) if !ref_field_slots(ix).is_empty() => {
                refs.restore_simple(row.refs, &mut attrs);
                resolve_entity(model, ix, i, &attrs, &idmap)
            }
            Member::Complex => {
                refs.restore_complex(row.refs, &mut parts);
                resolve_complex_entity(model, i, &parts, &idmap)
            }
            _ => continue,
        };
        if let Err(e) = r {
            fails.push((pos, e));
        }
    }
    fails
}

/// Read STEP source into the schema-faithful model, returning a provenance
/// `Report`.
///
/// The source is read one entity at a time: each is normalized and built
/// into the model as it is parsed, keeping only its references for later.
/// The generated read is per-entity fallible: an entity whose own attributes
/// the strict reader cannot consume is dropped (reason recorded), never
/// panicking. A read failure pre-drops that entity in the next `drop_pass`,
/// so its referrers cascade through the same engine. The loop is bounded and
/// converges in ≤2 real iterations (own-attr readability is
/// graph-independent, so all failures surface in the first round; the second
/// `drop_pass` cascades them to a clean set).
///
/// # Errors
/// Returns a [`Error`] only if the source does not parse. A parsed source
/// never fails the read: a malformed entity is dropped with a reason in
/// [`Report::dropped`] (the generated read is structurally panic-free).
pub fn read(src: &[u8]) -> Result<(StepModel, Report), Error> {
    /// ≤2 real iterations; one extra for slack (a violation degrades to dropping,
    /// not hanging — `debug_assert` flags non-convergence in dev).
    const MAX_ITERS: usize = 3;

    let mut parser = Parser::from_bytes(src);
    let prelude = parser.read_prelude()?;
    let header = crate::header::from_raw(&prelude.header, prelude.schema);
    let mut scan = Scan::default();
    let end = loop {
        match parser.next_entity() {
            Ok(Some(ent)) => scan.entity(ent),
            Ok(None) => break parser.finish(),
            Err(e) => break Err(e),
        }
    };
    let Scanned {
        mut model,
        mut table,
        refs,
        mut labels,
        norm,
        build_fail,
        n_in,
        n_synth,
        parse_warnings,
    } = scan.finish(src, end)?;

    // Outer cascade fixpoint: drop_pass (graph/ref validity) -> resolve
    // (own-attr and ref validity). Read failures feed `known_bad` so the next
    // drop_pass cascades their referrers. `drop_pass` always seeds from the
    // whole table (never `kept`) and `known_bad` only grows, so the kept set
    // shrinks monotonically — this guarantees termination, and lets each
    // round trim the arenas the previous one left.
    let build_fail: Vec<(usize, u64, String)> = build_fail
        .into_iter()
        .filter_map(|(id, e)| table.pos(id).map(|p| (p, id, e)))
        .collect();
    let mut known_bad = vec![false; table.len()];
    let mut read_fail: BTreeMap<u64, String> = BTreeMap::new();
    let mut dropped: Vec<(u64, DropReason)> = Vec::new();
    let mut validated = 0usize;
    let mut converged = false;
    for _ in 0..MAX_ITERS {
        let (kept, n_kept, kept_dropped) = drop_pass(&table, &refs, &mut labels, &known_bad);
        finalize(&mut model, &mut table, &kept);
        // No catch_unwind belt: the generated read is structurally panic-free
        // (codegen emits no panic!/expect/unwrap on input data), so it returns
        // per-entity failures instead of unwinding.
        let mut fails: Vec<(usize, String)> = build_fail
            .iter()
            .filter(|(p, _, _)| kept[*p])
            .map(|(p, _, e)| (*p, e.clone()))
            .collect();
        fails.extend(resolve(&mut model, &table, &refs, &kept));
        dropped = kept_dropped;
        validated = n_kept;
        if fails.is_empty() {
            converged = true;
            break;
        }
        for (pos, e) in fails {
            read_fail.entry(table.rows()[pos].id).or_insert(e);
            known_bad[pos] = true;
        }
    }
    debug_assert!(
        converged,
        "fallible read did not converge in {MAX_ITERS} iters"
    );
    shrink_arenas(&mut model);

    // Finalize provenance: drop-pass drops (this iter) + nonstandard-value
    // (normalize) + accumulated read failures (Unclassified). Each id appears
    // exactly once — `drop_pass` excludes `known_bad` and normalize-dropped
    // entities from its `dropped`, and `read_fail` keys == `known_bad`, so
    // the three sets are disjoint (accounting invariant holds).
    for row in table.rows() {
        if row.member == Member::Rejected {
            dropped.push((
                row.id,
                DropReason {
                    kind: DropKind::NonstandardValue,
                    key: labels.get(row.label).to_string(),
                },
            ));
        }
    }
    for (id, r) in read_fail {
        dropped.push((
            id,
            DropReason {
                kind: DropKind::Unclassified,
                key: r,
            },
        ));
    }
    dropped.sort_by_key(|(id, _)| *id);
    model.header = header;
    Ok((
        model,
        Report {
            n_in,
            n_synth,
            validated,
            dropped,
            norm,
            parse_warnings,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::generated::model::Arena;

    /// `retain_order` keeps exactly the listed items, in the listed order,
    /// whatever cycles the permutation has.
    #[test]
    fn retain_order_keeps_the_listed_items_in_order() {
        let orders: [&[u32]; 7] = [
            &[],
            &[0, 1, 2],
            &[0, 1, 2, 3, 4, 5, 6, 7],
            &[7, 6, 5, 4, 3, 2, 1, 0],
            &[3, 0, 7, 1],
            &[1, 2, 3, 4, 5, 6, 7, 0],
            &[5, 2, 6, 0, 4],
        ];
        for order in orders {
            let mut arena = Arena {
                items: (0..8).map(|i| format!("#{i}")).collect::<Vec<_>>(),
            };
            arena.retain_order(order);
            let want: Vec<String> = order.iter().map(|i| format!("#{i}")).collect();
            assert_eq!(arena.items, want, "order {order:?}");
        }
    }
}
