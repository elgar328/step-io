//! Where the output schema (the AP242 edition `Ap242Author` writes) is
//! narrower than the model.
//!
//! The model is typed by the union of every edition (so reading accepts what
//! any of them allows), and `Ap242Author` writes the output edition only.
//! Where an output attribute — through its own declaration, a SELECT with
//! fewer members, or a redeclaration — admits fewer entities, or fewer
//! aggregate SELECT members or elements of them, than the model field, the
//! constructor takes a narrower reference type and converts it into the
//! field; a complex part checks the same at run time. The output edition's
//! `WHERE` rules that limit which types an attribute references (the
//! profile's `type_rules`, inherited by subtypes) narrow the same way; what
//! they ask of a complex value's parts together (none of a type, exactly one
//! of several) is checked at run time. The output edition's aggregate bounds
//! replace the model's where they differ.

use std::collections::{BTreeMap, BTreeSet};

use crate::classify::{Kind, pascal};
use crate::model_ir::{ModelIr, RefEnum};
use crate::schema::{RuleCount, Schema, TypeRule, strip_optional};

/// A reference type a constructor takes in place of the model field's.
#[derive(Debug, Clone)]
pub struct NarrowParam {
    /// Rust name of the narrower reference enum.
    pub narrow: String,
    /// Rust name of the model field's reference enum.
    pub wide: String,
}

/// A run-time narrowing check: the value of `field` (a model-typed
/// reference, or a list of them) must be one of `allowed` simple arms (Rust
/// target names of `wide`), a complex entity with a part in `fits` that
/// meets every one of `part_rules`, or an aggregate arm in `aggregates`
/// whose every element meets that arm's narrowing.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeNarrowing {
    pub wide: String,
    pub allowed: BTreeSet<String>,
    pub fits: BTreeSet<String>,
    /// What WHERE rules ask of a complex value's parts.
    pub part_rules: Vec<PartRule>,
    /// Aggregate arm (variant of `wide`) → what each element may be; an
    /// aggregate arm not listed is not admitted.
    pub aggregates: BTreeMap<String, RuntimeNarrowing>,
}

/// What a WHERE rule asks of a complex value's parts (upper-case part
/// names). A complex value's `TYPEOF` is the union over its parts, and it
/// holds every supertype as a part, so it holds a type exactly when some
/// part is, or belongs to, that type.
#[derive(Debug, Clone, PartialEq)]
pub enum PartRule {
    /// No part is, or belongs to, any of these types.
    NoneOf(BTreeSet<String>),
    /// Of these groups (one per type the rule names: the parts that are or
    /// belong to it), exactly one holds a part.
    ExactlyOneOf(Vec<BTreeSet<String>>),
}

/// Add `from` to `into`: the excluded parts of all rules make one set (a
/// part excluded by any rule is excluded); each count is its own rule.
fn merge_part_rules(into: &mut Vec<PartRule>, from: impl IntoIterator<Item = PartRule>) {
    for r in from {
        match r {
            PartRule::NoneOf(s) => {
                if let Some(PartRule::NoneOf(acc)) =
                    into.iter_mut().find(|r| matches!(r, PartRule::NoneOf(_)))
                {
                    acc.extend(s);
                } else {
                    into.insert(0, PartRule::NoneOf(s));
                }
            }
            r @ PartRule::ExactlyOneOf(_) => into.push(r),
        }
    }
}

/// One output-edition narrowing of a complex part's field: applies when the
/// instance holds part `by` (Rust variant name), a part of each `when` group
/// and no part in `unless` (upper-case part names: a WHERE rule's condition
/// on SELF's type).
#[derive(Debug, Clone)]
pub struct ComplexAuthorNarrowing {
    /// Rust variant name of the part declaring the field.
    pub owner: String,
    /// The field's position among the owner's own fields.
    pub idx: usize,
    pub by: String,
    pub when: Vec<BTreeSet<String>>,
    pub unless: BTreeSet<String>,
    pub check: RuntimeNarrowing,
}

