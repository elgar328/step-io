//! Where the AP242 edition 2 output schema is narrower than the model.
//!
//! The model is typed by the union of every edition (so reading accepts what
//! any of them allows), and `Ap242Author` writes edition 2 only. Where an
//! edition 2 attribute — through its own declaration, a SELECT with fewer
//! members, or a redeclaration — admits fewer entities than the model field,
//! the constructor takes a narrower reference type and converts it into the
//! field; a complex part checks the same at run time. Edition 2's `WHERE`
//! rules that limit which types an attribute references (the profile's
//! `type_rules`, inherited by subtypes) narrow the same way; what they ask of
//! a complex value's parts together (none of a type, exactly one of several)
//! is checked at run time. Edition 2's aggregate bounds replace the model's
//! where they differ.

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
/// target names of `wide`) or a complex entity with a part in `fits` that
/// meets every one of `part_rules`.
#[derive(Debug, Clone)]
pub struct RuntimeNarrowing {
    pub wide: String,
    pub allowed: BTreeSet<String>,
    pub fits: BTreeSet<String>,
    /// What WHERE rules ask of a complex value's parts.
    pub part_rules: Vec<PartRule>,
}

/// What a WHERE rule asks of a complex value's parts (upper-case part
/// names). A complex value's `TYPEOF` is the union over its parts, and it
/// holds every supertype as a part, so it holds a type exactly when some
/// part is, or belongs to, that type.
#[derive(Debug, Clone)]
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

/// One edition 2 narrowing of a complex part's field: applies when the
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
    /// each stands for in edition 2 (``as `type` `` or ``for `entity.attr` ``).
    pub new_enums: BTreeMap<String, (String, RefEnum)>,
    /// (entity Rust name, field name) → the argument's reference enum and
    /// what WHERE rules ask of a complex value's parts, checked in the
    /// constructor.
    pub part_rules: BTreeMap<(String, String), (String, Vec<PartRule>)>,
    /// (narrow, wide) enum pairs a constructor converts between.
    pub conversions: BTreeSet<(String, String)>,
    /// (entity Rust name, field name) → edition 2's outer aggregate bounds,
    /// where they differ from the model's.
    pub bounds: BTreeMap<(String, String), (u32, Option<u32>)>,
    pub complex: Vec<ComplexAuthorNarrowing>,
}

/// Edition 2's view: entity hierarchy and SELECT table, unfiltered by the
/// coverage `narrow` settings (which name members of the unioned SELECTs and
/// would cut members edition 2 reaches through nested SELECTs).
struct E2<'a> {
    schema: &'a Schema,
    children: BTreeMap<&'a str, Vec<&'a str>>,
}

impl<'a> E2<'a> {
    fn new(schema: &'a Schema) -> Self {
        let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (n, e) in &schema.entity {
            for p in &e.parents {
                children.entry(p.as_str()).or_default().push(n);
            }
        }
        Self { schema, children }
    }

    /// The entities (lowercase) a type admits; empty for a non-reference.
    fn admits(&self, ty: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        self.admits_into(element(ty), &mut out, 0);
        out
    }

