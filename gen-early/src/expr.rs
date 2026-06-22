//! Per-attribute codegen expressions: field types, bind expressions, and
//! serialize expressions (incl. the optional-aware variants).

use crate::classify::{Ctx, Kind, pascal};

pub(crate) fn field_ty(ctx: &Ctx, k: &Kind) -> String {
    match k {
        Kind::Ref => "u64".into(),
        Kind::Vec(inner) => format!("Vec<{}>", field_ty(ctx, inner)),
        Kind::Real => "f64".into(),
        Kind::Int => "i64".into(),
        Kind::Bool => "bool".into(),
        Kind::Logical => "crate::ir::geometry::Logical".into(),
        // binary carries its hex digits verbatim, as a String.
        Kind::Str | Kind::Binary => "String".into(),
        // Hinted ENUM reuses the L2 type; hint-less ENUM uses the synthesized
        // `Early*` (defined in the same generated/model.rs, so referenced bare).
        Kind::Enum(alias) => match ctx.mapping.enums.get(alias) {
            Some(h) => h.rust_type.clone(),
            None => format!("Early{}", pascal(alias)),
        },
        // Hinted SELECT reuses the hint's type; hint-less SELECT uses the
        // synthesized `Early*` (parallel to the ENUM branch).
        Kind::Select(alias) => match ctx.mapping.selects.get(alias) {
            Some(h) => h.rust_type.clone(),
            None => format!("Early{}", pascal(alias)),
        },
    }
}

pub(crate) fn bind_expr(k: &Kind, i: usize, field: &str) -> String {
    match k {
        Kind::Ref => {
            format!("crate::ir::attr::read_entity_ref(attrs, {i}, entity_id, \"{field}\")?")
        }
        Kind::Vec(inner) => match &**inner {
            // ENUM / SELECT list -> generated `<alias>_list` helper (see
            // `emit_enum_list` / `emit_select_list`).
            Kind::Enum(alias) | Kind::Select(alias) => {
                format!("{alias}_list(attrs, {i}, entity_id, \"{field}\")?")
            }
            // LOGICAL list -> single generated `logical_list` helper.
            Kind::Logical => format!("logical_list(attrs, {i}, entity_id, \"{field}\")?"),
            Kind::Ref | Kind::Real | Kind::Int | Kind::Str => {
                let list = match &**inner {
                    Kind::Ref => "read_entity_ref_list",
                    Kind::Real => "read_real_list",
                    Kind::Int => "read_integer_list",
                    Kind::Str => "read_string_list",
                    _ => unreachable!(),
                };
                format!("crate::ir::attr::{list}(attrs, {i}, entity_id, \"{field}\")?")
            }
            // `Vec<Vec<ref/real/int>>` -> grid helper (List of Lists),
            // `Vec<Vec<select>>` -> generated `<alias>_grid` helper (v4.21), or
            // `Vec<Vec<Vec<ref/real>>>` -> grid3 helper (v4.17).
            Kind::Vec(i2) => {
                if let Kind::Select(alias) = &**i2 {
                    return format!("{alias}_grid(attrs, {i}, entity_id, \"{field}\")?");
                }
                let grid = match &**i2 {
                    Kind::Ref => "read_entity_ref_grid",
                    Kind::Real => "read_real_grid",
                    Kind::Int => "read_integer_grid",
                    Kind::Vec(i3) => match &**i3 {
                        Kind::Ref => "read_entity_ref_grid3",
                        Kind::Real => "read_real_grid3",
                        other => panic!("gen-early: 3D grid of {other:?} not yet supported"),
                    },
                    other => panic!("gen-early: grid of {other:?} not yet supported"),
                };
                format!("crate::ir::attr::{grid}(attrs, {i}, entity_id, \"{field}\")?")
            }
            other => panic!("gen-early: aggregation of {other:?} not yet supported"),
        },
        Kind::Real => format!("crate::ir::attr::read_real(attrs, {i}, entity_id, \"{field}\")?"),
        Kind::Int => format!("crate::ir::attr::read_integer(attrs, {i}, entity_id, \"{field}\")?"),
        Kind::Bool => format!("crate::ir::attr::read_bool(attrs, {i}, entity_id, \"{field}\")?"),
        Kind::Logical => {
            format!("crate::ir::attr::read_logical(attrs, {i}, entity_id, \"{field}\")?")
        }
        Kind::Str => {
            format!(
                "crate::ir::attr::read_string_or_unset(attrs, {i}, entity_id, \"{field}\")?.to_owned()"
            )
        }
        Kind::Binary => {
            format!("crate::ir::attr::read_binary(attrs, {i}, entity_id, \"{field}\")?.to_owned()")
        }
        Kind::Enum(alias) => format!("bind_{alias}(attrs, {i}, entity_id, \"{field}\")?"),
        // Select fields drop the whole entity on `None`, so they are emitted in
        // the let-form path (see `main`), never via this expression helper.
        Kind::Select(_) => unreachable!("Select handled in the let-form bind path"),
    }
}

