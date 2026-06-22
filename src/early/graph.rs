//! `EarlyGraph` — a typed read facade over the raw [`EntityGraph`](crate::parser).
//!
//! L2-building code (handlers, `lower`) cross-walks OTHER entities through this
//! facade's typed accessors (each folds `get` + name-guard + the generated
//! strict `bind` into one call returning an owned `Early*`) instead of touching
//! `RawEntity` directly. No leniency is added — accessors use the same strict
//! `bind`, so the L1 invariant holds. Non-standard diagnostic readers that must
//! inspect malformed raw input keep `&EntityGraph` directly (they are not trait
//! `read` fns); a `raw()` escape is added when the first such caller migrates.
//!
//! Accessors are added as cross-walk sites migrate — only the entities actually
//! cross-walked get one (not the whole 322-entity schema).

use crate::ir::error::ConvertError;
use crate::parser::entity::{EntityGraph, RawEntity};

use super::bind;
use super::model::{
    EarlyDefinitionalRepresentation, EarlyPcurve, EarlyRepresentation,
    EarlyRepresentationRelationshipWithTransformation,
};

/// Typed front door to the L1 bind layer over a borrowed raw graph. `Copy` —
/// constructed on demand at each cross-walk (`EarlyGraph::new(graph)`).
#[derive(Clone, Copy)]
pub(crate) struct EarlyGraph<'a> {
    raw: &'a EntityGraph,
}

impl<'a> EarlyGraph<'a> {
    pub(crate) fn new(raw: &'a EntityGraph) -> Self {
        Self { raw }
    }

    /// The wrapped raw graph — a reader-internal escape for the few raw walks the
    /// typed facade can't express (e.g. `record_pcurve_wr3_drop`'s generic
    /// ref-subtree collection). Banned in `entities/` + `early/lower/` by the
    /// `no_raw_graph_access_in_l2_build` drift test; only `reader/` may call it.
    pub(crate) fn raw(self) -> &'a EntityGraph {
        self.raw
    }

    /// Whether an entity id is present in the graph (dangling-reference probe).
    pub(crate) fn exists(self, id: u64) -> bool {
        self.raw.get(id).is_some()
    }

    /// Type name of a `Simple` entity, or `None` if absent / `Complex`. Used to
    /// discriminate a shared entity name (e.g. `REPRESENTATION` vs a subtype)
    /// before binding.
    pub(crate) fn type_name(self, id: u64) -> Option<&'a str> {
        match self.raw.get(id)? {
            RawEntity::Simple { name, .. } => Some(name.as_str()),
            RawEntity::Complex { .. } => None,
        }
    }

    /// Bind a `PCURVE` (exact name). `None` when absent / not `PCURVE` / the
    /// strict bind rejects it (caller treats any failure as a dropped pcurve).
    pub(crate) fn pcurve(self, id: u64) -> Option<EarlyPcurve> {
        let RawEntity::Simple {
            name, attributes, ..
        } = self.raw.get(id)?
        else {
            return None;
        };
        if name != "PCURVE" {
            return None;
        }
        bind::bind_pcurve(id, attributes).ok()
    }

    /// Bind a `DEFINITIONAL_REPRESENTATION` (exact name). `None` on absent /
    /// wrong-name / bind failure.
    pub(crate) fn definitional_representation(
        self,
        id: u64,
    ) -> Option<EarlyDefinitionalRepresentation> {
        let RawEntity::Simple {
            name, attributes, ..
        } = self.raw.get(id)?
        else {
            return None;
        };
        if name != "DEFINITIONAL_REPRESENTATION" {
            return None;
        }
        bind::bind_definitional_representation(id, attributes).ok()
    }

    /// Bind the RRWT complex `(REPRESENTATION_RELATIONSHIP +
    /// REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION +
    /// SHAPE_REPRESENTATION_RELATIONSHIP)`. Outer `None` = not a `Complex` /
    /// missing a required part (the empty SRR part is guarded by `has_all_parts`,
    /// which the strict bind does not check). `Some(Err)` = bind defect;
    /// `Some(Ok(None))` = unrecognized `transformation` SELECT member.
    pub(crate) fn representation_relationship_with_transformation(
        self,
        id: u64,
    ) -> Option<Result<Option<EarlyRepresentationRelationshipWithTransformation>, ConvertError>>
    {
        let RawEntity::Complex { parts, .. } = self.raw.get(id)? else {
            return None;
        };
        if !crate::reader::has_all_parts(
            parts,
            &[
                "REPRESENTATION_RELATIONSHIP",
                "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
                "SHAPE_REPRESENTATION_RELATIONSHIP",
            ],
        ) {
            return None;
        }
        Some(bind::bind_representation_relationship_with_transformation(
            id, parts,
        ))
    }

    /// Bind a bare `REPRESENTATION` (exact name). `None` when absent or not
    /// exactly `REPRESENTATION`; `Some(Err)` propagates a strict-bind defect
    /// (incl. `unset_required_field` for the c3d `context_of_items=$` drop).
    pub(crate) fn representation(
        self,
        id: u64,
    ) -> Option<Result<EarlyRepresentation, ConvertError>> {
        let RawEntity::Simple {
            name, attributes, ..
        } = self.raw.get(id)?
        else {
            return None;
        };
        if name != "REPRESENTATION" {
            return None;
        }
        Some(bind::bind_representation(id, attributes))
    }
}

#[cfg(test)]
mod tests {
    /// L2-building code (`entities/`, `early/lower/`) must cross-walk OTHER
    /// entities through [`EarlyGraph`] typed accessors, never the raw graph
    /// directly. This scans those trees for `graph.get(` / `RawEntity::` in
    /// non-comment lines and fails on any raw access — the read-side counterpart
    /// of `nonstandard_input_only_via_funnel`. No exceptions: L2-build code does
    /// not touch the raw graph (non-standard required-field drops go through the
    /// strict `bind` + dispatch `unset_required_field` arm, not a raw probe).
    #[test]
    fn no_raw_graph_access_in_l2_build() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        for sub in ["entities", "early/lower"] {
            visit(&root.join(sub), &mut offenders);
        }
        assert!(
            offenders.is_empty(),
            "raw graph access in L2-build code (route through EarlyGraph instead): {offenders:?}"
        );
    }

    fn visit(dir: &std::path::Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, out);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let rel = path.to_string_lossy().replace('\\', "/");
            if rel.contains("tests") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for line in text.lines() {
                let code = line.trim_start();
                if code.starts_with("//") || code.starts_with('*') || code.starts_with("/*") {
                    continue;
                }
                if line.contains("graph.get(")
                    || line.contains("RawEntity::")
                    || line.contains(".raw(")
                {
                    out.push(format!("{rel}: {}", code.trim_end()));
                    break;
                }
            }
        }
    }
}
