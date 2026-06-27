//! The generator's own intermediate representation: the resolved set of arenas,
//! ref-enums, ENUMs and part-bags computed from the schema closure. `main.rs`
//! builds this, then `emit` writes Rust (incl. the runtime `StepModel`) from it.
//! (This is the code generator's IR — unrelated to any runtime data model.)

use std::collections::{BTreeMap, BTreeSet};

use crate::classify::{Kind, Resolver, pascal};
use crate::schema::{Schema, strip_optional};

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
    /// Aggregate arms for `SELECT(.., named_list_type, ..)`: one per aggregate
    /// MEMBER — (variant ident, element ref-enum rust name e.g.
    /// `DatumReferenceElementRef`, UPPER wire type name e.g. `COMMON_DATUM_LIST`).
    /// The arm holds `Vec<{element}>`; on the wire a named aggregate select member
    /// is a Typed literal `WIRE((#a,#b))`, so the wire name is kept for read
    /// discrimination + write. A non-empty list makes this enum non-Copy.
    pub aggregate: Vec<(String, String, String)>,
    /// Enum arms for `SELECT(.., some_enum)`: (variant ident, generated enum rust
    /// name, UPPER wire type name e.g. `SIMPLE_DATUM_REFERENCE_MODIFIER`). A named
    /// enum select member is a Typed literal `WIRE(.TOKEN.)`, so the wire name is
    /// kept for read discrimination + write. The arm holds the (Copy) enum.
    pub enum_arms: Vec<(String, String, String)>,
    /// Scalar arms for `SELECT(.., parameter_value)`: (variant ident, scalar Kind
    /// [Real/Str], UPPER wire name). The arm payload is the bare scalar (`f64` /
    /// `String`) — the standard form is the Typed literal `WIRE(value)`, so write
    /// always emits typed via the wire name; read accepts both the typed and the
    /// (non-standard) bare form and keeps only the value (form is normalized).
    pub scalar_arms: Vec<(String, Kind, String)>,
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
/// dispatch to cover every instance real-world input can write.
pub fn build_closure(
    schema: &Schema,
    res: &Resolver,
    seeds: &[String],
    exclude: &BTreeSet<String>,
) -> BTreeSet<String> {
    let children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut set: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<String> = seeds.to_vec();
    while let Some(name) = stack.pop() {
        // Excluded entities never enter the closure (not generated); referrers
        // escape via the round-trip filter. Deferral knob, not a model bend.
        if exclude.contains(&name) {
            continue;
        }
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
/// SELECTs). Concrete subtypes that actually appear in real-world input (e.g.
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
    if !res.schema.types.contains_key(alias) {
        if res.schema.entity.contains_key(alias) {
            out.push(alias.to_string());
        }
        return;
    }
    if let Some(members) = res.select_members(alias) {
        for m in members {
            if res.schema.entity.contains_key(m) {
                out.push(m.to_string());
            } else if let Some(e) = res.agg_element(m) {
                // aggregate member (LIST/SET OF E) -> pull E's entities so the
                // aggregate arm's element ref-enum is generated and non-empty.
                if res.schema.entity.contains_key(&e) {
                    out.push(e);
                } else {
                    select_entity_members(res, &e, out, depth + 1);
                }
            } else {
                select_entity_members(res, m, out, depth + 1);
            }
        }
    }
}

/// Aggregate arms of a ref-target: for a SELECT with `LIST/SET OF E` members,
/// one arm per distinct element E -> (variant ident, element ref-enum rust name).
/// Empty for entities and non-aggregate selects.
fn aggregate_arms(res: &Resolver, target: &str) -> Vec<(String, String, String)> {
    let Some(members) = res.select_members(target) else {
        return Vec::new();
    };
    // One arm per aggregate MEMBER (named list/set types are wire-distinct via
    // their Typed literal name, so do NOT collapse by element type).
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for m in members {
        if let Some(e) = res.agg_element(m)
            && res.ref_like(&e, 0)
            && seen.insert(m.to_string())
        {
            out.push((pascal(m), format!("{}Ref", pascal(&e)), m.to_uppercase()));
        }
    }
    out
}

/// Scalar arms of a ref-target: for a SELECT with named-scalar members, one arm
/// per member -> (variant ident, scalar Kind, UPPER wire name). Empty otherwise.
fn scalar_arms(res: &Resolver, target: &str) -> Vec<(String, Kind, String)> {
    let Some(members) = res.select_members(target) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for m in members {
        if res.is_scalar_member(m) {
            // (variant ident, scalar Kind, UPPER wire name) — same wire rule as
            // aggregate_arms. A named scalar select member is written as a Typed
            // literal `WIRE(value)` (the standard form); the bare form `value` is
            // accepted on read but normalized to typed on write.
            out.push((pascal(m), res.classify(m, 0), m.to_uppercase()));
        }
    }
    out
}

/// Enum arms of a ref-target: for a SELECT with ENUM members, one arm per
/// distinct enum alias -> (variant ident, generated enum rust name). The enum
/// alias is also returned so the caller can add it to `want_enums`. Empty for
/// entities and selects without enum members.
fn enum_arms(res: &Resolver, target: &str) -> Vec<(String, String, String)> {
    let Some(members) = res.select_members(target) else {
        return Vec::new();
    };
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for m in members {
        if let Some(alias) = res.enum_alias(m, 0)
            && seen.insert(alias.clone())
        {
            // (variant ident, enum rust name, enum alias for want_enums)
            out.push((pascal(&alias), pascal(&alias), alias));
        }
    }
    out
}

/// All closure entities a ref-target ty can name. The target is either an
/// entity (-> itself + transitive subtypes) or an all-entity SELECT alias (->
/// each member's entities + their subtypes). Result is intersected with the
/// closure (so only generated entities become arms).
fn subtypes_in_closure(
    schema: &Schema,
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
pub fn build_children(schema: &Schema) -> BTreeMap<String, Vec<String>> {
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
        schema: &Schema,
        res: &Resolver,
        closure: &BTreeSet<String>,
        complex_parts: &BTreeSet<String>,
        derived: &BTreeMap<String, Vec<String>>,
        derivable: &BTreeMap<String, BTreeSet<String>>,
    ) -> ModelIr {
        let children = build_children(schema);
        let has_part_bag = !complex_parts.is_empty();

        // Simple entities = EVERY closure entity gets a simple arena (flattened
        // attrs). dual-appearance: a complex-part entity ALSO gets a SimpleEnt
        // here AND a PartEnt below — the two are no longer mutually exclusive.
        // always-complex entities (units, rational b_spline) get an unused empty
        // arena (harmless); they are read via the complex path.
        let mut simples: Vec<SimpleEnt> = Vec::new();
        for name in closure {
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
        // Expand: an aggregate-mixed SELECT needs its element E's ref-enum too
        // (the Agg arm holds Vec<{E}Ref>). Fixpoint in case E is itself such.
        let mut queue: Vec<String> = want_refs.iter().cloned().collect();
        while let Some(t) = queue.pop() {
            if let Some(members) = res.select_members(&t) {
                for m in members {
                    if let Some(e) = res.agg_element(m)
                        && res.ref_like(&e, 0)
                        && want_refs.insert(e.clone())
                    {
                        queue.push(e);
                    }
                }
            }
        }

        for target in &want_refs {
            let leaves = subtypes_in_closure(schema, res, closure, &children, target);
            let mut simple_arms: Vec<(String, String)> = Vec::new();
            let mut has_complex = false;
            for leaf in &leaves {
                // dual-appearance: a leaf can be BOTH a simple arm (standalone
                // `#1=FOO(..)`) and resolve to a complex (`#2=(.. FOO(..) ..)`),
                // so set both independently (not else-if).
                if schema.entity.contains_key(leaf) {
                    simple_arms.push((pascal(leaf), pascal(leaf)));
                }
                if complex_parts.contains(leaf) {
                    has_complex = true;
                }
            }
            simple_arms.sort();
            simple_arms.dedup();
            // enum arms (SELECT with ENUM members); register each enum for
            // generation via want_enums.
            let earms = enum_arms(res, target);
            for (_, _, alias) in &earms {
                want_enums.insert(alias.clone());
            }
            ref_enums.insert(
                target.clone(),
                RefEnum {
                    rust: format!("{}Ref", pascal(target)),
                    simple_arms,
                    aggregate: aggregate_arms(res, target),
                    enum_arms: earms
                        .into_iter()
                        .map(|(v, r, alias)| (v, r, alias.to_uppercase()))
                        .collect(),
                    scalar_arms: scalar_arms(res, target),
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
    schema: &Schema,
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
    schema: &Schema,
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
    schema: &Schema,
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
