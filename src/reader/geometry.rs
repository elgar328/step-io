//! Geometry entity converters (Pass 4-1, 4-2, 4-3).

#![allow(clippy::unnecessary_lazy_evaluations, clippy::doc_markdown)]

use super::ReaderContext;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Pcurve, Surface, SurfaceOfOffset};
use crate::parser::entity::{Attribute, RawEntity};

impl ReaderContext {
    // ------------------------------------------------------------------
    // Pass 1: Points and directions
    // ------------------------------------------------------------------

    // ------------------------------------------------------------------
    // Pass 4-1: Simple leaf curves and surfaces
    // ------------------------------------------------------------------

    // ------------------------------------------------------------------
    // Pass 4-3: SURFACE_CURVE / SEAM_CURVE — transparent alias to curve_3d.
    //
    // STEP wraps edges on surfaces in `SURFACE_CURVE` (or its `SEAM_CURVE`
    // subtype) when the file carries pcurve data. The 3D curve is preserved
    // in the `curve_3d` slot; the associated pcurves are later resolved by
    // `collect_surface_curve_pcurves` (driven from `passes.rs`), using the
    // 2D arenas populated from `collect_pcurve_subtree_ids` via Pass 4a.
    //
    // We alias the surface/seam-curve id to the same `CurveId` as its
    // `curve_3d` so downstream code (e.g. `EDGE_CURVE`) keeps working.
    // ------------------------------------------------------------------

    // ------------------------------------------------------------------
    // Pass 4-3: Surfaces that reference curves
    // ------------------------------------------------------------------

    pub(super) fn convert_offset_surface(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        // Pass 4-4B (multi-round): skip entities already interned by a
        // previous round so the arena does not accumulate duplicates.
        if self.surface_map.contains_key(&entity_id) {
            return Ok(());
        }
        // STEP: OFFSET_SURFACE(name, basis_surface, distance, self_intersect)
        check_count(attrs, 4, entity_id, "OFFSET_SURFACE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let basis_ref = read_entity_ref(attrs, 1, entity_id, "basis_surface")?;
        let distance = read_real(attrs, 2, entity_id, "distance")?;
        // [3] self_intersect — informational LOGICAL, skipped (see ROADMAP).

        let basis = self.resolve_surface(entity_id, basis_ref, "basis_surface")?;

        let id = self
            .geometry
            .surfaces
            .push(Surface::Offset(SurfaceOfOffset { basis, distance }));
        self.surface_map.insert(entity_id, id);
        Ok(())
    }

    // ==================================================================
    // PCURVE resolution helper (used by Pass 4-3 extension)
    // ==================================================================
    //
    // Chain: PCURVE → (basis_surface, reference_to_curve) →
    //                DEFINITIONAL_REPRESENTATION → items[0] → 2D curve
    //
    // DEFINITIONAL_REPRESENTATION itself isn't stored in IR; we just
    // traverse through it. Returns `None` when any link is missing or
    // the referenced entity doesn't resolve — the caller treats `None`
    // as "skip this pcurve" and emits no warning (the underlying missing
    // reference would already surface in 3D pass errors).
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
