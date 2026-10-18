//! The reference store: what the reader keeps of each entity's attributes once
//! the entity is built — the slots resolved after the whole file is read, and
//! any other slot holding a reference. Values are kept as flat 16-byte tokens
//! in fixed-size chunks (no large reallocation, no per-value allocation) and
//! rebuilt into [`Attribute`]s only for the moment they are resolved.

use crate::parser::{Attribute, RawEntityPart};

/// One token. An entry is a simple entity's `Part` header, or a complex
/// entity's `Complex` header followed by one `Part` per part; each part's
/// `slots` slots follow it as a `Slot` token and the slot's value. A value
/// is one token, a `List` followed by its items, or a `Typed` followed by its
/// inner value.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Tok {
    Complex(u32),
    /// `pix` = the part's `COMPLEX_PART_NAMES` index (0 for a simple entity);
    /// `attrs` = how many attributes it has.
    Part {
        pix: u16,
        attrs: u32,
        slots: u32,
    },
    Slot(u32),
    Int(i64),
    Real(f64),
    Ref(u64),
    /// The `k`-th synthetic entity, numbered once the file is read.
    Synth(u32),
    Unset,
    Derived,
    /// Text in the string pool, as (offset, length).
    Str(u32, u32),
    Enum(u32, u32),
    Bin(u32, u32),
    List(u32),
    Typed(u32, u32),
}

const _: () = assert!(std::mem::size_of::<Tok>() == 16);

const CHUNK_BITS: usize = 16;
const CHUNK: usize = 1 << CHUNK_BITS;

fn has_ref(a: &Attribute) -> bool {
    match a {
        Attribute::EntityRef(_) => true,
        Attribute::List(l) => l.iter().any(has_ref),
        Attribute::Typed { value, .. } => has_ref(value),
        _ => false,
    }
}

/// Whether slot `i` is kept: it is resolved later, or it holds a reference.
fn kept_slot(fields: &[u16], i: usize, a: &Attribute) -> bool {
    u16::try_from(i).is_ok_and(|i| fields.binary_search(&i).is_ok()) || has_ref(a)
}

#[derive(Default)]
pub(crate) struct RefStore {
    chunks: Vec<Vec<Tok>>,
    len: usize,
    pool: String,
    synth_ids: Vec<u64>,
}

impl RefStore {
    fn push(&mut self, t: Tok) {
        if self.len % CHUNK == 0 {
            self.chunks.push(Vec::with_capacity(CHUNK));
        }
        if let Some(c) = self.chunks.last_mut() {
            c.push(t);
        }
        self.len += 1;
    }

    fn tok(&self, i: usize) -> Tok {
        self.chunks[i >> CHUNK_BITS][i & (CHUNK - 1)]
    }

    fn text(&mut self, s: &str) -> (u32, u32) {
        let at = u32::try_from(self.pool.len()).unwrap_or(u32::MAX);
        self.pool.push_str(s);
        (at, u32::try_from(s.len()).unwrap_or(u32::MAX))
    }

    fn pooled(&self, (at, n): (u32, u32)) -> &str {
        let at = at as usize;
        self.pool.get(at..at + n as usize).unwrap_or("")
    }

    fn start(&self) -> u32 {
        u32::try_from(self.len).unwrap_or(super::table::NONE)
    }

    fn push_value(&mut self, a: &Attribute) {
        let t = match a {
            Attribute::Integer(i) => Tok::Int(*i),
            Attribute::Real(r) => Tok::Real(*r),
            Attribute::String(s) => {
                let (at, n) = self.text(s);
                Tok::Str(at, n)
            }
            Attribute::Enum(s) => {
                let (at, n) = self.text(s);
                Tok::Enum(at, n)
            }
            Attribute::Binary(s) => {
                let (at, n) = self.text(s);
                Tok::Bin(at, n)
            }
            Attribute::EntityRef(id) => Tok::Ref(*id),
            Attribute::Unset => Tok::Unset,
            Attribute::Derived => Tok::Derived,
            Attribute::List(l) => {
                self.push(Tok::List(u32::try_from(l.len()).unwrap_or(u32::MAX)));
                for e in l {
                    self.push_value(e);
                }
                return;
            }
            Attribute::Typed { type_name, value } => {
                let (at, n) = self.text(type_name);
                self.push(Tok::Typed(at, n));
                self.push_value(value);
                return;
            }
        };
        self.push(t);
    }