/// The per-element closure that maps one `Vec<inner>` element to an
/// [`Attribute`], shared by `serialize_expr` (non-optional) and
/// `serialize_expr_opt` (optional). The `Ref` arm reproduces the previous
/// `VecRef` output byte-for-byte; `ENUM`/`SELECT` reuse the standalone
/// `<alias>_attr` / `<alias>_emit` helpers per element.
pub(crate) fn vec_elem_closure(inner: &Kind) -> String {
    match inner {
        Kind::Ref => "|&s| crate::parser::entity::Attribute::EntityRef(s)".to_string(),
        Kind::Real => "|&x| crate::parser::entity::Attribute::Real(x)".to_string(),
        Kind::Int => "|&x| crate::parser::entity::Attribute::Integer(x)".to_string(),
        Kind::Str => "|s| crate::parser::entity::Attribute::String(s.clone())".to_string(),
        Kind::Enum(alias) => format!("|e| {alias}_attr(e.clone())"),
        Kind::Select(alias) => format!("{alias}_emit"),
        Kind::Logical => {
            "|&l| crate::parser::entity::Attribute::Enum(crate::ir::attr::logical_to_step(l).into())"
                .to_string()
        }
        other => panic!("gen-early: aggregation of {other:?} not yet supported"),
    }
}

pub(crate) fn serialize_expr(k: &Kind, field: &str) -> String {
    match k {
        Kind::Ref => format!("crate::parser::entity::Attribute::EntityRef(l1.{field})"),
        // `Vec<Vec<ref/real/int>>` grid -> nested `List(List(..))`, or
        // `Vec<Vec<Vec<ref/real>>>` 3D grid -> `List(List(List(..)))` (v4.17).
        Kind::Vec(inner) if matches!(&**inner, Kind::Vec(_)) => {
            let Kind::Vec(i2) = &**inner else {
                unreachable!()
            };
            if let Kind::Vec(i3) = &**i2 {
                // 3D grid (v4.17) — separate branch; the 2D path below is kept
                // byte-for-byte unchanged.
                let elem3 = match &**i3 {
                    Kind::Ref => "|&s| crate::parser::entity::Attribute::EntityRef(s)",
                    Kind::Real => "|&x| crate::parser::entity::Attribute::Real(x)",
                    other => panic!("gen-early: 3D grid of {other:?} not yet supported"),
                };
                format!(
                    "crate::parser::entity::Attribute::List(l1.{field}.iter().map(|plane| crate::parser::entity::Attribute::List(plane.iter().map(|row| crate::parser::entity::Attribute::List(row.iter().map({elem3}).collect())).collect())).collect())"
                )
            } else {
                let elem2 = match &**i2 {
                    Kind::Ref => "|&s| crate::parser::entity::Attribute::EntityRef(s)".to_string(),
                    Kind::Real => "|&x| crate::parser::entity::Attribute::Real(x)".to_string(),
                    Kind::Int => "|&x| crate::parser::entity::Attribute::Integer(x)".to_string(),
                    // grid of a mixed select: per-element `<alias>_emit` (v4.21).
                    Kind::Select(alias) => format!("{alias}_emit"),
                    other => panic!("gen-early: grid of {other:?} not yet supported"),
                };
                format!(
                    "crate::parser::entity::Attribute::List(l1.{field}.iter().map(|row| crate::parser::entity::Attribute::List(row.iter().map({elem2}).collect())).collect())"
                )
            }
        }
        Kind::Vec(inner) => {
            let elem = vec_elem_closure(inner);
            format!(
                "crate::parser::entity::Attribute::List(l1.{field}.iter().map({elem}).collect())"
            )
        }
        Kind::Real => format!("crate::parser::entity::Attribute::Real(l1.{field})"),
        Kind::Int => format!("crate::parser::entity::Attribute::Integer(l1.{field})"),
        Kind::Bool => format!("bool_attr(l1.{field})"),
        Kind::Logical => format!(
            "crate::parser::entity::Attribute::Enum(crate::ir::attr::logical_to_step(l1.{field}).into())"
        ),
        Kind::Str => format!("crate::parser::entity::Attribute::String(l1.{field}.clone())"),
        Kind::Binary => format!("crate::parser::entity::Attribute::Binary(l1.{field}.clone())"),
        Kind::Enum(alias) => format!("{alias}_attr(l1.{field})"),
        Kind::Select(alias) => format!("{alias}_emit(&l1.{field})"),
    }
}

