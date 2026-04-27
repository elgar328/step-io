//! Geometry pool emission: points, directions, placements, curves, surfaces.

#![allow(clippy::doc_markdown)]

use super::WriteBuffer;
use crate::ir::{
    Circle2, Circle3, CompositeCurve, CompositeSegment, ConicalSurface, Curve, Curve2d, Curve2dId,
    CurveId, CylindricalSurface, Direction2, Direction2dId, Direction3, DirectionId, Ellipse2,
    Ellipse3, Line2, Line3, NurbsCurve, NurbsCurve2d, NurbsSurface, Pcurve, Placement1dId,
    Placement2dId, Placement3dId, Plane3, Point2, Point2dId, Point3, PointId, SphericalSurface,
    StepModel, Surface, SurfaceId, SurfaceOfLinearExtrusion, SurfaceOfOffset, SurfaceOfRevolution,
    ToroidalSurface, TransitionCode, TrimMaster, TrimmedCurve,
};
use crate::parser::entity::Attribute;
use crate::writer::WriteError;
use crate::writer::entity::{WriterBody, WriterEntity};

impl WriteBuffer<'_> {
    pub(crate) fn emit_point(&mut self, id: PointId) -> Result<u64, WriteError> {
        if let Some(&n) = self.point_ids.get(&id) {
            return Ok(n);
        }
        let p = point_at(self.model, id)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "CARTESIAN_POINT".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(vec![
                        Attribute::Real(p.x),
                        Attribute::Real(p.y),
                        Attribute::Real(p.z),
                    ]),
                ],
            },
        });
        self.point_ids.insert(id, n);
        Ok(n)
    }

    pub(crate) fn emit_direction(&mut self, id: DirectionId) -> Result<u64, WriteError> {
        // Step 1 pilot: dispatch through the EntityHandler trait. Body lives in
        // `src/entities/geometry/direction.rs`. Plan 2 removes this wrapper.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::direction::DirectionHandler::write(self, id)
    }

    pub(crate) fn emit_axis2_placement_3d(&mut self, id: Placement3dId) -> Result<u64, WriteError> {
        if let Some(&n) = self.placement_ids.get(&id) {
            return Ok(n);
        }
        let placement = self.model.geometry.placements[id];
        let loc = self.emit_point(placement.location)?;
        let axis_attr = match placement.axis {
            Some(dir) => Attribute::EntityRef(self.emit_direction(dir)?),
            None => Attribute::Unset,
        };
        let ref_attr = match placement.ref_direction {
            Some(dir) => Attribute::EntityRef(self.emit_direction(dir)?),
            None => Attribute::Unset,
        };
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "AXIS2_PLACEMENT_3D".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(loc),
                    axis_attr,
                    ref_attr,
                ],
            },
        });
        self.placement_ids.insert(id, n);
        Ok(n)
    }

    fn emit_vector(&mut self, direction: DirectionId, magnitude: f64) -> Result<u64, WriteError> {
        // Step 1 pilot: dispatch through the EntityHandler trait. Body lives in
        // `src/entities/geometry/vector.rs`. Plan 2 will replace this wrapper.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::vector::VectorHandler::write(self, (direction, magnitude))
    }

    pub(crate) fn emit_curve(&mut self, id: CurveId) -> Result<u64, WriteError> {
        if let Some(&n) = self.curve_ids.get(&id) {
            return Ok(n);
        }
        let curve = curve_at(self.model, id)?.clone();
        let n = match curve {
            Curve::Line(line) => self.emit_line(line)?,
            Curve::Circle(circle) => self.emit_circle(circle)?,
            Curve::Ellipse(ellipse) => self.emit_ellipse(ellipse)?,
            Curve::Nurbs(nurbs) => self.emit_nurbs_curve(nurbs)?,
            Curve::Trimmed(trimmed) => self.emit_trimmed_curve(trimmed)?,
            Curve::Composite(composite) => self.emit_composite_curve(&composite)?,
        };
        self.curve_ids.insert(id, n);
        Ok(n)
    }

    fn emit_trimmed_curve(&mut self, trimmed: TrimmedCurve) -> Result<u64, WriteError> {
        let basis = self.emit_curve(trimmed.basis)?;
        let trim_1 = self.build_trim_select(trimmed.trim_1_point, trimmed.trim_1_param)?;
        let trim_2 = self.build_trim_select(trimmed.trim_2_point, trimmed.trim_2_param)?;
        let master = match trimmed.master {
            TrimMaster::Cartesian => "CARTESIAN",
            TrimMaster::Parameter => "PARAMETER",
            TrimMaster::Unspecified => "UNSPECIFIED",
        };
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "TRIMMED_CURVE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(basis),
                    Attribute::List(trim_1),
                    Attribute::List(trim_2),
                    Attribute::Enum(if trimmed.sense_agreement { "T" } else { "F" }.into()),
                    Attribute::Enum(master.into()),
                ],
            },
        });
        Ok(n)
    }

    /// Build the SET-of-trim_select attribute list for a `TRIMMED_CURVE` slot.
    /// Either, both, or neither of the cartesian point and parameter value may
    /// be present; the writer emits whatever the IR carries, faithfully.
    fn build_trim_select(
        &mut self,
        point: Option<PointId>,
        param: Option<f64>,
    ) -> Result<Vec<Attribute>, WriteError> {
        let mut elements = Vec::new();
        if let Some(p) = point {
            elements.push(Attribute::EntityRef(self.emit_point(p)?));
        }
        if let Some(v) = param {
            elements.push(Attribute::Typed {
                type_name: "PARAMETER_VALUE".into(),
                value: Box::new(Attribute::Real(v)),
            });
        }
        Ok(elements)
    }

    fn emit_composite_curve(&mut self, composite: &CompositeCurve) -> Result<u64, WriteError> {
        let mut segment_refs = Vec::with_capacity(composite.segments.len());
        for seg in &composite.segments {
            segment_refs.push(Attribute::EntityRef(
                self.emit_composite_curve_segment(*seg)?,
            ));
        }
        let self_intersect = match composite.self_intersect {
            Some(true) => "T",
            Some(false) => "F",
            None => "U",
        };
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "COMPOSITE_CURVE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(segment_refs),
                    Attribute::Enum(self_intersect.into()),
                ],
            },
        });
        Ok(n)
    }

    fn emit_composite_curve_segment(
        &mut self,
        segment: CompositeSegment,
    ) -> Result<u64, WriteError> {
        let parent = self.emit_curve(segment.parent_curve)?;
        let transition = match segment.transition {
            TransitionCode::Continuous => "CONTINUOUS",
            TransitionCode::Discontinuous => "DISCONTINUOUS",
            TransitionCode::ContSameGradient => "CONT_SAME_GRADIENT",
            TransitionCode::ContSameGradientSameCurvature => "CONT_SAME_GRADIENT_SAME_CURVATURE",
            TransitionCode::Unspecified => "UNSPECIFIED",
        };
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "COMPOSITE_CURVE_SEGMENT".into(),
                attrs: vec![
                    Attribute::Enum(transition.into()),
                    Attribute::Enum(if segment.same_sense { "T" } else { "F" }.into()),
                    Attribute::EntityRef(parent),
                ],
            },
        });
        Ok(n)
    }

    fn emit_circle(&mut self, circle: Circle3) -> Result<u64, WriteError> {
        let pos = self.emit_axis2_placement_3d(circle.position)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "CIRCLE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(circle.radius),
                ],
            },
        });
        Ok(n)
    }

    fn emit_ellipse(&mut self, ellipse: Ellipse3) -> Result<u64, WriteError> {
        let pos = self.emit_axis2_placement_3d(ellipse.position)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "ELLIPSE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(ellipse.semi_axis_1),
                    Attribute::Real(ellipse.semi_axis_2),
                ],
            },
        });
        Ok(n)
    }

    fn emit_line(&mut self, line: Line3) -> Result<u64, WriteError> {
        let pnt = self.emit_point(line.point)?;
        let vec = self.emit_vector(line.direction, line.magnitude)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "LINE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pnt),
                    Attribute::EntityRef(vec),
                ],
            },
        });
        Ok(n)
    }

    pub(crate) fn emit_surface(&mut self, id: SurfaceId) -> Result<u64, WriteError> {
        if let Some(&n) = self.surface_ids.get(&id) {
            return Ok(n);
        }
        let surface = surface_at(self.model, id)?.clone();
        let n = match surface {
            Surface::Plane(p) => self.emit_plane(p)?,
            Surface::Cylinder(c) => self.emit_cylinder(c)?,
            Surface::Sphere(s) => self.emit_sphere(s)?,
            Surface::Cone(c) => self.emit_cone(c)?,
            Surface::Torus(t) => self.emit_torus(t)?,
            Surface::Revolution(r) => self.emit_surface_of_revolution(r)?,
            Surface::Extrusion(e) => self.emit_surface_of_linear_extrusion(e)?,
            Surface::Offset(o) => self.emit_offset_surface(o)?,
            Surface::Nurbs(nurbs) => self.emit_nurbs_surface(nurbs)?,
        };
        self.surface_ids.insert(id, n);
        Ok(n)
    }

    fn emit_plane(&mut self, p: Plane3) -> Result<u64, WriteError> {
        let pos = self.emit_axis2_placement_3d(p.position)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "PLANE".into(),
                attrs: vec![Attribute::String(String::new()), Attribute::EntityRef(pos)],
            },
        });
        Ok(n)
    }

    fn emit_cylinder(&mut self, c: CylindricalSurface) -> Result<u64, WriteError> {
        let pos = self.emit_axis2_placement_3d(c.position)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "CYLINDRICAL_SURFACE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(c.radius),
                ],
            },
        });
        Ok(n)
    }

    fn emit_sphere(&mut self, s: SphericalSurface) -> Result<u64, WriteError> {
        let pos = self.emit_axis2_placement_3d(s.position)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "SPHERICAL_SURFACE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(s.radius),
                ],
            },
        });
        Ok(n)
    }

    fn emit_cone(&mut self, c: ConicalSurface) -> Result<u64, WriteError> {
        let pos = self.emit_axis2_placement_3d(c.position)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "CONICAL_SURFACE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(c.radius),
                    Attribute::Real(c.semi_angle),
                ],
            },
        });
        Ok(n)
    }

    fn emit_torus(&mut self, t: ToroidalSurface) -> Result<u64, WriteError> {
        let pos = self.emit_axis2_placement_3d(t.position)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "TOROIDAL_SURFACE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(t.major_radius),
                    Attribute::Real(t.minor_radius),
                ],
            },
        });
        Ok(n)
    }

    fn emit_nurbs_curve(&mut self, nurbs: NurbsCurve) -> Result<u64, WriteError> {
        if nurbs.weights.is_some() {
            // Plan 3 stage 3: complex RATIONAL_B_SPLINE_CURVE flows through
            // the EntityHandler registry. Body lives in
            // `src/entities/geometry/rational_bspline_curve.rs`.
            use crate::entities::ComplexEntityHandler;
            return crate::entities::geometry::rational_bspline_curve::RationalBsplineCurveHandler::write(self, nurbs);
        }

        // Non-rational simple B_SPLINE_CURVE_WITH_KNOTS — 9 attrs.
        let mut cp_refs = Vec::with_capacity(nurbs.control_points.len());
        for &pid in &nurbs.control_points {
            cp_refs.push(self.emit_point(pid)?);
        }
        #[allow(clippy::cast_possible_wrap)]
        let degree_attr = Attribute::Integer(i64::from(nurbs.degree));
        let cps_attr = Attribute::List(cp_refs.into_iter().map(Attribute::EntityRef).collect());
        let mults_attr = Attribute::List(
            nurbs
                .knot_multiplicities
                .iter()
                .copied()
                .map(Attribute::Integer)
                .collect(),
        );
        let knots_attr =
            Attribute::List(nurbs.knots.iter().copied().map(Attribute::Real).collect());
        let closed_attr = Attribute::Enum(if nurbs.closed { "T".into() } else { "F".into() });
        let form = nurbs.form;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "B_SPLINE_CURVE_WITH_KNOTS".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    degree_attr,
                    cps_attr,
                    Attribute::Enum(form.as_step_enum().into()),
                    closed_attr,
                    Attribute::Enum("F".into()),
                    mults_attr,
                    knots_attr,
                    Attribute::Enum("UNSPECIFIED".into()),
                ],
            },
        });
        Ok(n)
    }

    #[allow(clippy::too_many_lines)]
    fn emit_nurbs_surface(&mut self, nurbs: NurbsSurface) -> Result<u64, WriteError> {
        // Build a 2D entity-ref grid for control points.
        let mut cp_rows: Vec<Attribute> = Vec::with_capacity(nurbs.control_points.len());
        for row in &nurbs.control_points {
            let mut refs = Vec::with_capacity(row.len());
            for &pid in row {
                refs.push(Attribute::EntityRef(self.emit_point(pid)?));
            }
            cp_rows.push(Attribute::List(refs));
        }
        let cps_attr = Attribute::List(cp_rows);
        #[allow(clippy::cast_possible_wrap)]
        let u_deg = Attribute::Integer(i64::from(nurbs.u_degree));
        #[allow(clippy::cast_possible_wrap)]
        let v_deg = Attribute::Integer(i64::from(nurbs.v_degree));
        let u_closed = Attribute::Enum(if nurbs.u_closed {
            "T".into()
        } else {
            "F".into()
        });
        let v_closed = Attribute::Enum(if nurbs.v_closed {
            "T".into()
        } else {
            "F".into()
        });
        let u_mults_attr = Attribute::List(
            nurbs
                .u_knot_multiplicities
                .iter()
                .copied()
                .map(Attribute::Integer)
                .collect(),
        );
        let v_mults_attr = Attribute::List(
            nurbs
                .v_knot_multiplicities
                .iter()
                .copied()
                .map(Attribute::Integer)
                .collect(),
        );
        let u_knots_attr =
            Attribute::List(nurbs.u_knots.iter().copied().map(Attribute::Real).collect());
        let v_knots_attr =
            Attribute::List(nurbs.v_knots.iter().copied().map(Attribute::Real).collect());
        let form = nurbs.form;

        let n = self.fresh();
        match nurbs.weights {
            None => {
                // Simple B_SPLINE_SURFACE_WITH_KNOTS — 13 attrs.
                self.entities.push(WriterEntity {
                    id: n,
                    body: WriterBody::Simple {
                        name: "B_SPLINE_SURFACE_WITH_KNOTS".into(),
                        attrs: vec![
                            Attribute::String(String::new()),
                            u_deg,
                            v_deg,
                            cps_attr,
                            Attribute::Enum(form.as_step_enum().into()),
                            u_closed,
                            v_closed,
                            Attribute::Enum("F".into()),
                            u_mults_attr,
                            v_mults_attr,
                            u_knots_attr,
                            v_knots_attr,
                            Attribute::Enum("UNSPECIFIED".into()),
                        ],
                    },
                });
            }
            Some(weights) => {
                // Complex RATIONAL_B_SPLINE_SURFACE — 7 parts, OCCT convention.
                let mut w_rows: Vec<Attribute> = Vec::with_capacity(weights.len());
                for row in weights {
                    w_rows.push(Attribute::List(
                        row.into_iter().map(Attribute::Real).collect(),
                    ));
                }
                let weights_attr = Attribute::List(w_rows);
                self.entities.push(WriterEntity {
                    id: n,
                    body: WriterBody::Complex {
                        parts: vec![
                            ("BOUNDED_SURFACE".into(), vec![]),
                            (
                                "B_SPLINE_SURFACE".into(),
                                vec![
                                    u_deg,
                                    v_deg,
                                    cps_attr,
                                    Attribute::Enum(form.as_step_enum().into()),
                                    u_closed,
                                    v_closed,
                                    Attribute::Enum("F".into()),
                                ],
                            ),
                            (
                                "B_SPLINE_SURFACE_WITH_KNOTS".into(),
                                vec![
                                    u_mults_attr,
                                    v_mults_attr,
                                    u_knots_attr,
                                    v_knots_attr,
                                    Attribute::Enum("UNSPECIFIED".into()),
                                ],
                            ),
                            ("GEOMETRIC_REPRESENTATION_ITEM".into(), vec![]),
                            ("RATIONAL_B_SPLINE_SURFACE".into(), vec![weights_attr]),
                            (
                                "REPRESENTATION_ITEM".into(),
                                vec![Attribute::String(String::new())],
                            ),
                            ("SURFACE".into(), vec![]),
                        ],
                    },
                });
            }
        }
        Ok(n)
    }

    fn emit_surface_of_revolution(&mut self, r: SurfaceOfRevolution) -> Result<u64, WriteError> {
        let swept = self.emit_curve(r.swept_curve)?;
        let axis = self.emit_axis1_placement(r.axis_placement)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "SURFACE_OF_REVOLUTION".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(swept),
                    Attribute::EntityRef(axis),
                ],
            },
        });
        Ok(n)
    }

    fn emit_surface_of_linear_extrusion(
        &mut self,
        e: SurfaceOfLinearExtrusion,
    ) -> Result<u64, WriteError> {
        let swept = self.emit_curve(e.swept_curve)?;
        let vector = self.emit_vector(e.extrusion_direction, e.depth)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "SURFACE_OF_LINEAR_EXTRUSION".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(swept),
                    Attribute::EntityRef(vector),
                ],
            },
        });
        Ok(n)
    }

    fn emit_offset_surface(&mut self, o: SurfaceOfOffset) -> Result<u64, WriteError> {
        let basis = self.emit_surface(o.basis)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "OFFSET_SURFACE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(basis),
                    Attribute::Real(o.distance),
                    // self_intersect LOGICAL — .F. hardcoded (informational,
                    // not stored in IR; see ROADMAP "LOGICAL 보존").
                    Attribute::Enum("F".into()),
                ],
            },
        });
        Ok(n)
    }

    pub(crate) fn emit_axis1_placement(&mut self, id: Placement1dId) -> Result<u64, WriteError> {
        if let Some(&n) = self.placement_1d_ids.get(&id) {
            return Ok(n);
        }
        let placement = self.model.geometry.placements_1d[id];
        let loc = self.emit_point(placement.location)?;
        let dir = self.emit_direction(placement.axis)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "AXIS1_PLACEMENT".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(loc),
                    Attribute::EntityRef(dir),
                ],
            },
        });
        self.placement_1d_ids.insert(id, n);
        Ok(n)
    }
}