    fn push_part(
        &mut self,
        pix: u16,
        attrs: &[Attribute],
        fields: &[u16],
        synth: Option<(usize, u32)>,
    ) {
        let kept = attrs
            .iter()
            .enumerate()
            .filter(|&(i, a)| kept_slot(fields, i, a))
            .count();
        self.push(Tok::Part {
            pix,
            attrs: u32::try_from(attrs.len()).unwrap_or(u32::MAX),
            slots: u32::try_from(kept).unwrap_or(u32::MAX),
        });
        for (i, a) in attrs.iter().enumerate() {
            if !kept_slot(fields, i, a) {
                continue;
            }
            self.push(Tok::Slot(u32::try_from(i).unwrap_or(u32::MAX)));
            match synth {
                Some((slot, k)) if slot == i => self.push(Tok::Synth(k)),
                _ => self.push_value(a),
            }
        }
    }

    /// Keep a simple entity's slots: `fields` (sorted) are resolved later;
    /// `synth` = a slot that references the `k`-th synthetic entity. Returns
    /// the entry's start, or `NONE` if nothing is kept.
    pub(crate) fn push_simple(
        &mut self,
        attrs: &[Attribute],
        fields: &[u16],
        synth: Option<(usize, u32)>,
    ) -> u32 {
        if fields.is_empty() && !attrs.iter().any(has_ref) {
            return super::table::NONE;
        }
        let at = self.start();
        self.push_part(0, attrs, fields, synth);
        at
    }

    /// Keep a complex entity's slots, part by part: `parts[i]` has index
    /// `pixs[i]` and resolves `fields(pixs[i])` later.
    pub(crate) fn push_complex(
        &mut self,
        parts: &[RawEntityPart],
        pixs: &[u16],
        fields: impl Fn(u16) -> &'static [u16],
    ) -> u32 {
        let any = parts
            .iter()
            .zip(pixs)
            .any(|(p, &pix)| !fields(pix).is_empty() || p.attributes.iter().any(has_ref));
        if !any {
            return super::table::NONE;
        }
        let at = self.start();
        self.push(Tok::Complex(u32::try_from(parts.len()).unwrap_or(u32::MAX)));
        for (p, &pix) in parts.iter().zip(pixs) {
            self.push_part(pix, &p.attributes, fields(pix), None);
        }
        at
    }

    /// Number the synthetic entities: `ids[k]` is the id of the `k`-th.
    pub(crate) fn set_synth_ids(&mut self, ids: Vec<u64>) {
        self.synth_ids = ids;
    }

    fn synth_id(&self, k: u32) -> u64 {
        self.synth_ids.get(k as usize).copied().unwrap_or(0)
    }

    /// The part headers of the entry at `at`: where the first is, and how
    /// many there are.
    fn parts(&self, at: u32) -> (usize, u32) {
        let at = at as usize;
        match self.tok(at) {
            Tok::Complex(n) => (at + 1, n),
            _ => (at, 1),
        }
    }

    /// Walk the value starting at `i`, passing each reference to `f`; returns
    /// the index after it.
    fn walk_value(&self, mut i: usize, mut f: impl FnMut(u64)) -> usize {
        let mut pending = 1usize;
        while pending > 0 {
            pending -= 1;
            match self.tok(i) {
                Tok::Ref(id) => f(id),
                Tok::Synth(k) => f(self.synth_id(k)),
                Tok::List(n) => pending += n as usize,
                Tok::Typed(..) => pending += 1,
                _ => {}
            }
            i += 1;
        }
        i
    }

