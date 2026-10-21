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
use std::io::Read;

use crate::generated::generic_normalize;
use crate::generated::model::{StepModel, TextDedup};
use crate::generated::read::{
    COMPLEX_PART_NAMES, RefSlot, SIMPLE_NAMES, build_complex_entity, build_entity,
    complex_part_index, complex_ref_field_slots, complex_ref_slots, finalize_arenas,
    ref_field_slots, ref_slots, resolve_complex_entity, resolve_entity, shrink_arenas,
    simple_index,
};
use crate::parser::p21::Parser;
use crate::parser::{Attribute, Error, ParseWarning, RawEntity, RawEntityPart};

mod entity_normalize;
mod refs;
pub(crate) mod table;

use entity_normalize::Synth;
use refs::RefStore;
use table::{EntityTable, IdMap, Labels, Member, NONE, Row, first_duplicate, sort_rows};

/// Why an entity was dropped before it could enter the model.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum DropKind {
    /// One of the entity's own values is nonstandard and has no standard
    /// rewrite: a required reference is missing, an integer field holds a
    /// fractional value, a list is longer than its type allows.
    NonstandardValue,
    /// The entity type is not one step-io reads: not yet supported, or not a
    /// schema entity at all.
    Unimplemented,
    /// One of its references points at an entity of a type the schema does
    /// not allow there.
    NonstandardReference,
    /// It references an entity that was dropped, or an id the file does not
    /// define.
    Cascade,
    /// Its own values could not be read (a value of the wrong kind, an
    /// unknown enumeration value, too few attributes) for a reason none of
    /// the other kinds names.
    Unclassified,
}

/// `nonstandard value`, `unimplemented`, `nonstandard reference`, `cascade`
/// or `unclassified`.
impl std::fmt::Display for DropKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NonstandardValue => "nonstandard value",
            Self::Unimplemented => "unimplemented",
            Self::NonstandardReference => "nonstandard reference",
            Self::Cascade => "cascade",
            Self::Unclassified => "unclassified",
        })
    }
}

/// Why an entity was dropped: its [`DropKind`] and the detail.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct DropReason {
    pub kind: DropKind,
    /// What the kind is about: the entity type (`Unimplemented`); the rule a
    /// value breaks (`NonstandardValue`); `<ENTITY>: '<attribute>' references
    /// <TYPE>` (`NonstandardReference`); `via-` and the reason of the entity
    /// the cascade starts from, or `via-dangling` for an id the file does not
    /// define (`Cascade`); what could not be read (`Unclassified`).
    pub detail: String,
}

/// `kind: detail`, as in `cascade: via-dangling`.
impl std::fmt::Display for DropReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.detail)
    }
}

