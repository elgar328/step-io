//! The generator's intermediate model: the resolved set of arenas, ref-enums,
//! ENUMs and part-bags computed from the schema closure. `main.rs` builds this
//! then emits Rust from it.

use std::collections::{BTreeMap, BTreeSet};

use crate::classify::{Kind, Resolver, pascal};
use crate::schema::{EarlyToml, strip_optional};

/// A resolved attribute field of a simple entity / complex part.
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub kind: Kind,
    pub optional: bool,
    /// `*` (EXPRESS Derived) slot: the positional slot is kept on the wire as
    /// `*` and the field is dropped from the struct.
    pub derived: bool,
    /// The slot is conditionally `*` (some complex cases write a value, some
    /// write `*` — e.g. `named_unit.dimensions`). Modeled as `Option<T>` where
    /// `None` serializes as `*`. The field is KEPT (unlike `derived`).
    pub derivable: bool,
}

/// A simple entity that gets its own arena + typed id.
#[derive(Debug, Clone)]
pub struct SimpleEnt {
    pub name: String, // UPPER STEP keyword
    pub rust: String, // PascalCase struct name
    pub fields: Vec<Field>,
}

/// A complex part entity (variant of the generic part-bag enum).
#[derive(Debug, Clone)]
pub struct PartEnt {
    pub name: String, // UPPER STEP keyword
    pub rust: String, // PascalCase variant name
    pub fields: Vec<Field>,
}

/// A discriminated ref enum over the concrete leaf entities a ref-target can be.
#[derive(Debug, Clone)]
pub struct RefEnum {
    pub rust: String, // e.g. CurveRef
    /// (variant ident, target simple-entity rust name) for simple arms.
    pub simple_arms: Vec<(String, String)>,
    /// True when the target can resolve to a complex part-bag instance.
    pub has_complex: bool,
}

/// A generated Rust enum from an EXPRESS ENUM.
#[derive(Debug, Clone)]
pub struct GenEnum {
    pub rust: String,                   // PascalCase
    pub members: Vec<(String, String)>, // (TOKEN upper, Variant)
}

pub struct ModelIr {
    pub simples: Vec<SimpleEnt>,
    pub parts: Vec<PartEnt>,
    pub ref_enums: BTreeMap<String, RefEnum>, // keyed by ref-target ty name
    pub enums: BTreeMap<String, GenEnum>,     // keyed by ENUM alias
    /// True when any complex part-bag instances exist (units).
    pub has_part_bag: bool,
}

/// Build the schema closure from the domain seeds (fixed point).
///
/// A ref-target is expanded to its FULL subtype tree: a `loop`-typed ref can
/// resolve to any concrete `loop` subtype (edge_loop / poly_loop / vertex_loop),
/// so all of them must be generated for the discriminated ref enum + read
/// dispatch to cover every instance the corpus can write.
pub fn build_closure(schema: &EarlyToml, res: &Resolver, seeds: &[String]) -> BTreeSet<String> {
    let children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut set: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<String> = seeds.to_vec();
    while let Some(name) = stack.pop() {
        if !set.insert(name.clone()) {
            continue;
        }
        let Some(ent) = schema.entity.get(&name) else {
            continue; // synthetic / dangling — nothing to expand
        };
        // parents (supertype attrs flatten into this entity)
        for p in &ent.parents {
            if !set.contains(p) {
                stack.push(p.clone());
            }
        }
        // reachable via flattened attrs (own + inherited): Ref / SELECT / Vec(Ref).
        for a in schema.flattened_attrs(&name) {
            let (_, inner) = strip_optional(&a.ty);
            collect_ref_targets(res, &children, inner, 0, &mut stack);
        }
    }
    set
}

/// Push every entity reachable through a ty's Ref / SELECT / Vec(Ref).
///
/// Ref-targets are added as the named entity / SELECT members only — NOT their
/// full subtype trees (blanket subtype expansion of abstract supertypes like
/// `curve`/`surface` leaks into out-of-domain PMI/maths entities with mixed
/// SELECTs). Concrete subtypes that actually appear in the corpus (e.g.
/// vertex_loop) are listed in the committed domain seeds instead — the seed
/// list IS the scoping mechanism. An instance of an un-seeded subtype panics
/// loudly at read (surfaced, not silently dropped).
fn collect_ref_targets(
    res: &Resolver,
    _children: &BTreeMap<String, Vec<String>>,
    ty: &str,
    depth: u32,
    out: &mut Vec<String>,
) {
    if depth > 64 {
        return;
    }
    let k = res.classify(ty, 0);
    push_kind_targets(res, &k, out);
}