    /// Walk the kept slots of the entry at `at` in order, calling
    /// `f(part index, slot position, value start)` until it returns true;
    /// returns whether it did.
    fn each_slot(&self, at: u32, mut f: impl FnMut(u16, u32, usize) -> bool) -> bool {
        if at == super::table::NONE {
            return false;
        }
        let (mut head, n) = self.parts(at);
        for _ in 0..n {
            let Tok::Part { pix, slots, .. } = self.tok(head) else {
                return false;
            };
            let mut i = head + 1;
            for _ in 0..slots {
                let Tok::Slot(idx) = self.tok(i) else {
                    return false;
                };
                if f(pix, idx, i + 1) {
                    return true;
                }
                i = self.walk_value(i + 1, |_| {});
            }
            head = i;
        }
        false
    }

    /// Every reference the entry at `at` holds, in attribute order (`out` is
    /// cleared first).
    pub(crate) fn refs(&self, at: u32, out: &mut Vec<u64>) {
        out.clear();
        if at == super::table::NONE {
            return;
        }
        let (mut i, n) = self.parts(at);
        for _ in 0..n {
            let Tok::Part { slots, .. } = self.tok(i) else {
                return;
            };
            i += 1;
            for _ in 0..slots {
                // Skip the `Slot` token, then the value.
                i = self.walk_value(i + 1, |id| out.push(id));
            }
        }
    }

    /// Call `f(part index, slot position, references)` for each kept slot of
    /// the entry at `at`, in order, until it returns true; returns whether it
    /// did. `buf` holds each slot's references in turn.
    pub(crate) fn slot_refs(
        &self,
        at: u32,
        buf: &mut Vec<u64>,
        mut f: impl FnMut(u16, u32, &[u64]) -> bool,
    ) -> bool {
        self.each_slot(at, |pix, idx, v| {
            buf.clear();
            self.walk_value(v, |id| buf.push(id));
            f(pix, idx, buf)
        })
    }

    fn decode(&self, i: &mut usize) -> Attribute {
        let t = self.tok(*i);
        *i += 1;
        match t {
            Tok::Int(v) => Attribute::Integer(v),
            Tok::Real(v) => Attribute::Real(v),
            Tok::Ref(id) => Attribute::EntityRef(id),
            Tok::Synth(k) => Attribute::EntityRef(self.synth_id(k)),
            Tok::Unset | Tok::Complex(_) | Tok::Part { .. } | Tok::Slot(_) => Attribute::Unset,
            Tok::Derived => Attribute::Derived,
            Tok::Str(at, n) => Attribute::String(self.pooled((at, n)).to_owned()),
            Tok::Enum(at, n) => Attribute::Enum(self.pooled((at, n)).to_owned()),
            Tok::Bin(at, n) => Attribute::Binary(self.pooled((at, n)).to_owned()),
            Tok::List(n) => Attribute::List((0..n).map(|_| self.decode(i)).collect()),
            Tok::Typed(at, n) => Attribute::Typed {
                type_name: self.pooled((at, n)).to_owned(),
                value: Box::new(self.decode(i)),
            },
        }
    }

    /// Rebuild the attributes of the part whose header is at `head`; returns
    /// the index after the part.
    fn restore_part(&self, head: usize, out: &mut Vec<Attribute>) -> usize {
        out.clear();
        let Tok::Part { attrs, slots, .. } = self.tok(head) else {
            return head + 1;
        };
        out.resize(attrs as usize, Attribute::Unset);
        let mut i = head + 1;
        for _ in 0..slots {
            let Tok::Slot(idx) = self.tok(i) else {
                break;
            };
            i += 1;
            let a = self.decode(&mut i);
            if let Some(slot) = out.get_mut(idx as usize) {
                *slot = a;
            }
        }
        i
    }

    /// Rebuild a simple entity's attributes from the entry at `at`: kept slots
    /// as read, the others `$`, the count as in the file.
    pub(crate) fn restore_simple(&self, at: u32, out: &mut Vec<Attribute>) {
        self.restore_part(at as usize, out);
    }