fn point_at(model: &StepModel, id: PointId) -> Result<Point3, WriteError> {
    model
        .geometry
        .points
        .iter()
        .nth(id.0 as usize)
        .copied()
        .ok_or_else(|| WriteError::DanglingId {
            detail: format!("PointId({})", id.0),
        })
}

pub(crate) fn direction_at(model: &StepModel, id: DirectionId) -> Result<Direction3, WriteError> {
    model
        .geometry
        .directions
        .iter()
        .nth(id.0 as usize)
        .copied()
        .ok_or_else(|| WriteError::DanglingId {
            detail: format!("DirectionId({})", id.0),
        })
}

fn curve_at(model: &StepModel, id: CurveId) -> Result<&Curve, WriteError> {
    model
        .geometry
        .curves
        .iter()
        .nth(id.0 as usize)
        .ok_or_else(|| WriteError::DanglingId {
            detail: format!("CurveId({})", id.0),
        })
}

fn surface_at(model: &StepModel, id: SurfaceId) -> Result<&Surface, WriteError> {
    model
        .geometry
        .surfaces
        .iter()
        .nth(id.0 as usize)
        .ok_or_else(|| WriteError::DanglingId {
            detail: format!("SurfaceId({})", id.0),
        })
}

// ---------------------------------------------------------------------------
// 2D geometry (PCURVE parametric space)
// ---------------------------------------------------------------------------

