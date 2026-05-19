//! Geometry pool emission: points, directions, placements, curves, surfaces.

#![allow(clippy::doc_markdown)]

use super::WriteBuffer;
use crate::ir::{
    Circle2, Circle3, CompositeCurve, ConicalSurface, Curve, Curve2d, Curve2dId, CurveId,
    CylindricalSurface, Direction2dId, Direction3, DirectionId, Ellipse2, Ellipse3, Hyperbola,
    Line2, Line3, NurbsCurve, NurbsCurve2d, NurbsSurface, OffsetCurve3d, Parabola, Pcurve,
    Placement1dId, Placement2dId, Placement3dId, Plane3, Point2dId, PointId, Polyline, Polyline2d,
    RectangularTrimmedSurface, SphericalSurface, StepModel, Surface, SurfaceId,
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
            Curve::Polyline(polyline) => self.emit_polyline(polyline)?,
            Curve::Hyperbola(h) => self.emit_hyperbola(h)?,
            Curve::Parabola(p) => self.emit_parabola(p)?,
            Curve::OffsetCurve3d(oc) => self.emit_offset_curve_3d(oc)?,
        };
        self.curve_ids.insert(id, n);
        Ok(n)
    }

    fn emit_polyline(&mut self, polyline: Polyline) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::polyline::PolylineHandler::write(self, polyline)
    }

    fn emit_hyperbola(&mut self, h: Hyperbola) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::hyperbola::HyperbolaHandler::write(self, h)
    }

    fn emit_parabola(&mut self, p: Parabola) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::parabola::ParabolaHandler::write(self, p)
    }

    fn emit_offset_curve_3d(&mut self, oc: OffsetCurve3d) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::offset_curve_3d::OffsetCurve3dHandler::write(self, oc)
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
            Surface::RectangularTrimmed(rts) => self.emit_rectangular_trimmed_surface(rts)?,
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

    fn emit_rectangular_trimmed_surface(
        &mut self,
        rts: RectangularTrimmedSurface,
    ) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::rectangular_trimmed_surface::RectangularTrimmedSurfaceHandler::write(self, rts)
    }

    fn emit_nurbs_curve(&mut self, nurbs: NurbsCurve) -> Result<u64, WriteError> {
        // Per the "compatibility-first form" policy (ROADMAP 전략 원칙):
        // emit only the de-facto-universal B_SPLINE_CURVE_WITH_KNOTS form
        // (or its rational complex variant). QUASI_UNIFORM_CURVE is read
        // back via the QUC handler but never emitted, because corpus
        // measurement showed BSCWK at 99.86% (700:1) prevalence — minority
        // forms risk reader incompatibility on some CAD targets.
        if nurbs.weights().is_some() {
            use crate::entities::ComplexEntityHandler;
            crate::entities::geometry::rational_bspline_curve::RationalBsplineCurveHandler::write(
                self, nurbs,
            )
        } else {
            use crate::entities::SimpleEntityHandler;
            crate::entities::geometry::b_spline_curve_with_knots::BSplineCurveWithKnotsHandler::write(self, nurbs)
        }
    }

    fn emit_nurbs_surface(&mut self, nurbs: NurbsSurface) -> Result<u64, WriteError> {
        // See `emit_nurbs_curve` for the compatibility-first form policy.
        // BSSWK measured at 99.95% (2,140:1) prevalence in corpus.
        if nurbs.weights().is_some() {
            use crate::entities::ComplexEntityHandler;
            crate::entities::geometry::rational_bspline_surface::RationalBsplineSurfaceHandler::write(self, nurbs)
        } else {
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
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::offset_surface::OffsetSurfaceHandler::write(self, o)
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

    pub(crate) fn emit_axis2_placement_2d(&mut self, id: Placement2dId) -> Result<u64, WriteError> {
        // Plan 5.5 stage C3: dispatch through EntityHandler trait.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::axis2_placement_2d::Axis2Placement2dHandler::write(self, id)
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
            Curve2d::Polyline(polyline) => self.emit_polyline_2d(polyline)?,
        };
        self.curve_2d_ids.insert(id, n);
        Ok(n)
    }

    fn emit_polyline_2d(&mut self, polyline: Polyline2d) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::polyline_2d::Polyline2dHandler::write(self, polyline)
    }

    fn emit_line_2d(&mut self, line: Line2) -> Result<u64, WriteError> {
        // Plan 5.5 stage C4: dispatch through EntityHandler trait.
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::line_2d::Line2dHandler::write(self, line)
    }

    fn emit_circle_2d(&mut self, c: Circle2) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::circle_2d::Circle2dHandler::write(self, c)
    }

    fn emit_ellipse_2d(&mut self, e: Ellipse2) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::geometry::ellipse_2d::Ellipse2dHandler::write(self, e)
    }

    fn emit_nurbs_curve_2d(&mut self, nurbs: &NurbsCurve2d) -> Result<u64, WriteError> {
        if nurbs.weights().is_some() {
            // Rational form → complex RATIONAL_B_SPLINE_CURVE (2D).
            use crate::entities::ComplexEntityHandler;
            crate::entities::geometry::rational_bspline_curve_2d::RationalBsplineCurve2dHandler::write(self, nurbs.clone())
        } else {
            // Non-rational form → simple B_SPLINE_CURVE_WITH_KNOTS (2D).
            use crate::entities::SimpleEntityHandler;
            crate::entities::geometry::b_spline_curve_2d_with_knots::BSplineCurve2dWithKnotsHandler::write(self, nurbs.clone())
        }
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