#[derive(Default)]
pub struct AuthorNarrowing {
    /// (entity Rust name, field name) → the constructor's reference type.
    pub params: BTreeMap<(String, String), NarrowParam>,
    /// (entity Rust name, field name) → the run-time check a self-referencing
    /// constructor (`add_*_cyclic`) runs, its argument being the model
    /// struct; the plain constructor takes the narrower type in `params`.
    pub cyclic: BTreeMap<(String, String), RuntimeNarrowing>,
    /// Entities (Rust name) that get a self-referencing constructor: some
    /// field's constructor type admits the entity itself.
    pub self_referencing: BTreeSet<String>,
    /// Narrower enums no model enum matches, keyed by Rust name, with what
    /// each stands for in the output edition (``as `type` `` or
    /// ``for `entity.attr` ``).
    pub new_enums: BTreeMap<String, (String, RefEnum)>,
    /// (entity Rust name, field name) → the argument's reference enum and
    /// what WHERE rules ask of a complex value's parts, checked in the
    /// constructor.
    pub part_rules: BTreeMap<(String, String), (String, Vec<PartRule>)>,
    /// (narrow, wide) enum pairs a constructor converts between.
    pub conversions: BTreeSet<(String, String)>,
    /// (entity Rust name, field name) → the output edition's outer aggregate
    /// bounds, where they differ from the model's.
    pub bounds: BTreeMap<(String, String), (u32, Option<u32>)>,
    /// Named LIST/SET/BAG type (lowercase) → its bounds in the output
    /// edition, for each type with a bound to check; read by the aggregate
    /// SELECT member that holds the type.
    pub aggregate_bounds: BTreeMap<String, (u32, Option<u32>)>,
    pub complex: Vec<ComplexAuthorNarrowing>,
}

/// What a type admits: the entities (lowercase) a reference may be, and its
/// aggregate SELECT members (alias → element type), which hold elements
/// rather than being one.
#[derive(Default)]
struct Admits {
    entities: BTreeSet<String>,
    aggregates: BTreeMap<String, String>,
}

/// What WHERE rules allow of an attribute: the entities, and per aggregate
/// SELECT member the entities its elements may be (a member not listed is not
/// allowed).
#[derive(Clone)]
struct Allowed {
    entities: BTreeSet<String>,
    aggregates: BTreeMap<String, BTreeSet<String>>,
}

impl Allowed {
    fn of(r: &TypeRule) -> Self {
        Self {
            entities: r.admits.clone(),
            aggregates: r
                .aggregates
                .iter()
                .map(|a| (a.type_name.clone(), a.admits.clone()))
                .collect(),
        }
    }

    /// What `self` and `other` both allow.
    fn and(mut self, other: &Self) -> Self {
        self.entities.retain(|e| other.entities.contains(e));
        self.aggregates.retain(|t, elements| {
            other.aggregates.get(t).is_some_and(|o| {
                elements.retain(|e| o.contains(e));
                true
            })
        });
        self
    }
}

/// The output edition's view: entity hierarchy and SELECT table, unfiltered
/// by the coverage `narrow` settings (which name members of the unioned
/// SELECTs and would cut members the output edition reaches through nested
/// SELECTs).
struct Target<'a> {
    schema: &'a Schema,
    children: BTreeMap<&'a str, Vec<&'a str>>,
}

impl<'a> Target<'a> {
    fn new(schema: &'a Schema) -> Self {
        let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (n, e) in &schema.entity {
            for p in &e.parents {
                children.entry(p.as_str()).or_default().push(n);
            }
        }
        Self { schema, children }
    }

    /// What a type admits (of an aggregate attribute, each element); empty
    /// for a non-reference.
    fn admits(&self, ty: &str) -> Admits {
        let mut out = Admits::default();
        self.admits_into(element(ty), &mut out, 0);
        out
    }

    /// The entities (lowercase) a type admits.
    fn entities(&self, ty: &str) -> BTreeSet<String> {
        self.admits(ty).entities
    }

