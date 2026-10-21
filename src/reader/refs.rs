//! The reference store: what the reader keeps of each entity's attributes once
//! the entity is built — the slots resolved after the whole file is read, and
//! any other slot holding a reference. Values are kept as tokens of one to
//! three 32-bit words in fixed-size chunks (no large reallocation, no
//! per-value allocation) and rebuilt into [`Attribute`]s only for the moment
//! they are resolved.

use crate::parser::{Attribute, RawEntityPart};

/// One token, as read back. An entry is a simple entity's `Part` header, or
/// a complex entity's `Complex` header followed by one `Part` per part; each
/// part's `slots` slots follow it as a `Slot` token and the slot's value. A
/// value is one token, a `List` followed by its items, or a `Typed` followed
/// by its inner value.
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

const CHUNK_BITS: usize = 16;
const CHUNK: usize = 1 << CHUNK_BITS;

// A token's first word holds its kind in the top four bits and a value in
// the other 28. A value that does not fit is `WIDE` there and follows as two
// more words, low half first; so do an `Int`'s and a `Real`'s 64 bits. A
// `Part`'s first word holds `pix`, its next two `attrs` and `slots`; a
// pooled text's hold `(offset, length)`.
const KIND_SHIFT: u32 = 28;
const VALUE: u32 = (1 << KIND_SHIFT) - 1;
const WIDE: u32 = VALUE;

const K_COMPLEX: u32 = 0;
const K_PART: u32 = 1;
const K_SLOT: u32 = 2;
const K_INT: u32 = 3;
const K_REAL: u32 = 4;
const K_REF: u32 = 5;
const K_SYNTH: u32 = 6;
const K_UNSET: u32 = 7;
const K_DERIVED: u32 = 8;
const K_STR: u32 = 9;
const K_ENUM: u32 = 10;
const K_BIN: u32 = 11;
const K_LIST: u32 = 12;
const K_TYPED: u32 = 13;

fn lo_hi(v: u64) -> (u32, u32) {
    (narrow(v & u64::from(u32::MAX)), narrow(v >> 32))
}

fn narrow(v: u64) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

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
    chunks: Vec<Vec<u32>>,
    len: usize,
    /// A position or count no longer fit in 32 bits.
    overflow: bool,
    pool: String,
    synth_ids: Vec<u64>,
}

impl RefStore {
    fn word(&mut self, w: u32) {
        if self.len % CHUNK == 0 {
            self.chunks.push(Vec::with_capacity(CHUNK));
        }
        if let Some(c) = self.chunks.last_mut() {
            c.push(w);
        }
        self.len += 1;
    }

    /// A token of `kind` holding `v`: one word, or three if `v` is wide.
    fn push_n(&mut self, kind: u32, v: u64) {
        match u32::try_from(v) {
            Ok(v) if v < WIDE => self.word(kind << KIND_SHIFT | v),
            _ => {
                self.word(kind << KIND_SHIFT | WIDE);
                self.push_64(v);
            }
        }
    }

    fn push_64(&mut self, v: u64) {
        let (lo, hi) = lo_hi(v);
        self.word(lo);
        self.word(hi);
    }

    fn push(&mut self, t: Tok) {
        match t {
            Tok::Complex(n) => self.push_n(K_COMPLEX, n.into()),
            Tok::Part { pix, attrs, slots } => {
                self.word(K_PART << KIND_SHIFT | u32::from(pix));
                self.word(attrs);
                self.word(slots);
            }
            Tok::Slot(i) => self.push_n(K_SLOT, i.into()),
            Tok::Int(v) => {
                self.word(K_INT << KIND_SHIFT);
                self.push_64(u64::from_le_bytes(v.to_le_bytes()));
            }
            Tok::Real(v) => {
                self.word(K_REAL << KIND_SHIFT);
                self.push_64(v.to_bits());
            }
            Tok::Ref(id) => self.push_n(K_REF, id),
            Tok::Synth(k) => self.push_n(K_SYNTH, k.into()),
            Tok::Unset => self.word(K_UNSET << KIND_SHIFT),
            Tok::Derived => self.word(K_DERIVED << KIND_SHIFT),
            Tok::Str(at, n) => self.push_pair(K_STR, at, n),
            Tok::Enum(at, n) => self.push_pair(K_ENUM, at, n),
            Tok::Bin(at, n) => self.push_pair(K_BIN, at, n),
            Tok::List(n) => self.push_n(K_LIST, n.into()),
            Tok::Typed(at, n) => self.push_pair(K_TYPED, at, n),
        }
    }