/// Field type, wrapping `Option<…>` for an `OPTIONAL` schema attribute (faithful
/// L1 optionality; `lower` decides any collapse).
pub(crate) fn field_ty_full(ctx: &Ctx, k: &Kind, optional: bool) -> String {
    let base = field_ty(ctx, k);
    if optional {
        format!("Option<{base}>")
    } else {
        base
    }
}

/// `bind` expression, optional-aware (non-`Select` kinds; `Select` is emitted
/// inline in `main`'s let-form path, never here).
pub(crate) fn bind_expr_full(k: &Kind, i: usize, field: &str, optional: bool) -> String {
    if optional {
        bind_expr_opt(k, i, field)
    } else {
        bind_expr(k, i, field)
    }
}

/// Optional `bind` for kinds with an existing `read_optional_*` helper. Other
/// kinds (`Bool`/`Logical`/`Enum`/`Vec`) are deferred — they need new helpers
/// that would be dead code until an entity uses them.
pub(crate) fn bind_expr_opt(k: &Kind, i: usize, field: &str) -> String {
    match k {
        Kind::Ref => {
            format!(
                "crate::ir::attr::read_optional_entity_ref(attrs, {i}, entity_id, \"{field}\")?"
            )
        }
        Kind::Real => {
            format!("crate::ir::attr::read_optional_real(attrs, {i}, entity_id, \"{field}\")?")
        }
        Kind::Int => {
            format!("crate::ir::attr::read_optional_integer(attrs, {i}, entity_id, \"{field}\")?")
        }
        Kind::Str => {
            format!("crate::ir::attr::read_optional_string(attrs, {i}, entity_id, \"{field}\")?")
        }
        // Optional enum: `$`/`*` -> None, else the standalone `bind_<alias>`.
        Kind::Enum(alias) => format!(
            "match attrs.get({i}) {{ Some(crate::parser::entity::Attribute::Unset | crate::parser::entity::Attribute::Derived) => None, _ => Some(bind_{alias}(attrs, {i}, entity_id, \"{field}\")?) }}"
        ),
        // Optional single-level aggregation: `$`/`*` -> None, else the existing
        // non-optional list bind (`read_*_list` / `<alias>_list`), unchanged.
        Kind::Vec(_) => format!(
            "match attrs.get({i}) {{ Some(crate::parser::entity::Attribute::Unset | crate::parser::entity::Attribute::Derived) => None, _ => Some({}) }}",
            bind_expr(k, i, field)
        ),
        Kind::Bool | Kind::Logical | Kind::Binary => {
            panic!("gen-early: OPTIONAL {k:?} not yet supported (v4.x)")
        }
        Kind::Select(_) => unreachable!("optional Select handled in the let-form bind path"),
    }
}