    fn admits_into(&self, name: &str, out: &mut Admits, depth: u32) {
        if depth > 32 {
            return;
        }
        if self.schema.entity.contains_key(name) {
            let mut stack = vec![name];
            while let Some(n) = stack.pop() {
                if out.entities.insert(n.to_string())
                    && let Some(kids) = self.children.get(n)
                {
                    stack.extend(kids.iter().copied());
                }
            }
            return;
        }
        let Some(td) = self.schema.types.get(name) else {
            return;
        };
        if let Some(members) = td
            .aliased
            .strip_prefix("SELECT(")
            .and_then(|m| m.strip_suffix(')'))
        {
            for m in members.split(',').map(str::trim) {
                // An aggregate member holds elements; it is not one of them.
                match self.schema.types.get(m) {
                    Some(t) if outer_bounds(&t.aliased).is_some() => {
                        out.aggregates
                            .insert(m.to_string(), element(&t.aliased).to_string());
                    }
                    _ => self.admits_into(m, out, depth + 1),
                }
            }
        } else {
            self.admits_into(element(&td.aliased), out, depth + 1);
        }
    }
}

/// The innermost element of an attribute type: past `OPTIONAL` and any
/// `LIST/SET/BAG/ARRAY [..] OF`.
fn element(ty: &str) -> &str {
    let (_, mut t) = strip_optional(ty.trim());
    loop {
        let Some(rest) = ["LIST", "SET", "BAG", "ARRAY"]
            .iter()
            .find_map(|k| t.strip_prefix(k))
        else {
            return t.trim();
        };
        let rest = rest.trim_start();
        let rest = match rest.strip_prefix('[') {
            Some(r) => r.split_once(']').map_or(r, |(_, after)| after),
            None => rest,
        };
        t = rest
            .trim_start()
            .strip_prefix("OF")
            .unwrap_or(rest)
            .trim_start();
        let (_, inner) = strip_optional(t);
        t = inner;
    }
}

/// The outer aggregate bounds of an attribute type, if it is one.
fn outer_bounds(ty: &str) -> Option<(u32, Option<u32>)> {
    let (_, t) = strip_optional(ty.trim());
    let rest = ["LIST", "SET", "BAG", "ARRAY"]
        .iter()
        .find_map(|k| t.strip_prefix(k))?
        .trim_start();
    let Some(b) = rest.strip_prefix('[') else {
        return Some((0, None));
    };
    let (b, _) = b.split_once(']')?;
    let (lo, hi) = b.split_once(':')?;
    let lo = lo.trim().parse().ok()?;
    let hi = hi.trim();
    Some((lo, if hi == "?" { None } else { hi.parse().ok() }))
}

/// The reference enum a field's kind names, for a `Ref` or a list of them.
fn ref_target(k: &Kind) -> Option<&str> {
    match k {
        Kind::Ref(t) => Some(t),
        Kind::Vec(inner) => match inner.as_ref() {
            Kind::Ref(t) => Some(t),
            _ => None,
        },
        _ => None,
    }
}