fn push_kind_targets(res: &Resolver, k: &Kind, out: &mut Vec<String>) {
    match k {
        Kind::Ref(target) => {
            for e in concrete_or_self(res, target) {
                out.push(e);
            }
        }
        Kind::Vec(inner) => push_kind_targets(res, inner, out),
        _ => {}
    }
}

/// Expand a ref-target ty (entity OR all-entity SELECT) to the entity names it
/// can name (the SELECT's direct entity members, or the entity itself).
fn concrete_or_self(res: &Resolver, target: &str) -> Vec<String> {
    if res.schema.entity.contains_key(target) {
        return vec![target.to_string()];
    }
    // SELECT alias: gather member entity names (recursing through nested selects).
    let mut out = Vec::new();
    select_entity_members(res, target, &mut out, 0);
    out
}

fn select_entity_members(res: &Resolver, alias: &str, out: &mut Vec<String>, depth: u32) {
    if depth > 32 {
        return;
    }
    let Some(td) = res.schema.types.get(alias) else {
        if res.schema.entity.contains_key(alias) {
            out.push(alias.to_string());
        }
        return;
    };
    if let Some(members) = crate::schema::select_members(td.aliased.trim()) {
        for m in members {
            if res.schema.entity.contains_key(m) {
                out.push(m.to_string());
            } else {
                select_entity_members(res, m, out, depth + 1);
            }
        }
    }
}

/// All closure entities a ref-target ty can name. The target is either an
/// entity (-> itself + transitive subtypes) or an all-entity SELECT alias (->
/// each member's entities + their subtypes). Result is intersected with the
/// closure (so only generated entities become arms).
fn subtypes_in_closure(
    schema: &EarlyToml,
    res: &Resolver,
    closure: &BTreeSet<String>,
    children: &BTreeMap<String, Vec<String>>,
    target: &str,
) -> BTreeSet<String> {
    // Seed roots: the target entity, or the SELECT alias's member entities.
    let mut roots: Vec<String> = Vec::new();
    if schema.entity.contains_key(target) {
        roots.push(target.to_string());
    } else {
        select_entity_members(res, target, &mut roots, 0);
    }
    let mut out = BTreeSet::new();
    let mut stack = roots;
    while let Some(n) = stack.pop() {
        if !out.insert(n.clone()) {
            continue;
        }
        if let Some(kids) = children.get(&n) {
            for k in kids {
                stack.push(k.clone());
            }
        }
    }
    out.intersection(closure).cloned().collect()
}

/// Invert `parents` into a child-list map.
pub fn build_children(schema: &EarlyToml) -> BTreeMap<String, Vec<String>> {
    let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, ent) in &schema.entity {
        for p in &ent.parents {
            m.entry(p.clone()).or_default().push(name.clone());
        }
    }
    m
}

impl ModelIr {
    /// Build the full IR from the closure.
    pub fn build(
        schema: &EarlyToml,
        res: &Resolver,
        closure: &BTreeSet<String>,
        complex_parts: &BTreeSet<String>,
        derived: &BTreeMap<String, Vec<String>>,
        derivable: &BTreeMap<String, BTreeSet<String>>,
    ) -> ModelIr {
        let children = build_children(schema);
        let has_part_bag = !complex_parts.is_empty();

        // Simple entities = closure entities that are NOT complex-only parts and
        // are concrete enough to carry an instance. We emit an arena for every
        // such closure entity (unused ones become empty arenas, harmless).
        let mut simples: Vec<SimpleEnt> = Vec::new();
        for name in closure {
            if complex_parts.contains(name) {
                continue;
            }
            // Skip pure synthetic / non-entity names.
            if !schema.entity.contains_key(name) {
                continue;
            }
            let empty = BTreeSet::new();
            let fields = build_fields(
                schema,
                res,
                name,
                derived,
                derivable.get(name).unwrap_or(&empty),
            );
            simples.push(SimpleEnt {
                name: name.to_uppercase(),
                rust: pascal(name),
                fields,
            });
        }

        // Complex part entities -> part-bag variants (own_attrs only).
        let mut parts: Vec<PartEnt> = Vec::new();
        for name in complex_parts {
            let empty = BTreeSet::new();
            let fields = build_own_fields(
                schema,
                res,
                name,
                derived,
                derivable.get(name).unwrap_or(&empty),
            );
            parts.push(PartEnt {
                name: name.to_uppercase(),
                rust: pascal(name),
                fields,
            });
        }

        // Discriminated ref enums for every Ref(target) used by an emitted field.
        let mut ref_enums: BTreeMap<String, RefEnum> = BTreeMap::new();
        let mut enums: BTreeMap<String, GenEnum> = BTreeMap::new();
        let mut want_refs: BTreeSet<String> = BTreeSet::new();
        let mut want_enums: BTreeSet<String> = BTreeSet::new();
        for s in &simples {
            for f in &s.fields {
                collect_field_wants(&f.kind, &mut want_refs, &mut want_enums);
            }
        }
        for p in &parts {
            for f in &p.fields {
                collect_field_wants(&f.kind, &mut want_refs, &mut want_enums);
            }
        }

        for target in &want_refs {
            let leaves = subtypes_in_closure(schema, res, closure, &children, target);
            let mut simple_arms: Vec<(String, String)> = Vec::new();
            let mut has_complex = false;
            for leaf in &leaves {
                if complex_parts.contains(leaf) {
                    has_complex = true;
                } else if schema.entity.contains_key(leaf) {
                    simple_arms.push((pascal(leaf), pascal(leaf)));
                }
            }
            simple_arms.sort();
            simple_arms.dedup();
            ref_enums.insert(
                target.clone(),
                RefEnum {
                    rust: format!("{}Ref", pascal(target)),
                    simple_arms,
                    has_complex,
                },
            );
        }

        for alias in &want_enums {
            if let Some(ge) = build_enum(res, alias) {
                enums.insert(alias.clone(), ge);
            }
        }

        ModelIr {
            simples,
            parts,
            ref_enums,
            enums,
            has_part_bag,
        }
    }
}