    fn push_pair(&mut self, kind: u32, a: u32, b: u32) {
        self.word(kind << KIND_SHIFT);
        self.word(a);
        self.word(b);
    }

    #[inline]
    fn word_at(&self, i: usize) -> u32 {
        self.chunks[i >> CHUNK_BITS][i & (CHUNK - 1)]
    }

    #[inline]
    fn u64_at(&self, i: usize) -> u64 {
        u64::from(self.word_at(i)) | u64::from(self.word_at(i + 1)) << 32
    }

    /// The word after the `push_n` token at `at`.
    #[inline]
    fn after_n(&self, at: usize) -> usize {
        if self.word_at(at) & VALUE < WIDE {
            at + 1
        } else {
            at + 3
        }
    }

    /// The token at word `at`, and the word after it.
    #[inline]
    fn tok(&self, at: usize) -> (Tok, usize) {
        let head = self.word_at(at);
        let value = head & VALUE;
        // A `push_n` token's value, and the word after it.
        let (n, after) = if value < WIDE {
            (u64::from(value), at + 1)
        } else {
            (self.u64_at(at + 1), at + 3)
        };
        match head >> KIND_SHIFT {
            K_REF => (Tok::Ref(n), after),
            K_SLOT => (Tok::Slot(narrow(n)), after),
            K_LIST => (Tok::List(narrow(n)), after),
            K_COMPLEX => (Tok::Complex(narrow(n)), after),
            K_SYNTH => (Tok::Synth(narrow(n)), after),
            K_PART => {
                let part = Tok::Part {
                    pix: u16::try_from(value).unwrap_or(u16::MAX),
                    attrs: self.word_at(at + 1),
                    slots: self.word_at(at + 2),
                };
                (part, at + 3)
            }
            K_INT => {
                let v = i64::from_le_bytes(self.u64_at(at + 1).to_le_bytes());
                (Tok::Int(v), at + 3)
            }
            K_REAL => (Tok::Real(f64::from_bits(self.u64_at(at + 1))), at + 3),
            K_DERIVED => (Tok::Derived, at + 1),
            kind @ (K_STR | K_ENUM | K_BIN | K_TYPED) => {
                let (off, len) = (self.word_at(at + 1), self.word_at(at + 2));
                let text = match kind {
                    K_STR => Tok::Str(off, len),
                    K_ENUM => Tok::Enum(off, len),
                    K_BIN => Tok::Bin(off, len),
                    _ => Tok::Typed(off, len),
                };
                (text, at + 3)
            }
            _ => (Tok::Unset, at + 1),
        }
    }

    /// `n` as a token field, noting when it does not fit.
    fn fit(&mut self, n: usize) -> u32 {
        u32::try_from(n).unwrap_or_else(|_| {
            self.overflow = true;
            u32::MAX
        })
    }

    /// Whether a position or count has outgrown the 32-bit token fields;
    /// what was stored since is then unusable.
    pub(crate) fn overflowed(&self) -> bool {
        self.overflow || self.len >= super::table::NONE as usize
    }

    fn text(&mut self, s: &str) -> (u32, u32) {
        let at = self.fit(self.pool.len());
        self.pool.push_str(s);
        (at, self.fit(s.len()))
    }

    fn pooled(&self, (at, n): (u32, u32)) -> &str {
        let at = at as usize;
        self.pool.get(at..at + n as usize).unwrap_or("")
    }

    fn start(&mut self) -> u32 {
        self.fit(self.len)
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
                let n = self.fit(l.len());
                self.push(Tok::List(n));
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
        let (attrs_n, slots) = (self.fit(attrs.len()), self.fit(kept));
        self.push(Tok::Part {
            pix,
            attrs: attrs_n,
            slots,
        });
        for (i, a) in attrs.iter().enumerate() {
            if !kept_slot(fields, i, a) {
                continue;
            }
            let idx = self.fit(i);
            self.push(Tok::Slot(idx));
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
        let n = self.fit(parts.len());
        self.push(Tok::Complex(n));
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
            (Tok::Complex(n), next) => (next, n),
            _ => (at, 1),
        }
    }

