//! Geometry pool emission: points, directions, placements, curves, surfaces.

#![allow(clippy::doc_markdown)]

use super::WriteBuffer;
use crate::ir::{
    Circle2, Circle3, CompositeCurve, ConicalSurface, Curve, Curve2d, Curve2dId, CurveId,
    CylindricalSurface, Direction2dId, Direction3, DirectionId, Ellipse2, Ellipse3, Line2, Line3,
    NurbsCurve, NurbsCurve2d, NurbsSurface, Pcurve, Placement1dId, Placement2dId, Placement3dId,
    Plane3, Point2dId, PointId, SphericalSurface, StepModel, Surface, SurfaceId,
    SurfaceOfLinearExtrusion, SurfaceOfOffset, SurfaceOfRevolution, ToroidalSurface, TrimmedCurve,
};
use crate::parser::entity::Attribute;
use crate::writer::WriteError;
use crate::writer::entity::{WriterBody, WriterEntity};

impl WriteBuffer<'_> {
    pub(crate) fn emit_point(&mut self, id: PointId) -> Result<u64, WriteError> {
        // Plan 5 stage C1: dispatch through the EntityHandler trait. Body
        // lives in `src/entities/geometry/cartesian_point.rs`.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::cartesian_point::CartesianPointHandler::write(self, id)
    }

    pub(crate) fn emit_direction(&mut self, id: DirectionId) -> Result<u64, WriteError> {
        // Step 1 pilot: dispatch through the EntityHandler trait. Body lives in
        // `src/entities/geometry/direction.rs`. Plan 2 removes this wrapper.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::direction::DirectionHandler::write(self, id)
    }

    pub(crate) fn emit_axis2_placement_3d(&mut self, id: Placement3dId) -> Result<u64, WriteError> {
        // Plan 5 stage C1: dispatch through the EntityHandler trait.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler::write(self, id)
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
        // Plan 5 stage C5: dispatch through EntityHandler trait.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::trimmed_curve::TrimmedCurveHandler::write(self, trimmed)
    }

    fn emit_composite_curve(&mut self, composite: &CompositeCurve) -> Result<u64, WriteError> {
        // Plan 5 stage C5: dispatch through EntityHandler trait. Cloning
        // the IR struct is cheap (segments are a small Vec of Copy values).
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::composite_curve::CompositeCurveHandler::write(
            self,
            composite.clone(),
        )
    }

    fn emit_circle(&mut self, circle: Circle3) -> Result<u64, WriteError> {
        // Plan 5 stage C2: dispatch through EntityHandler trait.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::circle::CircleHandler::write(self, circle)
    }

    fn emit_ellipse(&mut self, ellipse: Ellipse3) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::ellipse::EllipseHandler::write(self, ellipse)
    }

    fn emit_line(&mut self, line: Line3) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::line::LineHandler::write(self, line)
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
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::plane::PlaneHandler::write(self, p)
    }

    fn emit_cylinder(&mut self, c: CylindricalSurface) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::cylindrical_surface::CylindricalSurfaceHandler::write(self, c)
    }

    fn emit_sphere(&mut self, s: SphericalSurface) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::spherical_surface::SphericalSurfaceHandler::write(self, s)
    }

    fn emit_cone(&mut self, c: ConicalSurface) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::conical_surface::ConicalSurfaceHandler::write(self, c)
    }

    fn emit_torus(&mut self, t: ToroidalSurface) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::toroidal_surface::ToroidalSurfaceHandler::write(self, t)
    }

    fn emit_nurbs_curve(&mut self, nurbs: NurbsCurve) -> Result<u64, WriteError> {
        if nurbs.weights.is_some() {
            // Rational form → complex RATIONAL_B_SPLINE_CURVE (Plan 3).
            use crate::entities::ComplexEntityHandler;
            crate::entities::geometry::rational_bspline_curve::RationalBsplineCurveHandler::write(
                self, nurbs,
            )
        } else {
            // Non-rational form → simple B_SPLINE_CURVE_WITH_KNOTS (Plan 5).
            use crate::entities::SimpleEntityHandler;
            crate::entities::geometry::b_spline_curve_with_knots::BSplineCurveWithKnotsHandler::write(self, nurbs)
        }
    }

    fn emit_nurbs_surface(&mut self, nurbs: NurbsSurface) -> Result<u64, WriteError> {
        if nurbs.weights.is_some() {
            // Rational form → complex RATIONAL_B_SPLINE_SURFACE (Plan 5 C3).
            use crate::entities::ComplexEntityHandler;
            crate::entities::geometry::rational_bspline_surface::RationalBsplineSurfaceHandler::write(self, nurbs)
        } else {
            // Non-rational form → simple B_SPLINE_SURFACE_WITH_KNOTS (Plan 5 C2).
            use crate::entities::SimpleEntityHandler;
            crate::entities::geometry::b_spline_surface_with_knots::BSplineSurfaceWithKnotsHandler::write(self, nurbs)
        }
    }

    fn emit_surface_of_revolution(&mut self, r: SurfaceOfRevolution) -> Result<u64, WriteError> {
        // Plan 5 stage C6: dispatch through EntityHandler trait.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::surface_of_revolution::SurfaceOfRevolutionHandler::write(self, r)
    }

    fn emit_surface_of_linear_extrusion(
        &mut self,
        e: SurfaceOfLinearExtrusion,
    ) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::surface_of_linear_extrusion::SurfaceOfLinearExtrusionHandler::write(self, e)
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
        // Plan 5 stage C1: dispatch through the EntityHandler trait.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::axis1_placement::Axis1PlacementHandler::write(self, id)
    }
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
        // Plan 5.5 stage C2: dispatch through EntityHandler trait.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::cartesian_point_2d::CartesianPoint2dHandler::write(self, id)
    }

    pub(crate) fn emit_direction_2d(&mut self, id: Direction2dId) -> Result<u64, WriteError> {
        // Plan 5.5 stage C2: dispatch through EntityHandler trait.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::direction_2d::Direction2dHandler::write(self, id)
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

    pub(crate) fn emit_pcurve(&mut self, pc: Pcurve) -> Result<u64, WriteError> {
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
        // Plan 5 stage C4: dispatch through EntityHandler trait.
        use crate::entities::SimpleEntityHandler;
        let is_seam = pcurves.len() >= 2
            && pcurves
                .iter()
                .all(|p| p.basis_surface == pcurves[0].basis_surface);
        let pcurves_owned = pcurves.to_vec();
        if is_seam {
            crate::entities::geometry::seam_curve::SeamCurveHandler::write(
                self,
                (curve_3d_ref, pcurves_owned),
            )
        } else {
            crate::entities::geometry::surface_curve::SurfaceCurveHandler::write(
                self,
                (curve_3d_ref, pcurves_owned),
            )
        }
    }
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