    /// Rebuild a complex entity's parts from the entry at `at`, every part in
    /// its place (the part names are not kept).
    pub(crate) fn restore_complex(&self, at: u32, out: &mut Vec<RawEntityPart>) {
        out.clear();
        let (mut head, n) = self.parts(at);
        for _ in 0..n {
            let mut attributes = Vec::new();
            head = self.restore_part(head, &mut attributes);
            out.push(RawEntityPart {
                name: String::new(),
                attributes,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn attrs(src: &str) -> Vec<Attribute> {
        let g = parse(&format!(
            "ISO-10303-21;HEADER;FILE_DESCRIPTION((''),'2;1');\
             FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;\
             DATA;#1=X({src});ENDSEC;END-ISO-10303-21;"
        ))
        .unwrap();
        match g.entities.into_values().next().unwrap() {
            crate::parser::RawEntity::Simple { attributes, .. } => attributes,
            crate::parser::RawEntity::Complex { .. } => unreachable!(),
        }
    }

    #[test]
    fn keeps_every_kind_of_value_in_a_kept_slot() {
        let a = attrs("#5,(#6,'s',(1,2.5,$,*)),T(.E.),U('x'),\"0F\",-7,'',(),#8");
        let mut store = RefStore::default();
        // Slot 0..4 are resolved later; 8 holds a reference.
        let at = store.push_simple(&a, &[0, 1, 2, 3, 4], None);
        let mut back = Vec::new();
        store.restore_simple(at, &mut back);
        let mut want = a.clone();
        for w in &mut want[5..8] {
            *w = Attribute::Unset;
        }
        assert_eq!(back, want);
        let mut refs = Vec::new();
        store.refs(at, &mut refs);
        assert_eq!(refs, [5, 6, 8]);
    }

    #[test]
    fn keeps_nothing_without_references() {
        let a = attrs("'a',(1.,2.,3.)");
        let mut store = RefStore::default();
        assert_eq!(store.push_simple(&a, &[], None), crate::reader::table::NONE);
    }

    #[test]
    fn keeps_the_attribute_count_of_a_short_entity() {
        let a = attrs("'a'");
        let mut store = RefStore::default();
        let at = store.push_simple(&a, &[1], None);
        let mut back = Vec::new();
        store.restore_simple(at, &mut back);
        assert_eq!(back, [Attribute::Unset]);
    }

    #[test]
    fn numbers_a_synthetic_reference_later() {
        let a = attrs(".N.,#0");
        let mut store = RefStore::default();
        let at = store.push_simple(&a, &[1], Some((1, 0)));
        store.set_synth_ids(vec![42]);
        let mut back = Vec::new();
        store.restore_simple(at, &mut back);
        assert_eq!(back[1], Attribute::EntityRef(42));
    }

    #[test]
    fn restores_complex_parts_in_place() {
        let parts = vec![
            RawEntityPart {
                name: "A".into(),
                attributes: attrs("'no refs'"),
            },
            RawEntityPart {
                name: "B".into(),
                attributes: attrs("1,#3"),
            },
        ];
        let mut store = RefStore::default();
        let at = store.push_complex(&parts, &[0, 1], |pix| if pix == 1 { &[1] } else { &[] });
        let mut back = Vec::new();
        store.restore_complex(at, &mut back);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].attributes, [Attribute::Unset]);
        assert_eq!(
            back[1].attributes,
            [Attribute::Unset, Attribute::EntityRef(3)]
        );
    }

    #[test]
    fn spans_chunks() {
        let a = attrs(&vec!["#1"; 40_000].join(","));
        let mut store = RefStore::default();
        let first = store.push_simple(&a, &[], None);
        let second = store.push_simple(&a, &[], None);
        let mut back = Vec::new();
        store.restore_simple(second, &mut back);
        assert_eq!(back, a);
        store.restore_simple(first, &mut back);
        assert_eq!(back, a);
    }
}
