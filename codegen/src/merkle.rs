//! Order-insensitive (data-equivalence) comparison of two parsed entity maps.
//!
//! The two public entry fns take `&BTreeMap<u64, RawEntity>` directly (the closure
//! subset is filtered into a standalone map rather than carrying a whole
//! `EntityGraph`). The keying is part-order-invariant for complex instances
//! (sorts parts by name) and renumber-invariant (folds referenced keys).

use std::collections::{BTreeMap, HashMap, HashSet};
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
    let memo = entity_keys(g);
    let mut out: HashMap<u64, (String, i64)> = HashMap::new();
    for (&id, ent) in g {
        let k = memo[&id];
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

/// Post-order DFS over the whole map, filling `id -> key` for every entity. An
/// explicit stack (bounded) rather than recursion — required for deep graphs
/// (e.g. tessellated faces) that would overflow a recursive walk. Cycle handling:
/// an on-path ancestor ref hashes a token (self-ref = "SELF", indirect =
/// 0xC0FFEE+id) instead of recursing.
fn entity_keys(g: &Map) -> HashMap<u64, u64> {
    let mut memo: HashMap<u64, u64> = HashMap::new();
    for &start in g.keys() {
        if memo.contains_key(&start) {
            continue;
        }
        let mut stack: Vec<(u64, bool)> = vec![(start, false)];
        let mut on_path: HashSet<u64> = HashSet::new();
        while let Some((id, post)) = stack.pop() {
            if post {
                if !memo.contains_key(&id) {
                    let k = compute_key(id, g, &memo, &on_path);
                    memo.insert(id, k);
                    on_path.remove(&id);
                }
            } else if !memo.contains_key(&id) && !on_path.contains(&id) {
                on_path.insert(id);
                stack.push((id, true));
                // Push children reversed so LIFO pops them in attr order — matches
                // the recursive version's ref visitation, which decides which node
                // in a cycle gets the token vs the memo'd key.
                let mut refs = Vec::new();
                if let Some(ent) = g.get(&id) {
                    ent_refs(ent, &mut refs);
                }
                for &r in refs.iter().rev() {
                    if !memo.contains_key(&r) && !on_path.contains(&r) {
                        stack.push((r, false));
                    }
                }
            }
        }
    }
    memo
}

/// Hash one entity's key from already-computed `memo` (refs) plus cycle tokens for
/// on-path ancestors.
fn compute_key(id: u64, g: &Map, memo: &HashMap<u64, u64>, on_path: &HashSet<u64>) -> u64 {
    let mut h = new_hasher();
    match g.get(&id) {
        Some(RawEntity::Simple {
            name, attributes, ..
        }) => {
            name.hash(&mut h);
            hash_attrs(attributes, &mut h, id, memo, on_path);
        }
        Some(RawEntity::Complex { parts, .. }) => {
            let mut order: Vec<usize> = (0..parts.len()).collect();
            order.sort_by(|&i, &j| parts[i].name.cmp(&parts[j].name));
            for i in order {
                parts[i].name.hash(&mut h);
                hash_attrs(&parts[i].attributes, &mut h, id, memo, on_path);
            }
        }
        None => {
            0xDEADu64.hash(&mut h);
            id.hash(&mut h);
        }
    }
    h.finish()
}

fn hash_attrs(
    attrs: &[Attribute],
    h: &mut impl Hasher,
    cur: u64,
    memo: &HashMap<u64, u64>,
    on_path: &HashSet<u64>,
) {
    (attrs.len() as u64).hash(h);
    for a in attrs {
        hash_attr(a, h, cur, memo, on_path);
    }
}

fn hash_attr(
    a: &Attribute,
    h: &mut impl Hasher,
    cur: u64,
    memo: &HashMap<u64, u64>,
    on_path: &HashSet<u64>,
) {
    match a {
        Attribute::Integer(i) => {
            1u8.hash(h);
            i.hash(h);
        }
        Attribute::Real(r) => {
            2u8.hash(h);
            r.to_bits().hash(h);
        }
        Attribute::String(s) => {
            3u8.hash(h);
            s.hash(h);
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
            ref_key(*n, cur, memo, on_path).hash(h);
        }
        Attribute::Unset => 7u8.hash(h),
        Attribute::Derived => 8u8.hash(h),
        Attribute::List(v) => {
            9u8.hash(h);
            (v.len() as u64).hash(h);
            for e in v {
                hash_attr(e, h, cur, memo, on_path);
            }
        }
        Attribute::Typed { type_name, value } => {
            10u8.hash(h);
            type_name.hash(h);
            hash_attr(value, h, cur, memo, on_path);
        }
    }
}

/// Resolve a ref to its key: memo'd key, else a cycle token for an on-path
/// ancestor (self-ref = "SELF", indirect = 0xC0FFEE+n). Dangling targets are
/// visited by the DFS and memo'd as 0xDEAD+n, so they normally hit the memo
/// branch (the else is a safety fallback).
fn ref_key(n: u64, cur: u64, memo: &HashMap<u64, u64>, on_path: &HashSet<u64>) -> u64 {
    if let Some(&k) = memo.get(&n) {
        return k;
    }
    let mut h = new_hasher();
    if on_path.contains(&n) {
        if n == cur {
            "SELF".hash(&mut h);
        } else {
            0xC0FFEEu64.hash(&mut h);
            n.hash(&mut h);
        }
    } else {
        0xDEADu64.hash(&mut h);
        n.hash(&mut h);
    }
    h.finish()
}

/// Direct ref targets of an entity, in the order compute_key visits them (attr
/// order; Complex parts name-sorted) so the DFS assigns cycle tokens vs memo
/// identically to the recursive form.
fn ent_refs(ent: &RawEntity, out: &mut Vec<u64>) {
    match ent {
        RawEntity::Simple { attributes, .. } => {
            for a in attributes {
                collect_a(a, out);
            }
        }
        RawEntity::Complex { parts, .. } => {
            let mut order: Vec<usize> = (0..parts.len()).collect();
            order.sort_by(|&i, &j| parts[i].name.cmp(&parts[j].name));
            for i in order {
                for a in &parts[i].attributes {
                    collect_a(a, out);
                }
            }
        }
    }
}

fn collect_a(a: &Attribute, out: &mut Vec<u64>) {
    match a {
        Attribute::EntityRef(n) => out.push(*n),
        Attribute::List(v) => v.iter().for_each(|e| collect_a(e, out)),
        Attribute::Typed { value, .. } => collect_a(value, out),
        _ => {}
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