    fn admits_into(&self, name: &str, out: &mut BTreeSet<String>, depth: u32) {
        if depth > 32 {
            return;
        }
        if self.schema.entity.contains_key(name) {
            let mut stack = vec![name];
            while let Some(n) = stack.pop() {
                if out.insert(n.to_string())
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
            for m in members.split(',') {
                self.admits_into(element(m.trim()), out, depth + 1);
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
    pub fn build(ir: &ModelIr, e2_schema: &Schema, legal: &BTreeSet<String>) -> Self {
        let e2 = E2::new(e2_schema);
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
            let entities: BTreeSet<String> = names.flat_map(|n| e2.admits(n)).collect();
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
        // What edition 2 admits of a model reference enum, for `ty` and
        // within `rules` (the entities WHERE rules allow) if given, with a
        // complex value meeting `part_rules`.
        let narrowed = |re: &RefEnum,
                        ty: &str,
                        rules: Option<&BTreeSet<String>>,
                        part_rules: &[PartRule]|
         -> Option<RuntimeNarrowing> {
            let mut admits = e2.admits(ty);
            if admits.is_empty() {
                return None;
            }
            if let Some(rules) = rules {
                admits.retain(|a| rules.contains(a));
            }
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
            let part_rules = if re.has_complex {
                part_rules.to_vec()
            } else {
                Vec::new()
            };
            (allowed != m || fits != fits_m || !part_rules.is_empty()).then(|| RuntimeNarrowing {
                wide: re.rust.clone(),
                allowed,
                fits,
                part_rules,
            })
        };

        let mut out = AuthorNarrowing::default();
        let mut checks: BTreeMap<(String, String), RuntimeNarrowing> = BTreeMap::new();
        for se in &ir.simples {
            if !legal.contains(&se.name) {
                continue;
            }
            let attrs = e2_schema.flattened_attrs(&se.name.to_lowercase());
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
                let Some(target) = ref_target(&f.kind) else {
                    continue;
                };
                let re = &ir.ref_enums[target];
                let lower = se.name.to_lowercase();
                // The rules on this field holding for an instance of `ent`:
                // SELF's type is fixed, so its conditions are settled here.
                let rules_for = |ent: &str| -> Vec<(&str, &TypeRule)> {
                    e2_schema
                        .type_rules_for(ent)
                        .into_iter()
                        .filter(|(_, r)| r.name == f.name && r.holds_for(e2_schema, ent))
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
                let allowed_by_rules = rules.iter().map(|(_, r)| &r.admits).fold(
                    None::<BTreeSet<String>>,
                    |acc, admits| {
                        Some(match acc {
                            None => admits.clone(),
                            Some(acc) => acc.intersection(admits).cloned().collect(),
                        })
                    },
                );
                let part_rules = part_rules_of(&mut rules.iter().map(|(_, r)| *r));
                let Some(n) = narrowed(re, &a.ty, allowed_by_rules.as_ref(), &part_rules) else {
                    continue;
                };
                assert!(
                    re.aggregate.is_empty() && re.enum_arms.is_empty() && re.scalar_arms.is_empty(),
                    "{}.{}: edition 2 narrows a reference enum with non-entity arms ({})",
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
                    m.as_ref()
                        .is_some_and(|m| m.allowed == n.allowed && m.fits == n.fits)
                };
                // A rule names the type only where it holds the same for
                // its holder: not where this entity is exempt from another.
                let alone = rules
                    .iter()
                    .find(|(h, r)| same(&narrowed(re, &a.ty, Some(&r.admits), none)) && within(h));
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
                        let alike: Vec<&str> = e2_schema
                            .entity
                            .keys()
                            .map(String::as_str)
                            .filter(|e| {
                                e2_schema.is_a(&lower, e)
                                    && e2_schema
                                        .flattened_attrs(e)
                                        .iter()
                                        .any(|a| a.name == f.name)
                                    && same_rules(e)
                            })
                            .collect();
                        alike
                            .iter()
                            .find(|g| alike.iter().all(|e| e2_schema.is_a(e, g)))
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

        // A self-referencing constructor exists where edition 2 lets the
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

        // Complex parts: edition 2's redeclarations of another part's field,
        // and a part's own field typed narrower in edition 2.
        let part_rust: BTreeMap<String, &str> = ir
            .parts
            .iter()
            .map(|p| (p.name.to_lowercase(), p.rust.as_str()))
            .collect();
        for p in &ir.parts {
            let lower = p.name.to_lowercase();
            let Some(ent) = e2_schema.entity.get(&lower) else {
                continue;
            };
            let add = |owner: &str,
                       field: &str,
                       ty: &str,
                       rules: Option<&BTreeSet<String>>,
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
                let Some(target) = ref_target(&f.kind) else {
                    return;
                };
                if let Some(check) = narrowed(&ir.ref_enums[target], ty, rules, part_rules) {
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
                let Some(declared) = e2_schema
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
                    Some(&r.admits),
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
            true
        });
        out
    }

    /// The narrower enum `rust`: a model enum or one made for edition 2.
    fn narrow_enum<'s>(&'s self, ir: &'s ModelIr, rust: &str) -> &'s RefEnum {
        self.new_enums
            .get(rust)
            .map(|(_, re)| re)
            .or_else(|| ir.ref_enums.values().find(|re| re.rust == rust))
            .unwrap_or_else(|| panic!("narrower enum {rust} not found"))
    }

    /// The simple arms (Rust target names) of the narrower enum `rust`.
    fn narrow_arms(&self, ir: &ModelIr, rust: &str) -> BTreeSet<String> {
        let re = self.narrow_enum(ir, rust);
        re.simple_arms.iter().map(|(_, t)| t.clone()).collect()
    }

    /// Whether the narrower enum `rust` has a complex arm.
    fn has_complex(&self, ir: &ModelIr, rust: &str) -> bool {
        self.narrow_enum(ir, rust).has_complex
    }

    /// The enum a constructor takes for narrowing `n`: a model enum with
    /// exactly those AP242-legal arms and complex parts, or a new `Ap242…Ref`
    /// named after edition 2's type.
    fn enum_for(
        &mut self,
        ir: &ModelIr,
        n: &RuntimeNarrowing,
        e2_type: &str,
        what: &str,
        legal: &BTreeSet<String>,
        legal_arms: &impl Fn(&RefEnum) -> BTreeSet<String>,
    ) -> String {
        let pure = |re: &RefEnum| {
            re.aggregate.is_empty() && re.enum_arms.is_empty() && re.scalar_arms.is_empty()
        };
        let same = |re: &RefEnum| {
            pure(re)
                && legal_arms(re) == n.allowed
                && re
                    .complex_parts
                    .iter()
                    .filter(|p| legal.contains(p.as_str()))
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    == n.fits
        };
        if let Some(re) = ir.ref_enums.get(e2_type).filter(|re| same(re)) {
            return re.rust.clone();
        }
        if let Some(re) = ir.ref_enums.values().find(|re| same(re)) {
            return re.rust.clone();
        }
        let rust = format!("Ap242{}Ref", pascal(e2_type));
        let re = RefEnum {
            rust: rust.clone(),
            simple_arms: n.allowed.iter().map(|t| (t.clone(), t.clone())).collect(),
            aggregate: Vec::new(),
            enum_arms: Vec::new(),
            scalar_arms: Vec::new(),
            has_complex: !n.fits.is_empty(),
            complex_parts: n.fits.iter().cloned().collect(),
        };
        if let Some((_, prev)) = self.new_enums.get(&rust) {
            assert!(
                prev.simple_arms == re.simple_arms && prev.complex_parts == re.complex_parts,
                "{rust}: two narrowings of edition 2's {e2_type} differ"
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