/// `serialize` expression, optional-aware: `Some` -> inner attribute, `None` ->
/// `$` (`Attribute::Unset`).
pub(crate) fn serialize_expr_full(k: &Kind, field: &str, optional: bool) -> String {
    if optional {
        serialize_expr_opt(k, field)
    } else {
        serialize_expr(k, field)
    }
}

pub(crate) fn serialize_expr_opt(k: &Kind, field: &str) -> String {
    let unset = "crate::parser::entity::Attribute::Unset";
    match k {
        Kind::Ref => format!(
            "match l1.{field} {{ Some(v) => crate::parser::entity::Attribute::EntityRef(v), None => {unset} }}"
        ),
        Kind::Real => format!(
            "match l1.{field} {{ Some(v) => crate::parser::entity::Attribute::Real(v), None => {unset} }}"
        ),
        Kind::Int => format!(
            "match l1.{field} {{ Some(v) => crate::parser::entity::Attribute::Integer(v), None => {unset} }}"
        ),
        Kind::Str => format!(
            "match &l1.{field} {{ Some(v) => crate::parser::entity::Attribute::String(v.clone()), None => {unset} }}"
        ),
        Kind::Select(alias) => {
            format!("match &l1.{field} {{ Some(v) => {alias}_emit(v), None => {unset} }}")
        }
        Kind::Enum(alias) => {
            format!("match &l1.{field} {{ Some(e) => {alias}_attr(e.clone()), None => {unset} }}")
        }
        // Optional single-level aggregation: `Some` -> `List`, `None` -> `$`.
        // (opt-grid is rejected by `emittable`, so `inner` is never `Vec`.)
        Kind::Vec(inner) => {
            let elem = vec_elem_closure(inner);
            format!(
                "match &l1.{field} {{ Some(v) => crate::parser::entity::Attribute::List(v.iter().map({elem}).collect()), None => {unset} }}"
            )
        }
        Kind::Bool | Kind::Logical | Kind::Binary => {
            panic!("gen-early: OPTIONAL {k:?} not yet supported (v4.x)")
        }
    }
}
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::schema::EarlyToml;
    use crate::testutil::*;

    /// Faithful `Option<T>` codegen for the supported kinds.
    #[test]
    fn optional_codegen() {
        let ctx = ctx_from(EarlyToml {
            entity: BTreeMap::new(),
            types: BTreeMap::new(),
        });
        // field wraps Option only when optional.
        assert_eq!(field_ty_full(&ctx, &Kind::Real, true), "Option<f64>");
        assert_eq!(field_ty_full(&ctx, &Kind::Real, false), "f64");
        // bind uses the read_optional_* helpers.
        assert!(bind_expr_full(&Kind::Int, 2, "x", true).contains("read_optional_integer"));
        assert!(bind_expr_full(&Kind::Ref, 3, "c", true).contains("read_optional_entity_ref"));
        // required path is unchanged.
        assert!(bind_expr_full(&Kind::Int, 2, "x", false).contains("read_integer(attrs"));
        // serialize maps None -> Unset.
        let s = serialize_expr_full(&Kind::Ref, "c", true);
        assert!(s.contains("Some(v) => crate::parser::entity::Attribute::EntityRef(v)"));
        assert!(s.contains("None => crate::parser::entity::Attribute::Unset"));
    }

    /// `Option<EarlyEnum>` (v4.10): wraps the standalone `bind_<alias>` /
    /// `<alias>_attr` with an `$`/`*` -> None check.
    #[test]
    fn optional_enum_codegen() {
        let ctx = empty_ctx();
        assert!(ctx.emittable(&Kind::Enum("x".into()), true)); // optional enum now ok
        assert!(!ctx.emittable(&Kind::Bool, true)); // optional bool still deferred
        let b = bind_expr_full(&Kind::Enum("foo".into()), 1, "x", true);
        assert!(b.contains("bind_foo(attrs, 1, entity_id"));
        assert!(b.contains("crate::parser::entity::Attribute::Unset"));
        assert!(b.contains("=> None,"));
        let s = serialize_expr_full(&Kind::Enum("foo".into()), "y", true);
        assert!(s.contains("Some(e) => foo_attr(e.clone())"));
        assert!(s.contains("None => crate::parser::entity::Attribute::Unset"));
    }

    /// `Vec<T>` codegen for scalar inner; the ref arm stays byte-identical.
    #[test]
    fn vec_scalar_codegen() {
        let ctx = empty_ctx();
        assert_eq!(field_ty(&ctx, &Kind::Vec(Box::new(Kind::Real))), "Vec<f64>");
        assert_eq!(
            field_ty(&ctx, &Kind::Vec(Box::new(Kind::Str))),
            "Vec<String>"
        );
        assert!(bind_expr(&Kind::Vec(Box::new(Kind::Real)), 1, "x").contains("read_real_list"));
        assert!(bind_expr(&Kind::Vec(Box::new(Kind::Int)), 1, "x").contains("read_integer_list"));
        assert!(bind_expr(&Kind::Vec(Box::new(Kind::Str)), 1, "x").contains("read_string_list"));
        // ref arm unchanged.
        assert!(
            bind_expr(&Kind::Vec(Box::new(Kind::Ref)), 1, "x").contains("read_entity_ref_list")
        );
        let sr = serialize_expr(&Kind::Vec(Box::new(Kind::Ref)), "items");
        assert!(sr.contains("|&s| crate::parser::entity::Attribute::EntityRef(s)"));
        let ss = serialize_expr(&Kind::Vec(Box::new(Kind::Str)), "labels");
        assert!(ss.contains("|s| crate::parser::entity::Attribute::String(s.clone())"));
        assert!(serialize_expr(&Kind::Vec(Box::new(Kind::Real)), "v").contains("Real(x)"));
    }

    /// `Option<Vec<inner>>` (optional single-level aggregation): presence-check
    /// wrapper reuses the non-optional list bind/serialize; the grid stays
    /// non-optional only.
    #[test]
    fn opt_agg_codegen() {
        let ctx = empty_ctx();
        // optional single-level aggregations are now emittable...
        assert!(ctx.emittable(&Kind::Vec(Box::new(Kind::Ref)), true));
        assert!(ctx.emittable(&Kind::Vec(Box::new(Kind::Str)), true));
        // ...but an optional grid stays deferred.
        assert!(!ctx.emittable(&Kind::Vec(Box::new(Kind::Vec(Box::new(Kind::Ref)))), true));

        // opt-vec-ref bind: presence-check around the existing list reader.
        let b = bind_expr_full(&Kind::Vec(Box::new(Kind::Ref)), 1, "x", true);
        assert!(b.contains("read_entity_ref_list(attrs, 1"));
        assert!(b.contains("crate::parser::entity::Attribute::Unset"));
        assert!(b.contains("=> None"));
        assert!(b.contains("Some("));
        // opt-vec-ref serialize: Some -> List, None -> Unset.
        let s = serialize_expr_full(&Kind::Vec(Box::new(Kind::Ref)), "y", true);
        assert!(s.contains("Some(v) =>"));
        assert!(s.contains("v.iter().map(|&s| crate::parser::entity::Attribute::EntityRef(s))"));
        assert!(s.contains("None => crate::parser::entity::Attribute::Unset"));

        // opt-vec-select reuses the generated `<alias>_list` / `<alias>_emit`.
        let bs = bind_expr_full(
            &Kind::Vec(Box::new(Kind::Select("foo".into()))),
            1,
            "x",
            true,
        );
        assert!(bs.contains("foo_list(attrs, 1"));
        let ss = serialize_expr_full(&Kind::Vec(Box::new(Kind::Select("foo".into()))), "y", true);
        assert!(ss.contains("v.iter().map(foo_emit)"));
    }

    /// `binary` primitive: classifies as `Kind::Binary`, maps to `String` (hex),
    /// binds via `read_binary`, serializes to `Attribute::Binary`. Optional is
    /// deferred (no `read_optional_binary`).
    #[test]
    fn binary_codegen() {
        let ctx = empty_ctx();
        assert!(matches!(ctx.classify("binary", 0), Kind::Binary));
        assert!(matches!(ctx.try_classify("binary", 0), Some(Kind::Binary)));
        assert!(ctx.emittable(&Kind::Binary, false));
        assert!(!ctx.emittable(&Kind::Binary, true)); // optional binary deferred
        assert_eq!(field_ty(&ctx, &Kind::Binary), "String");
        let b = bind_expr(&Kind::Binary, 1, "x");
        assert!(b.contains("crate::ir::attr::read_binary(attrs, 1, entity_id, \"x\")?"));
        assert!(b.contains(".to_owned()"));
        assert_eq!(
            serialize_expr(&Kind::Binary, "y"),
            "crate::parser::entity::Attribute::Binary(l1.y.clone())"
        );
    }

    /// `Vec<Vec<ref/real/int>>` grid -> `read_*_grid` + nested `List(List(..))`.
    #[test]
    fn vec_grid_codegen() {
        let ctx = empty_ctx();
        let grid = |k| Kind::Vec(Box::new(Kind::Vec(Box::new(k))));
        assert!(ctx.emittable(&grid(Kind::Real), false));
        assert!(ctx.emittable(&grid(Kind::Ref), false));
        // inner-inner must be ref/real/int (no string/enum grid helper).
        assert!(!ctx.emittable(&grid(Kind::Str), false));
        assert!(!ctx.emittable(&grid(Kind::Enum("x".into())), false));
        assert!(bind_expr(&grid(Kind::Real), 1, "x").contains("read_real_grid"));
        assert!(bind_expr(&grid(Kind::Int), 1, "x").contains("read_integer_grid"));
        assert!(bind_expr(&grid(Kind::Ref), 1, "x").contains("read_entity_ref_grid"));
        let s = serialize_expr(&grid(Kind::Int), "g");
        assert_eq!(
            s.matches("crate::parser::entity::Attribute::List(").count(),
            2
        );
        assert!(s.contains("Integer(x)"));
    }

    /// `Vec<Vec<Vec<ref/real>>>` 3D grid -> `read_*_grid3` + `List(List(List(..)))`
    /// (B-spline volume control points / weights). ref/real only; int deferred.
    #[test]
    fn vec_grid3_codegen() {
        let ctx = empty_ctx();
        let grid3 = |k| Kind::Vec(Box::new(Kind::Vec(Box::new(Kind::Vec(Box::new(k))))));
        assert!(ctx.emittable(&grid3(Kind::Ref), false));
        assert!(ctx.emittable(&grid3(Kind::Real), false));
        // int / str 3D grids have no helper; optional 3D grid deferred.
        assert!(!ctx.emittable(&grid3(Kind::Int), false));
        assert!(!ctx.emittable(&grid3(Kind::Str), false));
        assert!(!ctx.emittable(&grid3(Kind::Ref), true));
        // bind dispatches to the grid3 helpers.
        assert!(bind_expr(&grid3(Kind::Ref), 1, "x").contains("read_entity_ref_grid3"));
        assert!(bind_expr(&grid3(Kind::Real), 1, "x").contains("read_real_grid3"));
        // serialize nests three `List(` levels.
        let sr = serialize_expr(&grid3(Kind::Ref), "g");
        assert_eq!(
            sr.matches("crate::parser::entity::Attribute::List(")
                .count(),
            3
        );
        assert!(sr.contains("|&s| crate::parser::entity::Attribute::EntityRef(s)"));
        assert!(
            serialize_expr(&grid3(Kind::Real), "g")
                .contains("|&x| crate::parser::entity::Attribute::Real(x)")
        );
    }

    /// STEP `logical` (.T./.F./.U.) -> `crate::ir::geometry::Logical`.
    #[test]
    fn logical_codegen() {
        let ctx = empty_ctx();
        assert!(matches!(ctx.classify("logical", 0), Kind::Logical));
        assert_eq!(
            field_ty(&ctx, &Kind::Logical),
            "crate::ir::geometry::Logical"
        );
        assert!(bind_expr(&Kind::Logical, 1, "x").contains("read_logical"));
        assert!(serialize_expr(&Kind::Logical, "x").contains("logical_to_step"));
    }
}