impl AuthorNarrowing {
    pub fn build(ir: &ModelIr, target_schema: &Schema, legal: &BTreeSet<String>) -> Self {
        let target = Target::new(target_schema);
        let upper: BTreeMap<&str, &str> = ir
            .simples
            .iter()
            .map(|s| (s.rust.as_str(), s.name.as_str()))
            .collect();
        let legal_arms = |re: &RefEnum| -> BTreeSet<String> {
            re.simple_arms
                .iter()
                .filter(|(_, t)| upper.get(t.as_str()).is_some_and(|u| legal.contains(*u)))
                .map(|(_, t)| t.clone())
                .collect()
        };
        // The complex parts that are, or belong to, any of `names`.
        let parts_of = |names: &mut dyn Iterator<Item = &String>| -> BTreeSet<String> {
            let entities: BTreeSet<String> = names.flat_map(|n| target.entities(n)).collect();
            ir.parts
                .iter()
                .filter(|p| entities.contains(&p.name.to_lowercase()))
                .map(|p| p.name.clone())
                .collect()
        };
        // What WHERE rules ask of a complex value's parts. A type no part is
        // or belongs to is never held; a count over one type is the `fits`
        // check already (a fitting part holds that type), and so is "at
        // least one".
        let part_rules_of = |rules: &mut dyn Iterator<Item = &TypeRule>| -> Vec<PartRule> {
            let mut out = Vec::new();
            for r in rules {
                match r.count {
                    RuleCount::Zero => {
                        let parts = parts_of(&mut r.names.iter());
                        if !parts.is_empty() {
                            merge_part_rules(&mut out, [PartRule::NoneOf(parts)]);
                        }
                    }
                    RuleCount::ExactlyOne => {
                        let groups: Vec<BTreeSet<String>> = r
                            .names
                            .iter()
                            .map(|n| parts_of(&mut std::iter::once(n)))
                            .filter(|g| !g.is_empty())
                            .collect();
                        if groups.len() > 1 {
                            merge_part_rules(&mut out, [PartRule::ExactlyOneOf(groups)]);
                        }
                    }
                    RuleCount::AtLeastOne => {}
                }
            }
            out
        };
        // Of a model reference enum, the AP242-legal simple arms and complex
        // parts that are among `admits`, and all of them.
        let arms_within = |re: &RefEnum, admits: &BTreeSet<String>| {
            let m = legal_arms(re);
            let allowed: BTreeSet<String> = m
                .iter()
                .filter(|t| {
                    upper
                        .get(t.as_str())
                        .is_some_and(|u| admits.contains(&u.to_lowercase()))
                })
                .cloned()
                .collect();
            let fits_m: BTreeSet<String> = re
                .complex_parts
                .iter()
                .filter(|p| legal.contains(p.as_str()))
                .cloned()
                .collect();
            let fits: BTreeSet<String> = fits_m
                .iter()
                .filter(|p| admits.contains(&p.to_lowercase()))
                .cloned()
                .collect();
            (allowed, fits, m, fits_m)
        };
        // What the output edition admits of a model reference enum, for `ty`
        // and within `rules` (what WHERE rules allow) if given, with a
        // complex value meeting `part_rules`.
        let narrowed = |re: &RefEnum,
                        ty: &str,
                        rules: Option<&Allowed>,
                        part_rules: &[PartRule]|
         -> Option<RuntimeNarrowing> {
            let mut admits = target.admits(ty);
            if admits.entities.is_empty() && admits.aggregates.is_empty() {
                return None;
            }
            if let Some(rules) = rules {
                admits.entities.retain(|a| rules.entities.contains(a));
            }
            let (allowed, fits, m, fits_m) = arms_within(re, &admits.entities);
            // Each aggregate arm: admitted where the type has the member and
            // every rule lists it, its elements within what they all allow.
            let mut aggregates = BTreeMap::new();
            let mut aggregates_narrower = false;
            for (variant, elem_rust, wire) in &re.aggregate {
                let alias = wire.to_lowercase();
                let elem = ir
                    .ref_enums
                    .values()
                    .find(|e| e.rust == *elem_rust)
                    .expect("aggregate element enum");
                let elements = admits.aggregates.get(&alias).map(|elem_ty| {
                    let mut elements = target.entities(elem_ty);
                    if let Some(rules) = rules {
                        match rules.aggregates.get(&alias) {
                            Some(r) => elements.retain(|e| r.contains(e)),
                            None => elements.clear(),
                        }
                    }
                    elements
                });
                let (e_allowed, e_fits, e_m, e_fits_m) =
                    arms_within(elem, &elements.unwrap_or_default());
                if e_allowed.is_empty() && e_fits.is_empty() {
                    aggregates_narrower = true;
                    continue;
                }
                aggregates_narrower |= e_allowed != e_m || e_fits != e_fits_m;
                aggregates.insert(
                    variant.clone(),
                    RuntimeNarrowing {
                        wide: elem.rust.clone(),
                        allowed: e_allowed,
                        fits: e_fits,
                        part_rules: Vec::new(),
                        aggregates: BTreeMap::new(),
                    },
                );
            }
            let part_rules = if re.has_complex {
                part_rules.to_vec()
            } else {
                Vec::new()
            };
            (allowed != m || fits != fits_m || !part_rules.is_empty() || aggregates_narrower).then(
                || RuntimeNarrowing {
                    wide: re.rust.clone(),
                    allowed,
                    fits,
                    part_rules,
                    aggregates,
                },
            )
        };

        let mut out = AuthorNarrowing::default();
        for (name, t) in &target_schema.types {
            let aliased = t.aliased.trim_start();
            if ["LIST", "SET", "BAG"]
                .iter()
                .any(|k| aliased.starts_with(k))
                && let Some(b) = outer_bounds(aliased)
                && b != (0, None)
            {
                out.aggregate_bounds.insert(name.clone(), b);
            }
        }
        let mut checks: BTreeMap<(String, String), RuntimeNarrowing> = BTreeMap::new();
        for se in &ir.simples {
            if !legal.contains(&se.name) {
                continue;
            }
            let attrs = target_schema.flattened_attrs(&se.name.to_lowercase());
            for f in &se.fields {
                if f.derived {
                    continue;
                }
                let Some(a) = attrs.iter().find(|a| a.name == f.name) else {
                    continue;
                };
                let key = (se.rust.clone(), f.name.clone());
                if matches!(f.kind, Kind::Vec(_))
                    && let Some(b) = outer_bounds(&a.ty)
                    && b != (f.min, f.max)
                {
                    out.bounds.insert(key.clone(), b);
                }
                let Some(ref_ty) = ref_target(&f.kind) else {
                    continue;
                };
                let re = &ir.ref_enums[ref_ty];
                let lower = se.name.to_lowercase();
                // The rules on this field holding for an instance of `ent`:
                // SELF's type is fixed, so its conditions are settled here.
                let rules_for = |ent: &str| -> Vec<(&str, &TypeRule)> {
                    target_schema
                        .type_rules_for(ent)
                        .into_iter()
                        .filter(|(_, r)| r.name == f.name && r.holds_for(target_schema, ent))
                        .collect()
                };
                let rules = rules_for(&lower);
                // Whether every rule holding for `ent` holds for this entity.
                let within = |ent: &str| {
                    rules_for(ent)
                        .iter()
                        .all(|(_, t)| rules.iter().any(|(_, r)| std::ptr::eq(*t, *r)))
                };
                let same_rules = |ent: &str| rules_for(ent).len() == rules.len() && within(ent);
                let allowed_by_rules = rules
                    .iter()
                    .map(|(_, r)| Allowed::of(r))
                    .reduce(|acc, r| acc.and(&r));
                let part_rules = part_rules_of(&mut rules.iter().map(|(_, r)| *r));
                let Some(n) = narrowed(re, &a.ty, allowed_by_rules.as_ref(), &part_rules) else {
                    continue;
                };
                assert!(
                    re.enum_arms.is_empty() && re.scalar_arms.is_empty(),
                    "{}.{}: the output edition narrows a reference enum with enum or scalar arms ({})",
                    se.name,
                    f.name,
                    re.rust
                );
                checks.insert(key.clone(), n.clone());
                // What WHERE rules ask of a complex value's parts is checked
                // in the constructor, not by the argument's type.
                let part_rules = n.part_rules;
                let none: &[PartRule] = &[];
                let Some(n) = narrowed(re, &a.ty, allowed_by_rules.as_ref(), none) else {
                    out.part_rules.insert(key, (re.rust.clone(), part_rules));
                    continue;
                };
                // Named after the declared type, unless WHERE rules narrow
                // it further: then after the type the one rule that alone
                // does so names, if the narrowing is all that type allows;
                // else after that rule's attribute, or this attribute.
                let same = |m: &Option<RuntimeNarrowing>| {
                    m.as_ref().is_some_and(|m| {
                        m.allowed == n.allowed && m.fits == n.fits && m.aggregates == n.aggregates
                    })
                };
                // A rule names the type only where it holds the same for
                // its holder: not where this entity is exempt from another.
                let alone = rules.iter().find(|(h, r)| {
                    same(&narrowed(re, &a.ty, Some(&Allowed::of(r)), none)) && within(h)
                });
                let (name, what) = if same(&narrowed(re, &a.ty, None, none)) {
                    let ty = element(&a.ty);
                    (ty.to_string(), format!("as `{ty}`"))
                } else if let Some(ty) = alone
                    .map(|(_, r)| r.type_name.as_str())
                    .filter(|ty| same(&narrowed(re, ty, None, none)))
                {
                    (ty.to_string(), format!("as `{ty}`"))
                } else {
                    // Else after the most general entity with this field
                    // that the same rules hold for.
                    let general = || {
                        let alike: Vec<&str> = target_schema
                            .entity
                            .keys()
                            .map(String::as_str)
                            .filter(|e| {
                                target_schema.is_a(&lower, e)
                                    && target_schema
                                        .flattened_attrs(e)
                                        .iter()
                                        .any(|a| a.name == f.name)
                                    && same_rules(e)
                            })
                            .collect();
                        alike
                            .iter()
                            .find(|g| alike.iter().all(|e| target_schema.is_a(e, g)))
                            .map_or(lower.clone(), |g| g.to_string())
                    };
                    let (holder, attr) = alone.map_or_else(
                        || (general(), f.name.as_str()),
                        |(h, r)| (h.to_string(), r.name.as_str()),
                    );
                    (format!("{holder}_{attr}"), format!("for `{holder}.{attr}`"))
                };
                let narrow = out.enum_for(ir, &n, &name, &what, legal, &legal_arms);
                out.conversions.insert((narrow.clone(), re.rust.clone()));
                out.params.insert(
                    key.clone(),
                    NarrowParam {
                        narrow,
                        wide: re.rust.clone(),
                    },
                );
                // An argument type with no complex arm takes no complex value.
                let narrow = out.params[&key].narrow.clone();
                if !part_rules.is_empty() && out.has_complex(ir, &narrow) {
                    out.part_rules.insert(key, (narrow, part_rules));
                }
            }
        }

        // A self-referencing constructor exists where the output edition lets the
        // entity reference itself: through a field's narrower type where it
        // has one. It takes the model struct, so it checks the narrowings at
        // run time; the plain constructor still takes the narrower types.
        for se in &ir.simples {
            if !legal.contains(&se.name) {
                continue;
            }
            let hits = se.fields.iter().any(|f| {
                if f.derived {
                    return false;
                }
                match out.params.get(&(se.rust.clone(), f.name.clone())) {
                    Some(p) => out.narrow_arms(ir, &p.narrow).contains(&se.rust),
                    None => crate::emit::kind_hits(ir, &f.kind, &se.rust),
                }
            });
            if hits {
                out.self_referencing.insert(se.rust.clone());
                for ((e, f), n) in &checks {
                    if *e == se.rust {
                        out.cyclic.insert((e.clone(), f.clone()), n.clone());
                    }
                }
            }
        }

        // Complex parts: the output edition's redeclarations of another
        // part's field, and a part's own field typed narrower in it.
        let part_rust: BTreeMap<String, &str> = ir
            .parts
            .iter()
            .map(|p| (p.name.to_lowercase(), p.rust.as_str()))
            .collect();
        for p in &ir.parts {
            let lower = p.name.to_lowercase();
            let Some(ent) = target_schema.entity.get(&lower) else {
                continue;
            };
            let add = |owner: &str,
                       field: &str,
                       ty: &str,
                       rules: Option<&Allowed>,
                       part_rules: &[PartRule],
                       (when, unless): (Vec<BTreeSet<String>>, BTreeSet<String>),
                       out: &mut AuthorNarrowing| {
                let Some(op) = ir.parts.iter().find(|o| o.rust == owner) else {
                    return;
                };
                let Some((idx, f)) = op.fields.iter().enumerate().find(|(_, f)| f.name == field)
                else {
                    return;
                };
                if f.derived {
                    return;
                }
                let Some(ref_ty) = ref_target(&f.kind) else {
                    return;
                };
                if let Some(check) = narrowed(&ir.ref_enums[ref_ty], ty, rules, part_rules) {
                    out.complex.push(ComplexAuthorNarrowing {
                        owner: owner.to_string(),
                        idx,
                        by: p.rust.clone(),
                        when,
                        unless,
                        check,
                    });
                }
            };
            for r in &ent.redeclared_attrs {
                if let Some(owner) = r.owner.as_deref().and_then(|o| part_rust.get(o)) {
                    add(
                        owner,
                        &r.name,
                        &r.ty,
                        None,
                        &[],
                        Default::default(),
                        &mut out,
                    );
                }
            }
            for a in &ent.own_attrs {
                add(
                    &p.rust,
                    &a.name,
                    &a.ty,
                    None,
                    &[],
                    Default::default(),
                    &mut out,
                );
            }
            for r in &ent.type_rules {
                let Some(owner) = part_rust.get(&r.owner) else {
                    continue;
                };
                let Some(declared) = target_schema
                    .entity
                    .get(&r.owner)
                    .and_then(|o| o.own_attrs.iter().find(|a| a.name == r.name))
                else {
                    continue;
                };
                // SELF's type is a complex value's parts: it is an X where a
                // part is, or belongs to, X. A rule only for a type no part
                // is never applies.
                let when: Vec<BTreeSet<String>> = r
                    .when_self
                    .iter()
                    .map(|t| parts_of(&mut std::iter::once(t)))
                    .collect();
                if when.iter().any(BTreeSet::is_empty) {
                    continue;
                }
                let unless = parts_of(&mut r.unless_self.iter());
                let part_rules = part_rules_of(&mut std::iter::once(r));
                add(
                    owner,
                    &r.name,
                    &declared.ty,
                    Some(&Allowed::of(r)),
                    &part_rules,
                    (when, unless),
                    &mut out,
                );
            }
        }
        let key = |c: &ComplexAuthorNarrowing| {
            (
                c.owner.clone(),
                c.idx,
                c.by.clone(),
                c.when.clone(),
                c.unless.clone(),
            )
        };
        out.complex.sort_by_key(key);
        // One check per field, part and condition: a declaration and a WHERE
        // rule narrowing the same field hold together.
        out.complex.dedup_by(|next, kept| {
            if key(next) != key(kept) {
                return false;
            }
            let k = &mut kept.check;
            k.allowed.retain(|a| next.check.allowed.contains(a));
            k.fits.retain(|p| next.check.fits.contains(p));
            merge_part_rules(&mut k.part_rules, next.check.part_rules.iter().cloned());
            k.aggregates.retain(|v, e| {
                next.check.aggregates.get(v).is_some_and(|n| {
                    e.allowed.retain(|a| n.allowed.contains(a));
                    e.fits.retain(|p| n.fits.contains(p));
                    true
                })
            });
            true
        });
        out
    }

