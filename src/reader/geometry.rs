//! Geometry entity converters (Pass 4-1, 4-2, 4-3).

#![allow(clippy::unnecessary_lazy_evaluations, clippy::doc_markdown)]

use super::{ReaderContext, require_part_attrs};
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref, read_entity_ref_grid, read_entity_ref_list, read_enum,
    read_integer, read_integer_list, read_optional_entity_ref, read_real, read_real_grid,
    read_real_list, read_string,
};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::geometry::{
    Axis1Placement, Axis2Placement2d, Axis2Placement3d, Circle2, Circle3, ConicalSurface, Curve,
    Curve2d, CurveForm, CylindricalSurface, Direction2, Direction3, Ellipse2, Ellipse3, Line2,
    Line3, NurbsCurve, NurbsCurve2d, NurbsSurface, Pcurve, Plane3, Point2, Point3,
    SphericalSurface, Surface, SurfaceForm, SurfaceOfLinearExtrusion, SurfaceOfRevolution,
    ToroidalSurface,
};
use crate::parser::entity::{Attribute, RawEntity, RawEntityPart};

impl ReaderContext {
    // ------------------------------------------------------------------
    // Pass 1: Points and directions
    // ------------------------------------------------------------------

    pub(super) fn convert_cartesian_point(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "CARTESIAN_POINT")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let coords = read_real_list(attrs, 1, entity_id, "coordinates")?;
        if coords.len() != 3 {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "coordinates",
                expected: 3,
                actual: coords.len(),
            });
        }
        let point = Point3 {
            x: coords[0],
            y: coords[1],
            z: coords[2],
        };
        let id = self.geometry.points.push(point);
        self.point_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_direction(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "DIRECTION")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let ratios = read_real_list(attrs, 1, entity_id, "direction_ratios")?;
        if ratios.len() != 3 {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "direction_ratios",
                expected: 3,
                actual: ratios.len(),
            });
        }
        let dir = Direction3 {
            x: ratios[0],
            y: ratios[1],
            z: ratios[2],
        };
        let id = self.geometry.directions.push(dir);
        self.direction_map.insert(entity_id, id);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 2: VECTOR (depends on DIRECTION)
    // ------------------------------------------------------------------

    pub(super) fn convert_vector(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "VECTOR")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let dir_ref = read_entity_ref(attrs, 1, entity_id, "orientation")?;
        let magnitude = read_real(attrs, 2, entity_id, "magnitude")?;

        let dir_id = self.resolve_direction(entity_id, dir_ref, "orientation")?;

        self.vector_map.insert(entity_id, (dir_id, magnitude));
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 3: Axis placements (depend on POINT + DIRECTION)
    // ------------------------------------------------------------------

    pub(super) fn convert_axis2_placement_3d(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "AXIS2_PLACEMENT_3D")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let loc_ref = read_entity_ref(attrs, 1, entity_id, "location")?;
        let axis_ref = read_optional_entity_ref(attrs, 2, entity_id, "axis")?;
        let ref_dir_ref = read_optional_entity_ref(attrs, 3, entity_id, "ref_direction")?;

        let location = self.resolve_point(entity_id, loc_ref, "location")?;
        let axis = axis_ref
            .map(|r| self.resolve_direction(entity_id, r, "axis"))
            .transpose()?;
        let ref_direction = ref_dir_ref
            .map(|r| self.resolve_direction(entity_id, r, "ref_direction"))
            .transpose()?;

        let placement = Axis2Placement3d {
            location,
            axis,
            ref_direction,
        };
        let id = self.geometry.placements.push(placement);
        self.placement_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_axis1_placement(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "AXIS1_PLACEMENT")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let loc_ref = read_entity_ref(attrs, 1, entity_id, "location")?;
        let axis_ref = read_entity_ref(attrs, 2, entity_id, "axis")?;

        let location = self.resolve_point(entity_id, loc_ref, "location")?;
        let axis = self.resolve_direction(entity_id, axis_ref, "axis")?;

        let placement = Axis1Placement { location, axis };
        let id = self.geometry.placements_1d.push(placement);
        self.axis1_map.insert(entity_id, id);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 4-1: Simple leaf curves and surfaces
    // ------------------------------------------------------------------

    pub(super) fn convert_line(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "LINE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pnt_ref = read_entity_ref(attrs, 1, entity_id, "pnt")?;
        let dir_ref = read_entity_ref(attrs, 2, entity_id, "dir")?;

        let point = self.resolve_point(entity_id, pnt_ref, "pnt")?;
        let (direction, magnitude) = self.resolve_vector(entity_id, dir_ref, "dir")?;

        let line = Line3 {
            point,
            direction,
            magnitude,
        };
        let id = self.geometry.curves.push(Curve::Line(line));
        self.curve_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_plane(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PLANE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;

        let position = self.resolve_placement(entity_id, pos_ref, "position")?;

        let plane = Plane3 { position };
        let id = self.geometry.surfaces.push(Surface::Plane(plane));
        self.surface_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_cylindrical_surface(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "CYLINDRICAL_SURFACE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let radius = read_real(attrs, 2, entity_id, "radius")?;

        let position = self.resolve_placement(entity_id, pos_ref, "position")?;

        let surface = CylindricalSurface { position, radius };
        let id = self.geometry.surfaces.push(Surface::Cylinder(surface));
        self.surface_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_spherical_surface(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SPHERICAL_SURFACE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let radius = read_real(attrs, 2, entity_id, "radius")?;

        let position = self.resolve_placement(entity_id, pos_ref, "position")?;

        let surface = SphericalSurface { position, radius };
        let id = self.geometry.surfaces.push(Surface::Sphere(surface));
        self.surface_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_conical_surface(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "CONICAL_SURFACE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let radius = read_real(attrs, 2, entity_id, "radius")?;
        let semi_angle = read_real(attrs, 3, entity_id, "semi_angle")?;

        let position = self.resolve_placement(entity_id, pos_ref, "position")?;

        let surface = ConicalSurface {
            position,
            radius,
            semi_angle,
        };
        let id = self.geometry.surfaces.push(Surface::Cone(surface));
        self.surface_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_toroidal_surface(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "TOROIDAL_SURFACE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let major_radius = read_real(attrs, 2, entity_id, "major_radius")?;
        let minor_radius = read_real(attrs, 3, entity_id, "minor_radius")?;

        let position = self.resolve_placement(entity_id, pos_ref, "position")?;

        let surface = ToroidalSurface {
            position,
            major_radius,
            minor_radius,
        };
        let id = self.geometry.surfaces.push(Surface::Torus(surface));
        self.surface_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_circle(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "CIRCLE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let radius = read_real(attrs, 2, entity_id, "radius")?;

        let position = self.resolve_placement(entity_id, pos_ref, "position")?;

        let circle = Circle3 { position, radius };
        let id = self.geometry.curves.push(Curve::Circle(circle));
        self.curve_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_ellipse(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "ELLIPSE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let semi_axis_1 = read_real(attrs, 2, entity_id, "semi_axis_1")?;
        let semi_axis_2 = read_real(attrs, 3, entity_id, "semi_axis_2")?;

        let position = self.resolve_placement(entity_id, pos_ref, "position")?;

        let ellipse = Ellipse3 {
            position,
            semi_axis_1,
            semi_axis_2,
        };
        let id = self.geometry.curves.push(Curve::Ellipse(ellipse));
        self.curve_map.insert(entity_id, id);
        Ok(())
    }

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

    pub(super) fn convert_surface_curve(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        self.convert_surface_or_seam_curve(entity_id, attrs, "SURFACE_CURVE")
    }

    pub(super) fn convert_seam_curve(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        self.convert_surface_or_seam_curve(entity_id, attrs, "SEAM_CURVE")
    }

    fn convert_surface_or_seam_curve(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
        tag: &'static str,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, tag)?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let curve_3d_ref = read_entity_ref(attrs, 1, entity_id, "curve_3d")?;
        // attrs[2] = associated_geometry (pcurves) — resolved in a separate
        // post-pass that has the EntityGraph in scope (see
        // `collect_surface_curve_pcurves` in `passes.rs`).
        // attrs[3] = master_representation enum — intentionally ignored; the
        // OCCT convention of `.PCURVE_S1.` is reproduced by the writer.

        let curve_3d = self.resolve_curve(entity_id, curve_3d_ref, "curve_3d")?;
        self.curve_map.insert(entity_id, curve_3d);
        Ok(())
    }

    /// Read a SURFACE_CURVE / SEAM_CURVE entity's `associated_geometry` list
    /// and convert each PCURVE reference into a [`Pcurve`]. Called by the
    /// post-pass in `passes.rs` after 2D geometry (Pass 4a) and Pass 4-3
    /// have populated the surface / curve_2d maps.
    pub(super) fn collect_surface_curve_pcurves(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &crate::parser::entity::EntityGraph,
    ) {
        let Ok(pcurve_refs) = read_entity_ref_list(attrs, 2, entity_id, "associated_geometry")
        else {
            return;
        };
        let mut pcurves = Vec::with_capacity(pcurve_refs.len());
        for &pcurve_ref in &pcurve_refs {
            if let Some(pc) = self.resolve_pcurve(pcurve_ref, graph) {
                pcurves.push(pc);
            }
        }
        if !pcurves.is_empty() {
            self.surface_curve_pcurves_map.insert(entity_id, pcurves);
        }
    }

    pub(super) fn convert_bspline_curve_with_knots(
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
        // [5] self_intersect — informational, skipped
        let knot_multiplicities = read_integer_list(attrs, 6, entity_id, "knot_multiplicities")?;
        let knots = read_real_list(attrs, 7, entity_id, "knots")?;
        // [8] knot_spec — informational, skipped

        let degree = u32::try_from(degree_i).map_err(|_| ConvertError::AttributeType {
            entity_id,
            field_name: "degree",
            expected: "non-negative Integer",
            actual: AttributeKindTag::Integer,
        })?;

        let mut control_points = Vec::with_capacity(cp_refs.len());
        for &r in &cp_refs {
            let pt = self.resolve_point(entity_id, r, "control_points_list")?;
            control_points.push(pt);
        }

        let curve = NurbsCurve {
            degree,
            control_points,
            weights: None,
            knot_multiplicities,
            knots,
            closed,
            form,
        };
        let id = self.geometry.curves.push(Curve::Nurbs(curve));
        self.curve_map.insert(entity_id, id);
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn convert_bspline_surface_with_knots(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 13, entity_id, "B_SPLINE_SURFACE_WITH_KNOTS")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let u_degree_i = read_integer(attrs, 1, entity_id, "u_degree")?;
        let v_degree_i = read_integer(attrs, 2, entity_id, "v_degree")?;
        let cp_grid = read_entity_ref_grid(attrs, 3, entity_id, "control_points_list")?;
        let form = SurfaceForm::from_step_enum(read_enum(attrs, 4, entity_id, "surface_form")?);
        // [7] self_intersect — informational, skipped
        let u_closed = read_bool(attrs, 5, entity_id, "u_closed")?;
        let v_closed = read_bool(attrs, 6, entity_id, "v_closed")?;
        let u_knot_multiplicities = read_integer_list(attrs, 8, entity_id, "u_multiplicities")?;
        let v_knot_multiplicities = read_integer_list(attrs, 9, entity_id, "v_multiplicities")?;
        let u_knots = read_real_list(attrs, 10, entity_id, "u_knots")?;
        let v_knots = read_real_list(attrs, 11, entity_id, "v_knots")?;
        // [12] knot_spec — informational, skipped

        let u_degree = u32::try_from(u_degree_i).map_err(|_| ConvertError::AttributeType {
            entity_id,
            field_name: "u_degree",
            expected: "non-negative Integer",
            actual: AttributeKindTag::Integer,
        })?;
        let v_degree = u32::try_from(v_degree_i).map_err(|_| ConvertError::AttributeType {
            entity_id,
            field_name: "v_degree",
            expected: "non-negative Integer",
            actual: AttributeKindTag::Integer,
        })?;

        let mut control_points = Vec::with_capacity(cp_grid.len());
        for row in &cp_grid {
            let mut pt_row = Vec::with_capacity(row.len());
            for &r in row {
                let pt = self.resolve_point(entity_id, r, "control_points_list")?;
                pt_row.push(pt);
            }
            control_points.push(pt_row);
        }

        let surface = NurbsSurface {
            u_degree,
            v_degree,
            control_points,
            weights: None,
            u_knot_multiplicities,
            v_knot_multiplicities,
            u_knots,
            v_knots,
            u_closed,
            v_closed,
            form,
        };
        let id = self.geometry.surfaces.push(Surface::Nurbs(surface));
        self.surface_map.insert(entity_id, id);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 4-2: Complex rational B-spline curves
    // ------------------------------------------------------------------

    pub(super) fn convert_rational_bspline_curve(
        &mut self,
        entity_id: u64,
        parts: &[RawEntityPart],
    ) -> Result<(), ConvertError> {
        let repr_attrs = require_part_attrs(parts, "REPRESENTATION_ITEM", entity_id)?;
        let _name = read_string(repr_attrs, 0, entity_id, "name")?;

        let bsc_attrs = require_part_attrs(parts, "B_SPLINE_CURVE", entity_id)?;
        let degree_i = read_integer(bsc_attrs, 0, entity_id, "degree")?;
        let cp_refs = read_entity_ref_list(bsc_attrs, 1, entity_id, "control_points_list")?;
        let form = CurveForm::from_step_enum(read_enum(bsc_attrs, 2, entity_id, "curve_form")?);
        let closed = read_bool(bsc_attrs, 3, entity_id, "closed_curve")?;

        let bswk_attrs = require_part_attrs(parts, "B_SPLINE_CURVE_WITH_KNOTS", entity_id)?;
        let knot_multiplicities =
            read_integer_list(bswk_attrs, 0, entity_id, "knot_multiplicities")?;
        let knots = read_real_list(bswk_attrs, 1, entity_id, "knots")?;

        let rat_attrs = require_part_attrs(parts, "RATIONAL_B_SPLINE_CURVE", entity_id)?;
        let weights = read_real_list(rat_attrs, 0, entity_id, "weights_data")?;

        if weights.len() != cp_refs.len() {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "weights_data",
                expected: cp_refs.len(),
                actual: weights.len(),
            });
        }

        let degree = u32::try_from(degree_i).map_err(|_| ConvertError::AttributeType {
            entity_id,
            field_name: "degree",
            expected: "non-negative Integer",
            actual: AttributeKindTag::Integer,
        })?;

        let mut control_points = Vec::with_capacity(cp_refs.len());
        for &r in &cp_refs {
            let pt = self.resolve_point(entity_id, r, "control_points_list")?;
            control_points.push(pt);
        }

        let curve = NurbsCurve {
            degree,
            control_points,
            weights: Some(weights),
            knot_multiplicities,
            knots,
            closed,
            form,
        };
        let id = self.geometry.curves.push(Curve::Nurbs(curve));
        self.curve_map.insert(entity_id, id);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 4-3: Surfaces that reference curves
    // ------------------------------------------------------------------

    pub(super) fn convert_surface_of_revolution(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SURFACE_OF_REVOLUTION")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let curve_ref = read_entity_ref(attrs, 1, entity_id, "swept_curve")?;
        let axis_ref = read_entity_ref(attrs, 2, entity_id, "axis_position")?;

        let swept_curve = self.resolve_curve(entity_id, curve_ref, "swept_curve")?;
        let axis_placement = self.resolve_axis1(entity_id, axis_ref, "axis_position")?;

        let surface = SurfaceOfRevolution {
            swept_curve,
            axis_placement,
        };
        let id = self.geometry.surfaces.push(Surface::Revolution(surface));
        self.surface_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_surface_of_linear_extrusion(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SURFACE_OF_LINEAR_EXTRUSION")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let curve_ref = read_entity_ref(attrs, 1, entity_id, "swept_curve")?;
        let vector_ref = read_entity_ref(attrs, 2, entity_id, "extrusion_axis")?;

        let swept_curve = self.resolve_curve(entity_id, curve_ref, "swept_curve")?;
        let (extrusion_direction, depth) =
            self.resolve_vector(entity_id, vector_ref, "extrusion_axis")?;

        let surface = SurfaceOfLinearExtrusion {
            swept_curve,
            extrusion_direction,
            depth,
        };
        let id = self.geometry.surfaces.push(Surface::Extrusion(surface));
        self.surface_map.insert(entity_id, id);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 4-2: Complex rational B-spline surface
    // ------------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    pub(super) fn convert_rational_bspline_surface(
        &mut self,
        entity_id: u64,
        parts: &[RawEntityPart],
    ) -> Result<(), ConvertError> {
        let repr_attrs = require_part_attrs(parts, "REPRESENTATION_ITEM", entity_id)?;
        let _name = read_string(repr_attrs, 0, entity_id, "name")?;

        let bss_attrs = require_part_attrs(parts, "B_SPLINE_SURFACE", entity_id)?;
        let u_degree_i = read_integer(bss_attrs, 0, entity_id, "u_degree")?;
        let v_degree_i = read_integer(bss_attrs, 1, entity_id, "v_degree")?;
        let cp_grid = read_entity_ref_grid(bss_attrs, 2, entity_id, "control_points_list")?;
        let form = SurfaceForm::from_step_enum(read_enum(bss_attrs, 3, entity_id, "surface_form")?);
        let u_closed = read_bool(bss_attrs, 4, entity_id, "u_closed")?;
        let v_closed = read_bool(bss_attrs, 5, entity_id, "v_closed")?;

        let bswk_attrs = require_part_attrs(parts, "B_SPLINE_SURFACE_WITH_KNOTS", entity_id)?;
        let u_knot_multiplicities =
            read_integer_list(bswk_attrs, 0, entity_id, "u_multiplicities")?;
        let v_knot_multiplicities =
            read_integer_list(bswk_attrs, 1, entity_id, "v_multiplicities")?;
        let u_knots = read_real_list(bswk_attrs, 2, entity_id, "u_knots")?;
        let v_knots = read_real_list(bswk_attrs, 3, entity_id, "v_knots")?;

        let rat_attrs = require_part_attrs(parts, "RATIONAL_B_SPLINE_SURFACE", entity_id)?;
        let weights = read_real_grid(rat_attrs, 0, entity_id, "weights_data")?;

        // Validate weights 2D grid matches control points 2D grid.
        if weights.len() != cp_grid.len() {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "weights_data",
                expected: cp_grid.len(),
                actual: weights.len(),
            });
        }
        for (w_row, cp_row) in weights.iter().zip(cp_grid.iter()) {
            if w_row.len() != cp_row.len() {
                return Err(ConvertError::DimensionMismatch {
                    entity_id,
                    field_name: "weights_data",
                    expected: cp_row.len(),
                    actual: w_row.len(),
                });
            }
        }

        let u_degree = u32::try_from(u_degree_i).map_err(|_| ConvertError::AttributeType {
            entity_id,
            field_name: "u_degree",
            expected: "non-negative Integer",
            actual: AttributeKindTag::Integer,
        })?;
        let v_degree = u32::try_from(v_degree_i).map_err(|_| ConvertError::AttributeType {
            entity_id,
            field_name: "v_degree",
            expected: "non-negative Integer",
            actual: AttributeKindTag::Integer,
        })?;

        let mut control_points = Vec::with_capacity(cp_grid.len());
        for row in &cp_grid {
            let mut pt_row = Vec::with_capacity(row.len());
            for &r in row {
                let pt = self.resolve_point(entity_id, r, "control_points_list")?;
                pt_row.push(pt);
            }
            control_points.push(pt_row);
        }

        let surface = NurbsSurface {
            u_degree,
            v_degree,
            control_points,
            weights: Some(weights),
            u_knot_multiplicities,
            v_knot_multiplicities,
            u_knots,
            v_knots,
            u_closed,
            v_closed,
            form,
        };
        let id = self.geometry.surfaces.push(Surface::Nurbs(surface));
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

    pub(super) fn convert_cartesian_point_2d(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "CARTESIAN_POINT")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let coords = read_real_list(attrs, 1, entity_id, "coordinates")?;
        if coords.len() != 2 {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "coordinates",
                expected: 2,
                actual: coords.len(),
            });
        }
        let id = self.geometry.points_2d.push(Point2 {
            x: coords[0],
            y: coords[1],
        });
        self.point_2d_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_direction_2d(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "DIRECTION")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let coords = read_real_list(attrs, 1, entity_id, "direction_ratios")?;
        if coords.len() != 2 {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "direction_ratios",
                expected: 2,
                actual: coords.len(),
            });
        }
        let id = self.geometry.directions_2d.push(Direction2 {
            x: coords[0],
            y: coords[1],
        });
        self.direction_2d_map.insert(entity_id, id);
        Ok(())
    }

    pub(super) fn convert_vector_2d(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "VECTOR")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let dir_ref = read_entity_ref(attrs, 1, entity_id, "orientation")?;
        let magnitude = read_real(attrs, 2, entity_id, "magnitude")?;
        let dir =
            *self
                .direction_2d_map
                .get(&dir_ref)
                .ok_or_else(|| ConvertError::MissingReference {
                    from: entity_id,
                    to: dir_ref,
                    field_name: "orientation",
                })?;
        self.vector_2d_map.insert(entity_id, (dir, magnitude));
        Ok(())
    }

    pub(super) fn convert_axis2_placement_2d(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "AXIS2_PLACEMENT_2D")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let loc_ref = read_entity_ref(attrs, 1, entity_id, "location")?;
        let ref_dir_ref = read_optional_entity_ref(attrs, 2, entity_id, "ref_direction")?;

        let location =
            *self
                .point_2d_map
                .get(&loc_ref)
                .ok_or_else(|| ConvertError::MissingReference {
                    from: entity_id,
                    to: loc_ref,
                    field_name: "location",
                })?;
        let ref_direction = match ref_dir_ref {
            Some(r) => Some(*self.direction_2d_map.get(&r).ok_or_else(|| {
                ConvertError::MissingReference {
                    from: entity_id,
                    to: r,
                    field_name: "ref_direction",
                }
            })?),
            None => None,
        };
        let placement = Axis2Placement2d {
            location,
            ref_direction,
        };
        let id = self.geometry.placements_2d.push(placement);
        self.placement_2d_map.insert(entity_id, id);
        Ok(())
    }

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
    pub(super) fn resolve_pcurve(
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