    /// Walk the value starting at `i`, passing each reference to `f`; returns
    /// the index after it.
    fn walk_value(&self, mut i: usize, mut f: impl FnMut(u64)) -> usize {
        let mut pending = 1usize;
        while pending > 0 {
            pending -= 1;
            // Most values are a reference that fits in its first word.
            let head = self.word_at(i);
            if head >> KIND_SHIFT == K_REF && head & VALUE < WIDE {
                f(u64::from(head & VALUE));
                i += 1;
                continue;
            }
            let (t, next) = self.tok(i);
            match t {
                Tok::Ref(id) => f(id),
                Tok::Synth(k) => f(self.synth_id(k)),
                Tok::List(n) => pending += n as usize,
                Tok::Typed(..) => pending += 1,
                _ => {}
            }
            i = next;
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
            let (Tok::Part { pix, slots, .. }, mut i) = self.tok(head) else {
                return false;
            };
            for _ in 0..slots {
                let (Tok::Slot(idx), value) = self.tok(i) else {
                    return false;
                };
                if f(pix, idx, value) {
                    return true;
                }
                i = self.walk_value(value, |_| {});
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
            let (Tok::Part { slots, .. }, next) = self.tok(i) else {
                return;
            };
            i = next;
            for _ in 0..slots {
                // Skip the `Slot` token, then the value.
                i = self.walk_value(self.after_n(i), |id| out.push(id));
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
        let (t, next) = self.tok(*i);
        *i = next;
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
        let (Tok::Part { attrs, slots, .. }, mut i) = self.tok(head) else {
            return head + 1;
        };
        out.resize(attrs as usize, Attribute::Unset);
        for _ in 0..slots {
            let (Tok::Slot(idx), value) = self.tok(i) else {
                break;
            };
            i = value;
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
        let g = parse(
            format!(
                "ISO-10303-21;HEADER;FILE_DESCRIPTION((''),'2;1');\
                 FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;\
                 DATA;#1=X({src});ENDSEC;END-ISO-10303-21;"
            )
            .as_bytes(),
        )
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
    fn keeps_wide_values() {
        let a = attrs(
            "#268435455,#268435456,#4294967296,#18446744073709551615,\
             (-9223372036854775808,1.5E300,#1)",
        );
        let mut store = RefStore::default();
        let at = store.push_simple(&a, &[4], None);
        let mut back = Vec::new();
        store.restore_simple(at, &mut back);
        assert_eq!(back, a);
        let mut refs = Vec::new();
        store.refs(at, &mut refs);
        assert_eq!(refs, [268_435_455, 268_435_456, 4_294_967_296, u64::MAX, 1]);
    }

    #[test]
    fn values_one_word_up_to_the_wide_mark() {
        let mut store = RefStore::default();
        for (n, words) in [(0, 1), (WIDE - 1, 1), (WIDE, 3), (u32::MAX, 3)] {
            let at = store.len;
            store.push(Tok::List(n));
            assert_eq!(store.tok(at), (Tok::List(n), at + words), "{n:#x}");
        }
        let at = store.len;
        store.push(Tok::Ref(u64::MAX));
        assert_eq!(store.tok(at), (Tok::Ref(u64::MAX), at + 3));
    }

    #[test]
    fn tokens_cross_a_chunk_boundary() {
        // A first entry of `fill` references in one slot, ending `gap` words
        // before the chunk boundary: Part (3) + Slot + List + the references.
        // The second entry's Part (3 words) crosses it at a gap of 1 or 2,
        // its Int (3 words, after Part and Slot) at 5 or 6.
        for gap in [1, 2, 5, 6] {
            let fill = CHUNK - gap - 5;
            let first = attrs(&format!("({})", vec!["#1"; fill].join(",")));
            let second = attrs("-7,1.5E300,#5,'kept'");
            let mut store = RefStore::default();
            store.push_simple(&first, &[], None);
            assert_eq!(store.len, CHUNK - gap);
            let at = store.push_simple(&second, &[0, 1, 3], None);
            let mut back = Vec::new();
            store.restore_simple(at, &mut back);
            assert_eq!(back, second, "gap {gap}");
            let mut refs = Vec::new();
            store.refs(at, &mut refs);
            assert_eq!(refs, [5], "gap {gap}");
        }
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
