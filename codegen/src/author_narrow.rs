//! Where the AP242 edition 2 output schema is narrower than the model.
//!
//! The model is typed by the union of every edition (so reading accepts what
//! any of them allows), and `Ap242Author` writes edition 2 only. Where an
//! edition 2 attribute — through its own declaration, a SELECT with fewer
//! members, or a redeclaration — admits fewer entities than the model field,
//! the constructor takes a narrower reference type and converts it into the
//! field; a complex part checks the same at run time. Edition 2's aggregate
//! bounds replace the model's where they differ.

use std::collections::{BTreeMap, BTreeSet};

use crate::classify::{Kind, pascal};
use crate::model_ir::{ModelIr, RefEnum};
use crate::schema::{Schema, strip_optional};

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
/// target names of `wide`) or a complex entity with a part in `fits`.
#[derive(Debug, Clone)]
pub struct RuntimeNarrowing {
    pub wide: String,
    pub allowed: BTreeSet<String>,
    pub fits: BTreeSet<String>,
}

/// One edition 2 narrowing of a complex part's field: applies when the
/// instance holds part `by` (Rust variant name).
#[derive(Debug, Clone)]
pub struct ComplexAuthorNarrowing {
    /// Rust variant name of the part declaring the field.
    pub owner: String,
    /// The field's position among the owner's own fields.
    pub idx: usize,
    pub by: String,
    pub check: RuntimeNarrowing,
}

#[derive(Default)]
pub struct AuthorNarrowing {
    /// (entity Rust name, field name) → the constructor's reference type.
    pub params: BTreeMap<(String, String), NarrowParam>,
    /// (entity Rust name, field name) → the run-time check a self-referencing
    /// constructor (`add_*_cyclic`) runs instead, its argument being the
    /// model struct.
    pub cyclic: BTreeMap<(String, String), RuntimeNarrowing>,
    /// Narrower enums no model enum matches, keyed by Rust name, with the
    /// edition 2 type each stands for.
    pub new_enums: BTreeMap<String, (String, RefEnum)>,
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
        // What edition 2 admits of a model reference enum, for `ty`.
        let narrowed = |re: &RefEnum, ty: &str| -> Option<RuntimeNarrowing> {
            let admits = e2.admits(ty);
            if admits.is_empty() {
                return None;
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
            (allowed != m || fits != fits_m).then(|| RuntimeNarrowing {
                wide: re.rust.clone(),
                allowed,
                fits,
            })
        };

        let mut out = AuthorNarrowing::default();
        let cyclic: BTreeSet<&str> = ir
            .simples
            .iter()
            .filter(|s| crate::emit::self_ref_capable(ir, s))
            .map(|s| s.rust.as_str())
            .collect();
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
                let Some(n) = narrowed(re, &a.ty) else {
                    continue;
                };
                assert!(
                    re.aggregate.is_empty() && re.enum_arms.is_empty() && re.scalar_arms.is_empty(),
                    "{}.{}: edition 2 narrows a reference enum with non-entity arms ({})",
                    se.name,
                    f.name,
                    re.rust
                );
                if cyclic.contains(se.rust.as_str()) {
                    out.cyclic.insert(key, n);
                    continue;
                }
                let narrow = out.enum_for(ir, &n, element(&a.ty), legal, &legal_arms);
                out.conversions.insert((narrow.clone(), re.rust.clone()));
                out.params.insert(
                    key,
                    NarrowParam {
                        narrow,
                        wide: re.rust.clone(),
                    },
                );
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
            let add = |owner: &str, field: &str, ty: &str, out: &mut AuthorNarrowing| {
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
                if let Some(check) = narrowed(&ir.ref_enums[target], ty) {
                    out.complex.push(ComplexAuthorNarrowing {
                        owner: owner.to_string(),
                        idx,
                        by: p.rust.clone(),
                        check,
                    });
                }
            };
            for r in &ent.redeclared_attrs {
                if let Some(owner) = r.owner.as_deref().and_then(|o| part_rust.get(o)) {
                    add(owner, &r.name, &r.ty, &mut out);
                }
            }
            for a in &ent.own_attrs {
                add(&p.rust, &a.name, &a.ty, &mut out);
            }
        }
        out.complex
            .sort_by(|a, b| (&a.owner, a.idx, &a.by).cmp(&(&b.owner, b.idx, &b.by)));
        out
    }

    /// The enum a constructor takes for narrowing `n`: a model enum with
    /// exactly those AP242-legal arms and complex parts, or a new `Ap242…Ref`
    /// named after edition 2's type.
    fn enum_for(
        &mut self,
        ir: &ModelIr,
        n: &RuntimeNarrowing,
        e2_type: &str,
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
        self.new_enums
            .insert(rust.clone(), (e2_type.to_string(), re));
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
            lines.push(format!(
                "complex {}#{} by {}: {} to {} arms",
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
