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
    Axis2Placement2d, Circle2, CompositeCurve, CompositeSegment, Curve, Curve2d, CurveForm,
    Direction2, Ellipse2, Line2, NurbsCurve2d, NurbsSurface, Pcurve, Point2, Surface, SurfaceForm,
    SurfaceOfLinearExtrusion, SurfaceOfOffset, SurfaceOfRevolution, TransitionCode, TrimMaster,
    TrimmedCurve,
};
use crate::parser::entity::{Attribute, RawEntity, RawEntityPart};

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

    // ------------------------------------------------------------------
    // Pass 4-1b: TRIMMED_CURVE
    // ------------------------------------------------------------------
    //
    // `TRIMMED_CURVE(name, basis, trim_1, trim_2, sense_agreement, master)`.
    // Each trim slot is a SET that may carry a CARTESIAN_POINT ref, a
    // PARAMETER_VALUE typed parameter, or both. We preserve whatever is
    // present so the writer can reproduce the original form.

    pub(super) fn convert_trimmed_curve(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 6, entity_id, "TRIMMED_CURVE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let basis_ref = read_entity_ref(attrs, 1, entity_id, "basis_curve")?;
        let basis = self.resolve_curve(entity_id, basis_ref, "basis_curve")?;
        let (trim_1_param, trim_1_point) = self.read_trim_select(attrs, 2, entity_id, "trim_1")?;
        let (trim_2_param, trim_2_point) = self.read_trim_select(attrs, 3, entity_id, "trim_2")?;
        let sense_agreement = read_bool(attrs, 4, entity_id, "sense_agreement")?;
        let master = read_enum(attrs, 5, entity_id, "master_representation")?;
        let master = match master {
            "CARTESIAN" => TrimMaster::Cartesian,
            "PARAMETER" => TrimMaster::Parameter,
            _ => TrimMaster::Unspecified,
        };

        let trimmed = TrimmedCurve {
            basis,
            trim_1_param,
            trim_1_point,
            trim_2_param,
            trim_2_point,
            sense_agreement,
            master,
        };
        let id = self.geometry.curves.push(Curve::Trimmed(trimmed));
        self.curve_map.insert(entity_id, id);
        Ok(())
    }

    /// Decode a `TRIMMED_CURVE` trim slot — a SET (`Attribute::List`) of
    /// `CARTESIAN_POINT` refs and `PARAMETER_VALUE(real)` typed parameters.
    /// Either, both, or neither may appear; missing values come back as
    /// `None`.
    fn read_trim_select(
        &self,
        attrs: &[Attribute],
        index: usize,
        entity_id: u64,
        field_name: &'static str,
    ) -> Result<(Option<f64>, Option<crate::ir::id::PointId>), ConvertError> {
        let Attribute::List(elements) = attrs.get(index).ok_or(ConvertError::AttributeCount {
            entity_id,
            entity_name: "TRIMMED_CURVE".into(),
            expected: index + 1,
            actual: attrs.len(),
        })?
        else {
            return Err(ConvertError::AttributeType {
                entity_id,
                field_name,
                expected: "List",
                actual: AttributeKindTag::from_attribute(attrs.get(index).unwrap()),
            });
        };
        let mut param = None;
        let mut point = None;
        for el in elements {
            match el {
                Attribute::EntityRef(r) => {
                    if let Some(&pid) = self.point_map.get(r) {
                        point = Some(pid);
                    }
                }
                Attribute::Typed { type_name, value } if type_name == "PARAMETER_VALUE" => {
                    if let Attribute::Real(v) = **value {
                        param = Some(v);
                    }
                }
                _ => {}
            }
        }
        Ok((param, point))
    }

    // ------------------------------------------------------------------
    // Pass 4-1c: COMPOSITE_CURVE_SEGMENT
    // ------------------------------------------------------------------
    //
    // `COMPOSITE_CURVE_SEGMENT(transition, same_sense, parent_curve)`. We
    // record the segment's metadata keyed by entity id; the actual
    // `CompositeSegment` struct is built when the owning COMPOSITE_CURVE
    // resolves, so the parent_curve ref is stored as a step id and only
    // converted to `CurveId` at that later moment.

    pub(super) fn convert_composite_curve_segment(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "COMPOSITE_CURVE_SEGMENT")?;
        let transition_str = read_enum(attrs, 0, entity_id, "transition")?;
        let same_sense = read_bool(attrs, 1, entity_id, "same_sense")?;
        let parent_step_id = read_entity_ref(attrs, 2, entity_id, "parent_curve")?;

        let transition = match transition_str {
            "CONTINUOUS" => TransitionCode::Continuous,
            "DISCONTINUOUS" => TransitionCode::Discontinuous,
            "CONT_SAME_GRADIENT" => TransitionCode::ContSameGradient,
            "CONT_SAME_GRADIENT_SAME_CURVATURE" => TransitionCode::ContSameGradientSameCurvature,
            _ => TransitionCode::Unspecified,
        };
        self.composite_segment_map
            .insert(entity_id, (transition, same_sense, parent_step_id));
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 4-1d: COMPOSITE_CURVE
    // ------------------------------------------------------------------
    //
    // `COMPOSITE_CURVE(name, segments, self_intersect)`. Resolves each
    // segment ref via `composite_segment_map` and converts the parent_curve
    // step id to `CurveId` via `resolve_curve` (which is fully populated by
    // this pass since TRIMMED_CURVE / SEGMENT have run already).

    pub(super) fn convert_composite_curve(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "COMPOSITE_CURVE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let segment_refs = read_entity_ref_list(attrs, 1, entity_id, "segments")?;
        let self_intersect_str = read_enum(attrs, 2, entity_id, "self_intersect")?;
        let self_intersect = match self_intersect_str {
            "T" => Some(true),
            "F" => Some(false),
            _ => None,
        };

        let mut segments = Vec::with_capacity(segment_refs.len());
        for seg_ref in segment_refs {
            let Some(&(transition, same_sense, parent_step_id)) =
                self.composite_segment_map.get(&seg_ref)
            else {
                return Err(ConvertError::MissingReference {
                    from: entity_id,
                    to: seg_ref,
                    field_name: "segments",
                });
            };
            let parent_curve = self.resolve_curve(entity_id, parent_step_id, "parent_curve")?;
            segments.push(CompositeSegment {
                transition,
                same_sense,
                parent_curve,
            });
        }

        let composite = CompositeCurve {
            segments,
            self_intersect,
        };
        let id = self.geometry.curves.push(Curve::Composite(composite));
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
