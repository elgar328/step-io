//! Geometry helpers that did not migrate into `entities/` modules.
//!
//! After Plan 7 the file holds a single helper: `resolve_pcurve`. It
//! walks `PCURVE → DEFINITIONAL_REPRESENTATION → 2D curve` and is called
//! from `entities/geometry/surface_curve.rs` while it post-processes the
//! pcurves attached to a `SURFACE_CURVE` / `SEAM_CURVE`. Hosting it on
//! `ReaderContext` (rather than inside the surface_curve handler) keeps
//! the helper available to any future entity that needs the same
//! traversal — e.g. a Plan 7+ IR Roadmap rewrite of pcurve handling.

#![allow(clippy::unnecessary_lazy_evaluations, clippy::doc_markdown)]

use super::ReaderContext;
use crate::ir::geometry::Pcurve;
use crate::parser::entity::{Attribute, RawEntity};

impl ReaderContext {
    // ==================================================================
    // PCURVE resolution helper (used by Pass 4-3 extension)
    // ==================================================================
    //
    // Chain: PCURVE → (basis_surface, reference_to_curve) →
    //                DEFINITIONAL_REPRESENTATION → items[0] → 2D curve
    //
    // DEFINITIONAL_REPRESENTATION itself isn't stored in IR; we just
    // traverse through it. Returns `None` when any link is missing or
    // the referenced entity doesn't resolve — the caller
    // (`collect_surface_curve_pcurves`) emits a warning so the dropped
    // pcurve is visible in reader diagnostics.
    pub(crate) fn resolve_pcurve(
        &self,
        pcurve_ref: u64,
        graph: &crate::parser::entity::EntityGraph,
    ) -> Option<Pcurve> {
        let RawEntity::Simple {
            name, attributes, ..
        } = graph.get(pcurve_ref)?
        else {
            return None;
        };
        if name != "PCURVE" {
            return None;
        }
        let Attribute::EntityRef(basis_surface_ref) = attributes.get(1)? else {
            return None;
        };
        let Attribute::EntityRef(def_repr_ref) = attributes.get(2)? else {
            return None;
        };

        let basis_surface = *self.surface_map.get(basis_surface_ref)?;

        let RawEntity::Simple {
            name: def_name,
            attributes: def_attrs,
            ..
        } = graph.get(*def_repr_ref)?
        else {
            return None;
        };
        if def_name != "DEFINITIONAL_REPRESENTATION" {
            return None;
        }
        let Attribute::List(items) = def_attrs.get(1)? else {
            return None;
        };
        let Attribute::EntityRef(first_item_ref) = items.first()? else {
            return None;
        };
        let curve_2d = *self.curve_2d_map.get(first_item_ref)?;

        Some(Pcurve {
            basis_surface,
            curve_2d,
        })
    }
}