/// What reading did with the file's entities. Every entity read, and every
/// entity reading added, is either kept or dropped with a reason:
/// `entities_kept + dropped.len() == entities_read + entities_added`. The
/// source header, with the identified schema, is on the model:
/// [`StepModel::header`].
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Report {
    /// The entities in the file's DATA section.
    pub entities_read: usize,
    /// Entities reading added to give a fixed entity a value it lacked (a
    /// bare `COLOUR` for a missing surface colour).
    pub entities_added: usize,
    /// The entities that entered the model.
    pub entities_kept: usize,
    /// The entities dropped, by id and reason, in id order.
    pub dropped: Vec<(u64, DropReason)>,
    /// What was rewritten to the standard form, by entity id and note
    /// (`missing required text set to empty`), in file order. The entity may
    /// still be dropped for another reason.
    pub normalized: Vec<(u64, &'static str)>,
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
    norm: Vec<(u64, &'static str)>,
    /// The notes of the entity being normalized, before they get its id.
    notes: Vec<&'static str>,
    build_fail: Vec<(u64, String)>,
    synthetic: Vec<Synthetic>,
    max_id: Option<u64>,
    /// Each distinct string of the model, once.
    dedup: TextDedup,
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
            line: ent.span().line,
        };
        // A synthetic entity is added before the generic rules run: it stays
        // in the model even if they then drop the entity that references it.
        let synth = entity_normalize::normalize_entity(&mut ent, &mut self.notes).map(|s| {
            let Synth::Colour { slot } = s;
            let k = to_u32(self.synthetic.len());
            self.add_colour(id);
            (slot, k)
        });
        let generic = generic_normalize::normalize_entity(&mut ent, &mut self.notes);
        self.norm
            .extend(self.notes.drain(..).map(|note| (id, note)));
        if let Err(reason) = generic {
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
                    match build_entity(&mut self.model, ix, &attributes, &mut self.dedup) {
                        Ok(i) => row.arena = i,
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
                    match build_complex_entity(&mut self.model, &parts, &mut self.dedup) {
                        Ok(i) => row.arena = i,
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
            line: 0,
        };
        let mut build = Ok(());
        match simple_index("COLOUR") {
            Some(ix) => {
                row.member = Member::Simple(ix);
                match build_entity(&mut self.model, ix, &[], &mut self.dedup) {
                    Ok(i) => row.arena = i,
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
    fn finish(mut self, end: Result<Vec<ParseWarning>, Error>) -> Result<Scanned, Error> {
        let mut rows = std::mem::take(&mut self.rows);
        sort_rows(&mut rows);
        if let Some((first, repeat)) = first_duplicate(&rows) {
            return Err(Error::DuplicateEntityId {
                id: repeat.id,
                line: repeat.line,
                first_line: first.line,
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
            dedup: self.dedup,
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

/// The streamed source, ready for the drop pass and the resolve step.
struct Scanned {
    dedup: TextDedup,
    model: StepModel,
    table: EntityTable,
    refs: RefStore,
    labels: Labels,
    norm: Vec<(u64, &'static str)>,
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
/// (fixpoint). Returns the kept flags (by table position), how many are set,
/// and the drops this pass recorded.
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
                        detail: labels.get(row.label).to_string(),
                    },
                ));
            }
        }
    }
    let mut ids: Vec<u64> = Vec::new();
    let mut buf: Vec<u64> = Vec::new();
    let mut first = true;
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
                let detail = format!("via-{}", root.map_or("dangling", |l| labels.get(l)));
                let root = root.unwrap_or_else(|| labels.intern("dangling"));
                dropped.push((
                    row.id,
                    DropReason {
                        kind: DropKind::Cascade,
                        detail,
                    },
                ));
                root_of.insert(pos, root);
                removed = true;
                continue;
            }
            // Whether a slot admits its targets' types does not depend on
            // what is kept: an entity that passes in the first round always
            // passes.
            if !first {
                continue;
            }
            if let Some(reason) = nonstd_ref(table, refs, labels, row, &mut buf) {
                keep[pos] = false;
                root_of.insert(pos, labels.intern(&reason));
                dropped.push((
                    row.id,
                    DropReason {
                        kind: DropKind::NonstandardReference,
                        detail: reason,
                    },
                ));
                removed = true;
            }
        }
        first = false;
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
    dedup: &mut TextDedup,
) -> Vec<(usize, String)> {
    let idmap = IdMap::new(table);
    let mut fails = Vec::new();
    let mut attrs: Vec<Attribute> = Vec::new();
    let mut parts: Vec<RawEntityPart> = Vec::new();
    for (pos, row) in table.rows().iter().enumerate() {
        if !kept[pos] || row.arena == NONE || row.refs == NONE {
            continue;
        }
        let i = row.arena;
        let r = match row.member {
            Member::Simple(ix) if !ref_field_slots(ix).is_empty() => {
                refs.restore_simple(row.refs, &mut attrs);
                resolve_entity(model, ix, i, &attrs, &idmap, dedup)
            }
            Member::Complex => {
                refs.restore_complex(row.refs, &mut parts);
                resolve_complex_entity(model, i, &parts, &idmap, dedup)
            }
            _ => continue,
        };
        if let Err(e) = r {
            fails.push((pos, e));
        }
    }
    fails
}

/// Read a STEP file into the schema-faithful model, with a [`Report`] of what
/// was kept, rewritten to the standard form, or dropped and why.
///
/// `input` is read a chunk at a time (no need to wrap it in a `BufReader`;
/// pass `&mut r` to keep `r`), and the file one entity at a time: each is
/// normalized and built into the model as it is parsed, keeping only its
/// references for later. Each entity reads as UTF-8 or ISO 8859-1 as
/// [`parse`](crate::parser::parse) describes. An entity that cannot enter the
/// model is dropped with its reason, and so is every entity that references
/// it; the rest of the file reads as usual.
///
/// # Errors
/// [`Error::Io`] if `input` fails, and another [`Error`] if the file does not
/// parse as Part 21 or is too large to index ([`Error::TooLarge`]). A file
/// that parses is read: a malformed entity is dropped, with its reason in
/// [`Report::dropped`].
pub fn read(input: impl Read) -> Result<(StepModel, Report), Error> {
    // Row positions and arena indices are 32-bit, `NONE` reserved.
    read_at_most(input, NONE as usize)
}

/// [`read`] for a file of at most `limit` entities, synthetic ones
/// included; past that it is [`Error::TooLarge`].
fn read_at_most(input: impl Read, limit: usize) -> Result<(StepModel, Report), Error> {
    /// ≤2 real iterations; one extra for slack (a violation degrades to dropping,
    /// not hanging — `debug_assert` flags non-convergence in dev).
    const MAX_ITERS: usize = 3;

    // The model has no place for the ANCHOR and REFERENCE sections.
    let mut parser = Parser::new(input).discarding_ed3();
    let prelude = parser.read_prelude()?;
    let header = crate::header::from_raw(&prelude.header, prelude.schema);
    let mut scan = Scan::default();
    let end = loop {
        match parser.next_entity() {
            Ok(Some(ent)) => {
                let at = ent.span();
                scan.entity(ent);
                // Like a parse error, this yields to a duplicate id before it.
                if scan.rows.len() + scan.synthetic.len() > limit
                    || scan.refs.overflowed()
                    || scan.model.texts.overflowed()
                {
                    break Err(Error::TooLarge { span: at });
                }
            }
            Ok(None) => break parser.finish(),
            Err(e) => break Err(e),
        }
    };
    let Scanned {
        mut dedup,
        mut model,
        mut table,
        refs,
        mut labels,
        norm,
        build_fail,
        n_in,
        n_synth,
        parse_warnings,
    } = scan.finish(end)?;

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
        fails.extend(resolve(&mut model, &table, &refs, &kept, &mut dedup));
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
                    detail: labels.get(row.label).to_string(),
                },
            ));
        }
    }
    for (id, r) in read_fail {
        dropped.push((
            id,
            DropReason {
                kind: DropKind::Unclassified,
                detail: r,
            },
        ));
    }
    dropped.sort_by_key(|(id, _)| *id);
    model.header = header;
    Ok((
        model,
        Report {
            entities_read: n_in,
            entities_added: n_synth,
            entities_kept: validated,
            dropped,
            normalized: norm,
            parse_warnings,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::generated::model::Arena;
    use crate::parser::Error;

    /// Past the entity limit the read stops with `TooLarge`, at the entity
    /// that went over; a duplicate id before it is still reported first.
    #[test]
    fn too_many_entities_is_an_error() {
        let src = |body: &str| {
            format!(
                "ISO-10303-21;HEADER;FILE_DESCRIPTION((''),'2;1');\
                 FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;\
                 DATA;\n{body}ENDSEC;END-ISO-10303-21;"
            )
        };
        let three = "#1=CARTESIAN_POINT('',(0.,0.,0.));\n#2=CARTESIAN_POINT('',(0.,0.,0.));\n\
                     #3=CARTESIAN_POINT('',(0.,0.,0.));\n";
        assert!(super::read_at_most(src(three).as_bytes(), 3).is_ok());
        match super::read_at_most(src(three).as_bytes(), 2) {
            Err(Error::TooLarge { span }) => assert_eq!(span.line, 4),
            other => panic!("{other:?}"),
        }
        let dup = "#1=CARTESIAN_POINT('',(0.,0.,0.));\n#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
                   #3=CARTESIAN_POINT('',(0.,0.,0.));\n";
        assert!(matches!(
            super::read_at_most(src(dup).as_bytes(), 2),
            Err(Error::DuplicateEntityId { id: 1, .. })
        ));
    }

    /// Each distinct string is stored once and reads back as written;
    /// a `Text` of another model reads as `""`.
    #[test]
    fn texts_store_each_string_once() {
        use crate::generated::model::{StepModel, TextDedup};
        let mut model = StepModel::default();
        let mut dedup = TextDedup::default();
        let words = ["NONE", "", "NONE", "wheel", "", "NONE", "wheel"];
        let texts: Vec<_> = words
            .iter()
            .map(|w| dedup.intern(&mut model.texts, w))
            .collect();
        for (w, &t) in words.iter().zip(&texts) {
            assert_eq!(model.text(t), *w);
        }
        assert_eq!(texts[0], texts[2]);
        assert_eq!(texts[1], texts[4]);
        assert_eq!(texts[3], texts[6]);
        assert_ne!(texts[0], texts[3]);
        let mut other = StepModel::default();
        let far = (0..10)
            .map(|i| other.add_text(&i.to_string()))
            .last()
            .unwrap();
        assert_eq!(model.text(far), "");
    }

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