impl WriteBuffer<'_> {
    pub(crate) fn emit_point_2d(&mut self, id: Point2dId) -> Result<u64, WriteError> {
        if let Some(&n) = self.point_2d_ids.get(&id) {
            return Ok(n);
        }
        let p = point_2d_at(self.model, id)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "CARTESIAN_POINT".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(vec![Attribute::Real(p.x), Attribute::Real(p.y)]),
                ],
            },
        });
        self.point_2d_ids.insert(id, n);
        Ok(n)
    }

    pub(crate) fn emit_direction_2d(&mut self, id: Direction2dId) -> Result<u64, WriteError> {
        if let Some(&n) = self.direction_2d_ids.get(&id) {
            return Ok(n);
        }
        let d = direction_2d_at(self.model, id)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "DIRECTION".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(vec![Attribute::Real(d.x), Attribute::Real(d.y)]),
                ],
            },
        });
        self.direction_2d_ids.insert(id, n);
        Ok(n)
    }

    fn emit_vector_2d(
        &mut self,
        direction: Direction2dId,
        magnitude: f64,
    ) -> Result<u64, WriteError> {
        let dir_n = self.emit_direction_2d(direction)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "VECTOR".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(dir_n),
                    Attribute::Real(magnitude),
                ],
            },
        });
        Ok(n)
    }

    pub(crate) fn emit_axis2_placement_2d(&mut self, id: Placement2dId) -> Result<u64, WriteError> {
        if let Some(&n) = self.placement_2d_ids.get(&id) {
            return Ok(n);
        }
        let placement = self.model.geometry.placements_2d[id];
        let loc = self.emit_point_2d(placement.location)?;
        let ref_attr = match placement.ref_direction {
            Some(dir) => Attribute::EntityRef(self.emit_direction_2d(dir)?),
            None => Attribute::Unset,
        };
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "AXIS2_PLACEMENT_2D".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(loc),
                    ref_attr,
                ],
            },
        });
        self.placement_2d_ids.insert(id, n);
        Ok(n)
    }

    pub(crate) fn emit_curve_2d(&mut self, id: Curve2dId) -> Result<u64, WriteError> {
        if let Some(&n) = self.curve_2d_ids.get(&id) {
            return Ok(n);
        }
        let curve = curve_2d_at(self.model, id)?.clone();
        let n = match curve {
            Curve2d::Line(line) => self.emit_line_2d(line)?,
            Curve2d::Circle(c) => self.emit_circle_2d(c)?,
            Curve2d::Ellipse(e) => self.emit_ellipse_2d(e)?,
            Curve2d::Nurbs(nu) => self.emit_nurbs_curve_2d(&nu)?,
        };
        self.curve_2d_ids.insert(id, n);
        Ok(n)
    }

    fn emit_line_2d(&mut self, line: Line2) -> Result<u64, WriteError> {
        let pnt = self.emit_point_2d(line.point)?;
        let vec = self.emit_vector_2d(line.direction, line.magnitude)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "LINE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pnt),
                    Attribute::EntityRef(vec),
                ],
            },
        });
        Ok(n)
    }

    fn emit_circle_2d(&mut self, c: Circle2) -> Result<u64, WriteError> {
        let pos = self.emit_axis2_placement_2d(c.position)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "CIRCLE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(c.radius),
                ],
            },
        });
        Ok(n)
    }

    fn emit_ellipse_2d(&mut self, e: Ellipse2) -> Result<u64, WriteError> {
        let pos = self.emit_axis2_placement_2d(e.position)?;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "ELLIPSE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(e.semi_axis_1),
                    Attribute::Real(e.semi_axis_2),
                ],
            },
        });
        Ok(n)
    }

    fn emit_nurbs_curve_2d(&mut self, nurbs: &NurbsCurve2d) -> Result<u64, WriteError> {
        // 2D rational NURBS (complex RATIONAL_B_SPLINE_CURVE with 2D weights)
        // is absent from the current fixture set, so it's left unimplemented.
        // Should a fixture with `weights: Some(_)` appear, extend here with a
        // complex-entity emit analogous to the 3D rational path.
        if nurbs.weights.is_some() {
            return Err(WriteError::UnsupportedIrVariant {
                detail: "rational 2D NURBS curve (no fixture yet)".into(),
            });
        }
        let mut cp_refs = Vec::with_capacity(nurbs.control_points.len());
        for &pid in &nurbs.control_points {
            cp_refs.push(self.emit_point_2d(pid)?);
        }
        #[allow(clippy::cast_possible_wrap)]
        let degree_attr = Attribute::Integer(i64::from(nurbs.degree));
        let cps_attr = Attribute::List(cp_refs.into_iter().map(Attribute::EntityRef).collect());
        let mults_attr = Attribute::List(
            nurbs
                .knot_multiplicities
                .iter()
                .copied()
                .map(Attribute::Integer)
                .collect(),
        );
        let knots_attr =
            Attribute::List(nurbs.knots.iter().copied().map(Attribute::Real).collect());
        let closed_attr = Attribute::Enum(if nurbs.closed { "T".into() } else { "F".into() });
        let form = nurbs.form;
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "B_SPLINE_CURVE_WITH_KNOTS".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    degree_attr,
                    cps_attr,
                    Attribute::Enum(form.as_step_enum().into()),
                    closed_attr,
                    Attribute::Enum("F".into()),
                    mults_attr,
                    knots_attr,
                    Attribute::Enum("UNSPECIFIED".into()),
                ],
            },
        });
        Ok(n)
    }

    // -----------------------------------------------------------------
    // PCURVE / SURFACE_CURVE wrapper chain
    // -----------------------------------------------------------------

    /// Emit a fresh 2D representation context complex entity. Each
    /// `DEFINITIONAL_REPRESENTATION` in the fixture set owns its own context;
    /// we reproduce that convention (sharing a single context would still be
    /// parseable but would diverge from fixture output).
    fn emit_2d_representation_context(&mut self) -> u64 {
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex {
                parts: vec![
                    (
                        "GEOMETRIC_REPRESENTATION_CONTEXT".into(),
                        vec![Attribute::Integer(2)],
                    ),
                    ("PARAMETRIC_REPRESENTATION_CONTEXT".into(), vec![]),
                    (
                        "REPRESENTATION_CONTEXT".into(),
                        vec![
                            Attribute::String("2D SPACE".into()),
                            Attribute::String(String::new()),
                        ],
                    ),
                ],
            },
        });
        n
    }

    fn emit_definitional_representation(&mut self, curve_2d_ref: u64) -> u64 {
        let ctx = self.emit_2d_representation_context();
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "DEFINITIONAL_REPRESENTATION".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(vec![Attribute::EntityRef(curve_2d_ref)]),
                    Attribute::EntityRef(ctx),
                ],
            },
        });
        n
    }

    fn emit_pcurve(&mut self, pc: Pcurve) -> Result<u64, WriteError> {
        let surface_ref = self.emit_surface(pc.basis_surface)?;
        let curve_2d_ref = self.emit_curve_2d(pc.curve_2d)?;
        let def_repr = self.emit_definitional_representation(curve_2d_ref);
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "PCURVE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(surface_ref),
                    Attribute::EntityRef(def_repr),
                ],
            },
        });
        Ok(n)
    }

    /// Emit a SURFACE_CURVE or SEAM_CURVE wrapping `curve_3d_ref` plus each
    /// pcurve. The variant is chosen by the `pcurves` contents: if all
    /// pcurves share the same basis surface → SEAM_CURVE, otherwise
    /// SURFACE_CURVE. `master_representation` is always `.PCURVE_S1.` (OCCT
    /// convention — the reader doesn't consult this value).
    pub(crate) fn emit_surface_curve_wrapper(
        &mut self,
        curve_3d_ref: u64,
        pcurves: &[Pcurve],
    ) -> Result<u64, WriteError> {
        let mut pcurve_refs = Vec::with_capacity(pcurves.len());
        for pc in pcurves {
            pcurve_refs.push(self.emit_pcurve(*pc)?);
        }
        let is_seam = pcurves.len() >= 2
            && pcurves
                .iter()
                .all(|p| p.basis_surface == pcurves[0].basis_surface);
        let name = if is_seam {
            "SEAM_CURVE"
        } else {
            "SURFACE_CURVE"
        };
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: name.into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(curve_3d_ref),
                    Attribute::List(pcurve_refs.into_iter().map(Attribute::EntityRef).collect()),
                    Attribute::Enum("PCURVE_S1".into()),
                ],
            },
        });
        Ok(n)
    }
}

fn point_2d_at(model: &StepModel, id: Point2dId) -> Result<Point2, WriteError> {
    model
        .geometry
        .points_2d
        .iter()
        .nth(id.0 as usize)
        .copied()
        .ok_or_else(|| WriteError::DanglingId {
            detail: format!("Point2dId({})", id.0),
        })
}

fn direction_2d_at(model: &StepModel, id: Direction2dId) -> Result<Direction2, WriteError> {
    model
        .geometry
        .directions_2d
        .iter()
        .nth(id.0 as usize)
        .copied()
        .ok_or_else(|| WriteError::DanglingId {
            detail: format!("Direction2dId({})", id.0),
        })
}

fn curve_2d_at(model: &StepModel, id: Curve2dId) -> Result<&Curve2d, WriteError> {
    model
        .geometry
        .curves_2d
        .iter()
        .nth(id.0 as usize)
        .ok_or_else(|| WriteError::DanglingId {
            detail: format!("Curve2dId({})", id.0),
        })
}
