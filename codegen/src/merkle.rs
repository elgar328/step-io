//! Order-insensitive (data-equivalence) comparison of two parsed entity maps.
//!
//! Copied from step-io-reference-check `src/canonical.rs`, with the two public
//! entry fns adapted to take `&BTreeMap<u64, RawEntity>` directly (this spike
//! filters the units subset into a standalone map rather than carrying a whole
//! `EntityGraph`). The keying is part-order-invariant for complex instances
//! (sorts parts by name) and renumber-invariant (folds referenced keys).

use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

use step_io::{Attribute, RawEntity};

type Map = BTreeMap<u64, RawEntity>;

/// Core data-equivalence check over two entity maps.
/// Returns `None` when equivalent, or `Some(reason)` describing the divergence.
pub fn merkle_diff_maps(ga: &Map, gb: &Map) -> Option<String> {
    let ka = key_multiset(ga);
    let kb = key_multiset(gb);
    multiset_diff(&ka, &kb)
}

/// `key -> (type_name, count)` over the entities of one map.
fn key_multiset(g: &Map) -> HashMap<u64, (String, i64)> {
    let mut memo: HashMap<u64, u64> = HashMap::new();
    let mut in_progress: Vec<u64> = Vec::new();
    let mut out: HashMap<u64, (String, i64)> = HashMap::new();
    for (&id, ent) in g {
        let k = entity_key(id, g, &mut memo, &mut in_progress);
        out.entry(k).or_insert_with(|| (type_of(ent), 0)).1 += 1;
    }
    out
}

fn type_of(ent: &RawEntity) -> String {
    match ent {
        RawEntity::Simple { name, .. } => name.clone(),
        RawEntity::Complex { parts, .. } => {
            let mut names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
            names.sort_unstable();
            format!("({})", names.join("+"))
        }
    }
}

fn entity_key(id: u64, g: &Map, memo: &mut HashMap<u64, u64>, in_progress: &mut Vec<u64>) -> u64 {
    if let Some(&k) = memo.get(&id) {
        return k;
    }
    if in_progress.contains(&id) {
        let mut h = new_hasher();
        if in_progress.last() == Some(&id) {
            "SELF".hash(&mut h);
        } else {
            0xC0FFEEu64.hash(&mut h);
            id.hash(&mut h);
        }
        return h.finish();
    }
    in_progress.push(id);
    let mut h = new_hasher();
    match g.get(&id) {
        Some(RawEntity::Simple {
            name, attributes, ..
        }) => {
            name.hash(&mut h);
            hash_attrs(attributes, &mut h, g, memo, in_progress);
        }
        Some(RawEntity::Complex { parts, .. }) => {
            let mut order: Vec<usize> = (0..parts.len()).collect();
            order.sort_by(|&i, &j| parts[i].name.cmp(&parts[j].name));
            for i in order {
                parts[i].name.hash(&mut h);
                hash_attrs(&parts[i].attributes, &mut h, g, memo, in_progress);
            }
        }
        None => {
            0xDEADu64.hash(&mut h);
            id.hash(&mut h);
        }
    }
    let k = h.finish();
    in_progress.pop();
    memo.insert(id, k);
    k
}

fn hash_attrs(
    attrs: &[Attribute],
    h: &mut impl Hasher,
    g: &Map,
    memo: &mut HashMap<u64, u64>,
    in_progress: &mut Vec<u64>,
) {
    (attrs.len() as u64).hash(h);
    for a in attrs {
        hash_attr(a, h, g, memo, in_progress);
    }
}

fn hash_attr(
    a: &Attribute,
    h: &mut impl Hasher,
    g: &Map,
    memo: &mut HashMap<u64, u64>,
    in_progress: &mut Vec<u64>,
) {
    match a {
        Attribute::Integer(i) => {
            1u8.hash(h);
            i.hash(h);
        }
        Attribute::Real(r) => {
            2u8.hash(h);
            // Signed-zero canonical: -0.0 and +0.0 are the same value/point, so
            // `-0.` (raw input) and `0.` (normalized output) compare equal.
            (r + 0.0).to_bits().hash(h);
        }
        Attribute::String(s) => {
            // Modernizer NORM: a required string supplied as `$` (non-standard)
            // is normalized to `''` on output. Treat empty-string and Unset as
            // equivalent so that NORM does not register as a round-trip loss.
            if s.is_empty() {
                7u8.hash(h);
            } else {
                3u8.hash(h);
                s.hash(h);
            }
        }
        Attribute::Enum(s) => {
            4u8.hash(h);
            s.hash(h);
        }
        Attribute::Binary(s) => {
            5u8.hash(h);
            s.hash(h);
        }
        Attribute::EntityRef(n) => {
            6u8.hash(h);
            entity_key(*n, g, memo, in_progress).hash(h);
        }
        Attribute::Unset => 7u8.hash(h),
        Attribute::Derived => 8u8.hash(h),
        Attribute::List(v) => {
            // Modernizer NORM: a derived (`*`) field supplied as `()` (a
            // non-standard empty set, e.g. occt oriented_closed_shell.cfs_faces)
            // is normalized to `*`. Treat an empty list and Derived as equal.
            if v.is_empty() {
                8u8.hash(h);
                return;
            }
            9u8.hash(h);
            (v.len() as u64).hash(h);
            for e in v {
                hash_attr(e, h, g, memo, in_progress);
            }
        }
        Attribute::Typed { type_name, value } => {
            10u8.hash(h);
            type_name.hash(h);
            hash_attr(value, h, g, memo, in_progress);
        }
    }
}

fn new_hasher() -> std::collections::hash_map::DefaultHasher {
    std::collections::hash_map::DefaultHasher::new()
}

fn multiset_diff(
    a: &HashMap<u64, (String, i64)>,
    b: &HashMap<u64, (String, i64)>,
) -> Option<String> {
    let mut left: HashMap<String, i64> = HashMap::new();
    let mut right: HashMap<String, i64> = HashMap::new();
    for (k, (ty, ca)) in a {
        let cb = b.get(k).map_or(0, |(_, c)| *c);
        if *ca > cb {
            *left.entry(ty.clone()).or_insert(0) += ca - cb;
        }
    }
    for (k, (ty, cb)) in b {
        let ca = a.get(k).map_or(0, |(_, c)| *c);
        if *cb > ca {
            *right.entry(ty.clone()).or_insert(0) += cb - ca;
        }
    }
    if left.is_empty() && right.is_empty() {
        return None;
    }
    let mut types: Vec<String> = left.keys().chain(right.keys()).cloned().collect();
    types.sort_unstable();
    types.dedup();
    types
        .sort_by_key(|t| -(left.get(t).copied().unwrap_or(0) + right.get(t).copied().unwrap_or(0)));
    let detail = types
        .iter()
        .take(8)
        .map(|t| {
            format!(
                "{t}(left:{},right:{})",
                left.get(t).copied().unwrap_or(0),
                right.get(t).copied().unwrap_or(0)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "data-equivalence mismatch [left=a-only,right=b-only]: {detail}"
    ))
}