fn collect_field_wants(k: &Kind, refs: &mut BTreeSet<String>, enums: &mut BTreeSet<String>) {
    match k {
        Kind::Ref(t) => {
            refs.insert(t.clone());
        }
        Kind::Enum(a) => {
            enums.insert(a.clone());
        }
        Kind::Vec(inner) => collect_field_wants(inner, refs, enums),
        _ => {}
    }
}

fn build_fields(
    schema: &EarlyToml,
    res: &Resolver,
    name: &str,
    derived: &BTreeMap<String, Vec<String>>,
    derivable: &BTreeSet<String>,
) -> Vec<Field> {
    let dset = derived_set(schema, derived, name, /*flattened=*/ true);
    schema
        .flattened_attrs(name)
        .iter()
        .map(|a| make_field(res, a, &dset, derivable))
        .collect()
}

fn build_own_fields(
    schema: &EarlyToml,
    res: &Resolver,
    name: &str,
    derived: &BTreeMap<String, Vec<String>>,
    derivable: &BTreeSet<String>,
) -> Vec<Field> {
    let dset = derived_set(schema, derived, name, /*flattened=*/ false);
    schema
        .own_attrs(name)
        .iter()
        .map(|a| make_field(res, a, &dset, derivable))
        .collect()
}

/// Collect the derived (`*`) attr names that apply to an entity. For flattened
/// (simple) entities, also pull in ancestors' derived hints.
fn derived_set(
    schema: &EarlyToml,
    derived: &BTreeMap<String, Vec<String>>,
    name: &str,
    flattened: bool,
) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    if let Some(ds) = derived.get(name) {
        out.extend(ds.iter().cloned());
    }
    if flattened {
        // walk ancestors
        let mut stack = vec![name.to_string()];
        let mut seen = BTreeSet::new();
        while let Some(n) = stack.pop() {
            if !seen.insert(n.clone()) {
                continue;
            }
            if let Some(ds) = derived.get(&n) {
                out.extend(ds.iter().cloned());
            }
            if let Some(ent) = schema.entity.get(&n) {
                for p in &ent.parents {
                    stack.push(p.clone());
                }
            }
        }
    }
    out
}

fn make_field(
    res: &Resolver,
    a: &crate::schema::Attr,
    dset: &BTreeSet<String>,
    derivable: &BTreeSet<String>,
) -> Field {
    let (optional, kind) = res.classify_attr(&a.ty);
    // A flat `[derived]` entry hard-drops the slot; a complex-case `derived`
    // entry makes it conditionally `*` (kept as Option). The latter only
    // applies when not already a hard drop.
    let hard = dset.contains(&a.name);
    Field {
        name: a.name.clone(),
        kind,
        optional,
        derived: hard,
        derivable: !hard && derivable.contains(&a.name),
    }
}

fn build_enum(res: &Resolver, alias: &str) -> Option<GenEnum> {
    let td = res.schema.types.get(alias)?;
    let members = crate::schema::enum_members(td.aliased.trim())?;
    Some(GenEnum {
        rust: pascal(alias),
        members: members
            .iter()
            .map(|m| (m.to_uppercase(), pascal(m)))
            .collect(),
    })
}