    /// The narrower enum `rust`: a model enum or one made for the output
    /// edition.
    fn narrow_enum<'s>(&'s self, ir: &'s ModelIr, rust: &str) -> &'s RefEnum {
        self.new_enums
            .get(rust)
            .map(|(_, re)| re)
            .or_else(|| ir.ref_enums.values().find(|re| re.rust == rust))
            .unwrap_or_else(|| panic!("narrower enum {rust} not found"))
    }

    /// The simple arms (Rust target names) of the narrower enum `rust`, and
    /// of its aggregate arms' element enums.
    fn narrow_arms(&self, ir: &ModelIr, rust: &str) -> BTreeSet<String> {
        let re = self.narrow_enum(ir, rust);
        let mut arms: BTreeSet<String> = re.simple_arms.iter().map(|(_, t)| t.clone()).collect();
        for (_, elem, _) in &re.aggregate {
            arms.extend(self.narrow_arms(ir, elem));
        }
        arms
    }

    /// Whether the narrower enum `rust` has a complex arm.
    fn has_complex(&self, ir: &ModelIr, rust: &str) -> bool {
        self.narrow_enum(ir, rust).has_complex
    }

    /// The enum a constructor takes for narrowing `n`: a model enum with
    /// exactly those AP242-legal arms, complex parts and aggregate arms (each
    /// element enum holding exactly what the narrowing allows of an
    /// element), or a new `Ap242…Ref` named after the output edition's type.
    fn enum_for(
        &mut self,
        ir: &ModelIr,
        n: &RuntimeNarrowing,
        target_type: &str,
        what: &str,
        legal: &BTreeSet<String>,
        legal_arms: &impl Fn(&RefEnum) -> BTreeSet<String>,
    ) -> String {
        let legal_fits = |re: &RefEnum| -> BTreeSet<String> {
            re.complex_parts
                .iter()
                .filter(|p| legal.contains(p.as_str()))
                .cloned()
                .collect()
        };
        let model_enum = |rust: &str| {
            ir.ref_enums
                .values()
                .find(|e| e.rust == rust)
                .expect("model enum")
        };
        let holds = |re: &RefEnum, n: &RuntimeNarrowing| {
            legal_arms(re) == n.allowed && legal_fits(re) == n.fits
        };
        let same = |re: &RefEnum| {
            re.enum_arms.is_empty()
                && re.scalar_arms.is_empty()
                && holds(re, n)
                && re.aggregate.len() == n.aggregates.len()
                && re.aggregate.iter().all(|(v, elem, _)| {
                    n.aggregates
                        .get(v)
                        .is_some_and(|en| holds(model_enum(elem), en))
                })
        };
        if let Some(re) = ir.ref_enums.get(target_type).filter(|re| same(re)) {
            return re.rust.clone();
        }
        if let Some(re) = ir.ref_enums.values().find(|re| same(re)) {
            return re.rust.clone();
        }
        // Each admitted aggregate arm, with an element enum of its own where
        // the elements are narrowed.
        let wide = model_enum(&n.wide);
        let mut aggregate = Vec::new();
        for (variant, elem_wide, wire) in &wide.aggregate {
            let Some(en) = n.aggregates.get(variant) else {
                continue;
            };
            let alias = wire.to_lowercase();
            let elem = self.enum_for(
                ir,
                en,
                &format!("{target_type}_{alias}"),
                &format!("in a `{alias}` {what}"),
                legal,
                legal_arms,
            );
            if elem != *elem_wide {
                self.conversions.insert((elem.clone(), elem_wide.clone()));
            }
            aggregate.push((variant.clone(), elem, wire.clone()));
        }
        let rust = format!("Ap242{}Ref", pascal(target_type));
        let re = RefEnum {
            rust: rust.clone(),
            simple_arms: n.allowed.iter().map(|t| (t.clone(), t.clone())).collect(),
            aggregate,
            enum_arms: Vec::new(),
            scalar_arms: Vec::new(),
            has_complex: !n.fits.is_empty(),
            complex_parts: n.fits.iter().cloned().collect(),
        };
        if let Some((_, prev)) = self.new_enums.get(&rust) {
            assert!(
                prev.simple_arms == re.simple_arms
                    && prev.complex_parts == re.complex_parts
                    && prev.aggregate == re.aggregate,
                "{rust}: two narrowings of the output edition's {target_type} differ"
            );
        }
        assert!(
            !ir.ref_enums.values().any(|e| e.rust == rust),
            "{rust} would clash with a model enum"
        );
        self.new_enums.insert(rust.clone(), (what.to_string(), re));
        rust
    }

    /// One line per narrowing, for review.
    pub fn report(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for ((e, f), p) in &self.params {
            lines.push(format!("param   {e}.{f}: {} -> {}", p.wide, p.narrow));
        }
        for ((e, f), n) in &self.cyclic {
            lines.push(format!(
                "cyclic  {e}.{f}: {} to {} arms",
                n.wide,
                n.allowed.len()
            ));
        }
        for ((e, f), (lo, hi)) in &self.bounds {
            lines.push(format!(
                "bounds  {e}.{f}: [{lo}:{}]",
                hi.map_or("?".into(), |h| h.to_string())
            ));
        }
        for c in &self.complex {
            let mut on = String::new();
            for w in &c.when {
                on += &format!(" with one of {w:?}");
            }
            if !c.unless.is_empty() {
                on += &format!(" without {:?}", c.unless);
            }
            lines.push(format!(
                "complex {}#{} by {}{on}: {} to {} arms",
                c.owner,
                c.idx,
                c.by,
                c.check.wide,
                c.check.allowed.len()
            ));
        }
        lines
    }
}
