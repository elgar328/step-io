//! Geometry entity converters (Pass 4-1, 4-2, 4-3).

#![allow(clippy::unnecessary_lazy_evaluations, clippy::doc_markdown)]

use super::ReaderContext;
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref, read_entity_ref_list, read_enum, read_integer,
    read_integer_list, read_real, read_real_list, read_string,
};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::geometry::{
    Circle2, Curve2d, CurveForm, Ellipse2, Line2, NurbsCurve2d, Pcurve, Surface, SurfaceOfOffset,
};
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
    // Pass 4a: 2D geometry (PCURVE parametric space)
    // ==================================================================
    //
    // These converters mirror their 3D counterparts but target the 2D
    // arenas (`points_2d`, `directions_2d`, `curves_2d`). Dispatched only
    // on entities inside `pcurve_subtree_ids`, so a single entity name
    // like `CARTESIAN_POINT` can route to either the 3D or 2D converter
    // depending on whether the entity belongs to a PCURVE subtree.

    pub(super) fn convert_line_2d(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "LINE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pnt_ref = read_entity_ref(attrs, 1, entity_id, "pnt")?;
        let vec_ref = read_entity_ref(attrs, 2, entity_id, "dir")?;
        let point =
            *self
                .point_2d_map
                .get(&pnt_ref)
                .ok_or_else(|| ConvertError::MissingReference {
                    from: entity_id,
                    to: pnt_ref,
                    field_name: "pnt",
                })?;
        let (direction, magnitude) =
            *self
                .vector_2d_map
                .get(&vec_ref)
                .ok_or_else(|| ConvertError::MissingReference {
                    from: entity_id,
                    to: vec_ref,
                    field_name: "dir",
                })?;
        let id = self.geometry.curves_2d.push(Curve2d::Line(Line2 {
            point,
            direction,
            magnitude,
        }));
        self.curve_2d_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_circle_2d(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "CIRCLE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let radius = read_real(attrs, 2, entity_id, "radius")?;
        let position =
            *self
                .placement_2d_map
                .get(&pos_ref)
                .ok_or_else(|| ConvertError::MissingReference {
                    from: entity_id,
                    to: pos_ref,
                    field_name: "position",
                })?;
        let id = self
            .geometry
            .curves_2d
            .push(Curve2d::Circle(Circle2 { position, radius }));
        self.curve_2d_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_ellipse_2d(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "ELLIPSE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let semi_axis_1 = read_real(attrs, 2, entity_id, "semi_axis_1")?;
        let semi_axis_2 = read_real(attrs, 3, entity_id, "semi_axis_2")?;
        let position =
            *self
                .placement_2d_map
                .get(&pos_ref)
                .ok_or_else(|| ConvertError::MissingReference {
                    from: entity_id,
                    to: pos_ref,
                    field_name: "position",
                })?;
        let id = self.geometry.curves_2d.push(Curve2d::Ellipse(Ellipse2 {
            position,
            semi_axis_1,
            semi_axis_2,
        }));
        self.curve_2d_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_bspline_curve_2d_with_knots(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 9, entity_id, "B_SPLINE_CURVE_WITH_KNOTS")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let degree_i = read_integer(attrs, 1, entity_id, "degree")?;
        let cp_refs = read_entity_ref_list(attrs, 2, entity_id, "control_points_list")?;
        let form = CurveForm::from_step_enum(read_enum(attrs, 3, entity_id, "curve_form")?);
        let closed = read_bool(attrs, 4, entity_id, "closed_curve")?;
        let knot_multiplicities = read_integer_list(attrs, 6, entity_id, "knot_multiplicities")?;
        let knots = read_real_list(attrs, 7, entity_id, "knots")?;

        let degree = u32::try_from(degree_i).map_err(|_| ConvertError::AttributeType {
            entity_id,
            field_name: "degree",
            expected: "non-negative Integer",
            actual: AttributeKindTag::Integer,
        })?;

        let mut control_points = Vec::with_capacity(cp_refs.len());
        for &r in &cp_refs {
            let pt = *self
                .point_2d_map
                .get(&r)
                .ok_or_else(|| ConvertError::MissingReference {
                    from: entity_id,
                    to: r,
                    field_name: "control_points_list",
                })?;
            control_points.push(pt);
        }

        let id = self.geometry.curves_2d.push(Curve2d::Nurbs(NurbsCurve2d {
            degree,
            control_points,
            weights: None,
            knot_multiplicities,
            knots,
            closed,
            form,
        }));
        self.curve_2d_map.insert(entity_id, id);
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
