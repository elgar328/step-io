//! Read-only b-rep geometry navigation handles over a [`Scene`].
//!
//! The walk is `Solid → Face → Surface` and `Face → Bound → Edge → Curve /
//! Vertex → Point`. Surfaces and curves report their analytic kind
//! (`Plane`, `Cylindrical`, …); a shape outside the handled set comes back
//! as `SurfaceKind::Other` / `CurveKind::Other` carrying the raw STEP
//! keyword, never hidden.

// Every public method here is a pure read accessor; `#[must_use]` on each would be
// pure noise, so allow the pedantic lint module-wide.
#![allow(clippy::must_use_candidate)]

use crate::RefGraph;
use crate::StepModel;
use crate::scene::{Ctx, Scene};
// Alias the generated model module: bare supertype structs (`Face`, `Surface`,
// `Curve`, `Edge`, `Vertex`, `Point`, `Loop`) would shadow our handle names, so we
// never import them by bare name — everything generated is reached via `m::`.
use crate::generated::model as m;

/// Model-wide by-type queries. Naming convention: `all_*` returns every instance
/// of a type in the whole model (the storage is flat — one arena per type — so a
/// model-level query is always "find all of this type"); navigating *from* a
/// handle (e.g. `solid.faces()`) is unprefixed and scoped to that element.
impl Scene<'_> {
    /// Every b-rep solid in the model — `MANIFOLD_SOLID_BREP` and
    /// `BREP_WITH_VOIDS` (solids with internal cavities).
    pub fn all_solids(&self) -> impl Iterator<Item = Solid<'_>> + '_ {
        let cx = self.ctx();
        let manifold = (0..cx.model.manifold_solid_brep_arena.items.len())
            .map(move |i| Solid::from_id(cx, m::ManifoldSolidBrepId(i)));
        let with_voids = (0..cx.model.brep_with_voids_arena.items.len())
            .map(move |i| Solid::from_void_id(cx, m::BrepWithVoidsId(i)));
        manifold.chain(with_voids)
    }

    /// Every face in the model (`ADVANCED_FACE` and `FACE_SURFACE`).
    pub fn all_faces(&self) -> impl Iterator<Item = Face<'_>> + '_ {
        let cx = self.ctx();
        let advanced = (0..cx.model.advanced_face_arena.items.len()).map(move |i| Face {
            cx,
            which: FaceImpl::Advanced(m::AdvancedFaceId(i)),
        });
        let surfaces = (0..cx.model.face_surface_arena.items.len()).map(move |i| Face {
            cx,
            which: FaceImpl::Surface(m::FaceSurfaceId(i)),
        });
        advanced.chain(surfaces)
    }

    /// Every edge in the model (`EDGE_CURVE`).
    pub fn all_edges(&self) -> impl Iterator<Item = Edge<'_>> + '_ {
        let cx = self.ctx();
        (0..cx.model.edge_curve_arena.items.len()).map(move |i| Edge {
            cx,
            id: m::EdgeCurveId(i),
        })
    }

    /// Every vertex in the model (`VERTEX_POINT`).
    pub fn all_vertices(&self) -> impl Iterator<Item = Vertex<'_>> + '_ {
        let cx = self.ctx();
        (0..cx.model.vertex_point_arena.items.len()).map(move |i| Vertex {
            cx,
            id: m::VertexPointId(i),
        })
    }

    /// Every geometric point in the model (`CARTESIAN_POINT`).
    pub fn all_points(&self) -> impl Iterator<Item = Point<'_>> + '_ {
        let cx = self.ctx();
        (0..cx.model.cartesian_point_arena.items.len()).map(move |i| Point {
            cx,
            id: m::CartesianPointId(i),
        })
    }
}

// ---------------------------------------------------------------------------
// Solid
// ---------------------------------------------------------------------------

/// A b-rep solid — a `MANIFOLD_SOLID_BREP`, or a `BREP_WITH_VOIDS` (a body with
/// internal cavities). Use [`voids`](Solid::voids) to reach the cavity shells.
#[derive(Clone, Copy)]
pub struct Solid<'m> {
    cx: Ctx<'m>,
    which: SolidImpl,
}

#[derive(Clone, Copy)]
enum SolidImpl {
    Manifold(m::ManifoldSolidBrepId),
    WithVoids(m::BrepWithVoidsId),
}

impl<'m> Solid<'m> {
    /// Construct a solid handle from a `MANIFOLD_SOLID_BREP` id — used by other
    /// scene modules (e.g. `product`) to bridge into geometry; the fields are
    /// otherwise private.
    pub(crate) fn from_id(cx: Ctx<'m>, id: m::ManifoldSolidBrepId) -> Self {
        Solid {
            cx,
            which: SolidImpl::Manifold(id),
        }
    }

    /// Construct a solid handle from a `BREP_WITH_VOIDS` id.
    pub(crate) fn from_void_id(cx: Ctx<'m>, id: m::BrepWithVoidsId) -> Self {
        Solid {
            cx,
            which: SolidImpl::WithVoids(id),
        }
    }

    /// (`name`, `outer` shell) — `MANIFOLD_SOLID_BREP` and `BREP_WITH_VOIDS`
    /// share this layout.
    fn name_and_outer(&self) -> (&'m str, &'m m::ClosedShellRef) {
        match self.which {
            SolidImpl::Manifold(i) => {
                let s = self.cx.model.manifold_solid_brep_arena.get(i.0);
                (&s.name, &s.outer)
            }
            SolidImpl::WithVoids(i) => {
                let s = self.cx.model.brep_with_voids_arena.get(i.0);
                (&s.name, &s.outer)
            }
        }
    }

    pub fn name(&self) -> &'m str {
        self.name_and_outer().0
    }

    /// This solid's global identity (a `Copy` key for maps / deduplication; two
    /// handles to the same entity share one `key()`).
    pub fn key(&self) -> m::EntityKey {
        match self.which {
            SolidImpl::Manifold(i) => m::EntityKey::ManifoldSolidBrep(i),
            SolidImpl::WithVoids(i) => m::EntityKey::BrepWithVoids(i),
        }
    }

    /// The colour styled onto this solid, if any.
    pub fn color(&self) -> Option<crate::scene::presentation::Rgb> {
        crate::scene::presentation::colour_of(self.cx, self.key())
    }

    /// The transparency styled onto this solid (`0.0` opaque … `1.0` fully
    /// transparent), if any.
    pub fn transparency(&self) -> Option<f64> {
        crate::scene::presentation::transparency_of(self.cx, self.key())
    }

    /// The name of the presentation layer this solid belongs to, if any.
    pub fn layer(&self) -> Option<&'m str> {
        crate::scene::presentation::layer_of(self.cx, self.key())
    }

    /// Whether this solid is visible — `false` if an `INVISIBILITY` hides it (via
    /// its styled item or its layer). Defaults to visible.
    pub fn is_visible(&self) -> bool {
        crate::scene::presentation::is_visible(self.cx, self.key())
    }

    /// Faces of the solid's outer shell.
    pub fn faces(&self) -> impl Iterator<Item = Face<'m>> + 'm {
        let cx = self.cx;
        resolve_closed_shell(cx.model, self.name_and_outer().1)
            .into_iter()
            .flat_map(|shell| shell.cfs_faces.iter())
            .filter_map(move |fr| Face::from_ref(cx, fr))
    }

    /// Faces of each internal void shell — one inner `Vec` per cavity of a
    /// `BREP_WITH_VOIDS`. Empty for a plain `MANIFOLD_SOLID_BREP`.
    pub fn voids(&self) -> Vec<Vec<Face<'m>>> {
        let cx = self.cx;
        let SolidImpl::WithVoids(id) = self.which else {
            return Vec::new();
        };
        cx.model
            .brep_with_voids_arena
            .get(id.0)
            .voids
            .iter()
            .map(|ocs| {
                let shell = match ocs {
                    m::OrientedClosedShellRef::OrientedClosedShell(o) => resolve_closed_shell(
                        cx.model,
                        &cx.model
                            .oriented_closed_shell_arena
                            .get(o.0)
                            .closed_shell_element,
                    ),
                    m::OrientedClosedShellRef::Complex(_) => None,
                };
                shell
                    .into_iter()
                    .flat_map(|s| s.cfs_faces.iter())
                    .filter_map(|fr| Face::from_ref(cx, fr))
                    .collect()
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Face
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum FaceImpl {
    Advanced(m::AdvancedFaceId),
    Surface(m::FaceSurfaceId),
}

/// A bounded face of a solid (`ADVANCED_FACE` / `FACE_SURFACE`) — a surface
/// trimmed by boundary loops, with an orientation.
#[derive(Clone, Copy)]
pub struct Face<'m> {
    cx: Ctx<'m>,
    which: FaceImpl,
}

impl<'m> Face<'m> {
    /// Construct a face handle from an `ADVANCED_FACE` id — used by other scene
    /// modules (e.g. `pmi` feature geometry) to bridge into geometry.
    pub(crate) fn from_advanced_face(cx: Ctx<'m>, id: m::AdvancedFaceId) -> Self {
        Face {
            cx,
            which: FaceImpl::Advanced(id),
        }
    }

    fn from_ref(cx: Ctx<'m>, r: &m::FaceRef) -> Option<Self> {
        match r {
            m::FaceRef::AdvancedFace(i) => Some(Face {
                cx,
                which: FaceImpl::Advanced(*i),
            }),
            m::FaceRef::FaceSurface(i) => Some(Face {
                cx,
                which: FaceImpl::Surface(*i),
            }),
            // Bare `Face` carries no geometry; complex faces are out of scope.
            m::FaceRef::Face(_) | m::FaceRef::Complex(_) => None,
        }
    }

    /// (`name`, `bounds`, `face_geometry`, `same_sense`) — `ADVANCED_FACE` and
    /// `FACE_SURFACE` share this layout.
    fn parts(&self) -> (&'m str, &'m [m::FaceBoundRef], &'m m::SurfaceRef, bool) {
        match self.which {
            FaceImpl::Advanced(i) => {
                let f = self.cx.model.advanced_face_arena.get(i.0);
                (&f.name, &f.bounds, &f.face_geometry, f.same_sense)
            }
            FaceImpl::Surface(i) => {
                let f = self.cx.model.face_surface_arena.get(i.0);
                (&f.name, &f.bounds, &f.face_geometry, f.same_sense)
            }
        }
    }

    pub fn name(&self) -> &'m str {
        self.parts().0
    }

    pub fn same_sense(&self) -> bool {
        self.parts().3
    }

    /// The underlying (unbounded) surface this face sits on.
    pub fn surface(&self) -> Surface<'m> {
        Surface {
            cx: self.cx,
            r: self.parts().2,
        }
    }

    /// The boundary loops trimming this face.
    pub fn bounds(&self) -> impl Iterator<Item = Bound<'m>> + 'm {
        let cx = self.cx;
        self.parts()
            .1
            .iter()
            .filter_map(move |br| Bound::from_ref(cx, br))
    }

    /// The point set used to size a bounded patch for an unbounded surface:
    /// the control points of every boundary edge's NURBS form. The control
    /// hull contains the curve, so extents taken from these points are a
    /// *guaranteed* cover — a curved edge bulging past its vertices is
    /// included (vertices alone could under-cover). An edge that does not
    /// convert falls back to its two vertex points.
    fn boundary_points(&self) -> Vec<[f64; 3]> {
        let mut points = Vec::new();
        for bound in self.bounds() {
            for edge in bound.edges() {
                if let Some(n) = edge.to_nurbs() {
                    points.extend(n.control_points);
                } else {
                    for v in [edge.start(), edge.end()].into_iter().flatten() {
                        if let Some(pt) = v.point() {
                            points.push(pt.xyz());
                        }
                    }
                }
            }
        }
        points
    }

    /// The NURBS ([`NurbsSurface`](crate::scene::NurbsSurface)) form of this
    /// face's geometry. A closed analytic surface (sphere, torus, B-spline) is
    /// returned whole via [`Surface::to_nurbs`]. An *unbounded* surface (plane,
    /// cylinder, cone, linear extrusion) is sized to the control points of its
    /// boundary edges' NURBS forms — a guaranteed cover of the boundary — and
    /// is the untrimmed base patch (the edge loops trim it separately).
    /// `None` if the kind is unsupported or the bounds are degenerate.
    pub fn to_nurbs(&self) -> Option<crate::scene::nurbs::NurbsSurface> {
        if let Some(n) = self.surface().to_nurbs() {
            return Some(n);
        }
        let cx = self.cx;
        let pts = self.boundary_points();
        match self.surface().kind() {
            SurfaceKind::Plane(p) => crate::scene::nurbs::plane_patch(cx, p, &pts),
            SurfaceKind::Cylindrical(c) => {
                crate::scene::nurbs::cylinder_cone_patch(cx, &c.position, c.radius, 0.0, &pts)
            }
            SurfaceKind::Conical(c) => crate::scene::nurbs::cylinder_cone_patch(
                cx,
                &c.position,
                c.radius,
                c.semi_angle,
                &pts,
            ),
            SurfaceKind::LinearExtrusion(s) => {
                let profile = Curve::from_curve_ref(cx, &s.swept_curve).to_nurbs()?;
                crate::scene::nurbs::extrude_patch(cx, &profile, &s.extrusion_axis, &pts)
            }
            // Bounded profiles were already handled whole by the surface-level
            // conversion above; what remains is the unbounded LINE profile
            // (a lathe-style cylinder/cone/hyperboloid), bounded here by the
            // face's vertices.
            SurfaceKind::Revolution(s) => {
                let CurveKind::Line(line) = Curve::from_curve_ref(cx, &s.swept_curve).kind() else {
                    return None;
                };
                let frame = crate::scene::nurbs::axis1_frame(cx, &s.axis_position)?;
                let seg = crate::scene::nurbs::revolved_line_segment(cx, line, frame, &pts)?;
                Some(crate::scene::nurbs::revolve_curve(&seg, frame))
            }
            _ => None,
        }
    }

    /// This face's global identity (a `Copy` key for maps / deduplication; two
    /// handles to the same entity share one `key()`).
    pub fn key(&self) -> m::EntityKey {
        match self.which {
            FaceImpl::Advanced(i) => m::EntityKey::AdvancedFace(i),
            FaceImpl::Surface(i) => m::EntityKey::FaceSurface(i),
        }
    }

    /// The colour styled onto this face, if any.
    pub fn color(&self) -> Option<crate::scene::presentation::Rgb> {
        crate::scene::presentation::colour_of(self.cx, self.key())
    }

    /// The transparency styled onto this face (`0.0` opaque … `1.0` fully
    /// transparent), if any.
    pub fn transparency(&self) -> Option<f64> {
        crate::scene::presentation::transparency_of(self.cx, self.key())
    }

    /// The name of the presentation layer this face belongs to, if any.
    pub fn layer(&self) -> Option<&'m str> {
        crate::scene::presentation::layer_of(self.cx, self.key())
    }

    /// Whether this face is visible — `false` if an `INVISIBILITY` hides it (via
    /// its styled item or its layer). Defaults to visible.
    pub fn is_visible(&self) -> bool {
        crate::scene::presentation::is_visible(self.cx, self.key())
    }

    /// The b-rep solid this face belongs to, if any (reverse: a `CLOSED_SHELL`
    /// listing this face → a `MANIFOLD_SOLID_BREP` or `BREP_WITH_VOIDS` whose
    /// shell is that shell, directly or through an `ORIENTED_CLOSED_SHELL`).
    pub fn solid(&self) -> Option<Solid<'m>> {
        let rg = self.cx.ref_graph();
        for &r in rg.referrers(self.key()) {
            if let m::EntityKey::ClosedShell(cs) = r {
                if let Some(which) = closed_shell_to_solid(rg, cs) {
                    return Some(Solid { cx: self.cx, which });
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Surface
// ---------------------------------------------------------------------------

/// The geometry under a [`Face`]. Call [`Surface::kind`] to dispatch.
#[derive(Clone, Copy)]
pub struct Surface<'m> {
    cx: Ctx<'m>,
    r: &'m m::SurfaceRef,
}

/// The concrete surface geometry. Common kinds are typed; everything else is
/// `Other(keyword)` (the surface still exists — it is surfaced, not dropped).
#[derive(Clone, Copy, Debug)]
pub enum SurfaceKind<'m> {
    Plane(&'m m::Plane),
    Cylindrical(&'m m::CylindricalSurface),
    Conical(&'m m::ConicalSurface),
    Spherical(&'m m::SphericalSurface),
    Toroidal(&'m m::ToroidalSurface),
    BSpline(&'m m::BSplineSurface),
    BSplineWithKnots(&'m m::BSplineSurfaceWithKnots),
    LinearExtrusion(&'m m::SurfaceOfLinearExtrusion),
    Revolution(&'m m::SurfaceOfRevolution),
    QuasiUniform(&'m m::QuasiUniformSurface),
    Uniform(&'m m::UniformSurface),
    Bezier(&'m m::BezierSurface),
    Other(&'static str),
}

impl<'m> Surface<'m> {
    /// This surface's global identity (a `Copy` key for maps / deduplication; a
    /// surface shared by several faces yields the same `key()`).
    pub fn key(&self) -> m::EntityKey {
        self.r.entity_key()
    }

    /// The rational-B-spline ([`NurbsSurface`](crate::scene::NurbsSurface))
    /// form of this surface, if the kind is supported. Only *closed* analytic
    /// surfaces convert as-is — a full sphere or torus. Open surfaces (plane,
    /// cylinder, cone) are unbounded and need a face's bounds, so they return
    /// `None` here — use [`Face::to_nurbs`] instead, which has the bounds.
    pub fn to_nurbs(&self) -> Option<crate::scene::nurbs::NurbsSurface> {
        let cx = self.cx;
        match self.r {
            m::SurfaceRef::SphericalSurface(i) => {
                crate::scene::nurbs::sphere_to_nurbs(cx, cx.model.spherical_surface_arena.get(i.0))
            }
            m::SurfaceRef::ToroidalSurface(i) => {
                crate::scene::nurbs::torus_to_nurbs(cx, cx.model.toroidal_surface_arena.get(i.0))
            }
            m::SurfaceRef::BSplineSurfaceWithKnots(i) => {
                crate::scene::nurbs::bspline_surf_wk_to_nurbs(
                    cx,
                    cx.model.b_spline_surface_with_knots_arena.get(i.0),
                )
            }
            // A rational B-spline surface arrives as a complex instance.
            m::SurfaceRef::Complex(i) => crate::scene::nurbs::complex_bspline_surface_to_nurbs(
                cx,
                &cx.model.complex_unit_arena.get(i.0).parts,
            ),
            // A full revolution is closed in u, so a bounded profile gives the
            // whole surface (an unbounded LINE profile needs a face's bounds).
            m::SurfaceRef::SurfaceOfRevolution(i) => {
                let s = cx.model.surface_of_revolution_arena.get(i.0);
                let profile = Curve::from_curve_ref(cx, &s.swept_curve).to_nurbs()?;
                let frame = crate::scene::nurbs::axis1_frame(cx, &s.axis_position)?;
                Some(crate::scene::nurbs::revolve_curve(&profile, frame))
            }
            // Subtypes whose knot vectors are derived by a standard rule.
            m::SurfaceRef::UniformSurface(i) => {
                let s = cx.model.uniform_surface_arena.get(i.0);
                crate::scene::nurbs::uniform_family_surface_to_nurbs(
                    cx,
                    (s.u_degree, s.v_degree),
                    &s.control_points_list,
                    crate::scene::nurbs::KnotFamily::Uniform,
                )
            }
            m::SurfaceRef::QuasiUniformSurface(i) => {
                let s = cx.model.quasi_uniform_surface_arena.get(i.0);
                crate::scene::nurbs::uniform_family_surface_to_nurbs(
                    cx,
                    (s.u_degree, s.v_degree),
                    &s.control_points_list,
                    crate::scene::nurbs::KnotFamily::QuasiUniform,
                )
            }
            m::SurfaceRef::BezierSurface(i) => {
                let s = cx.model.bezier_surface_arena.get(i.0);
                crate::scene::nurbs::uniform_family_surface_to_nurbs(
                    cx,
                    (s.u_degree, s.v_degree),
                    &s.control_points_list,
                    crate::scene::nurbs::KnotFamily::Bezier,
                )
            }
            _ => None,
        }
    }

    pub fn kind(&self) -> SurfaceKind<'m> {
        let model = self.cx.model;
        match self.r {
            m::SurfaceRef::Plane(i) => SurfaceKind::Plane(model.plane_arena.get(i.0)),
            m::SurfaceRef::CylindricalSurface(i) => {
                SurfaceKind::Cylindrical(model.cylindrical_surface_arena.get(i.0))
            }
            m::SurfaceRef::ConicalSurface(i) => {
                SurfaceKind::Conical(model.conical_surface_arena.get(i.0))
            }
            m::SurfaceRef::SphericalSurface(i) => {
                SurfaceKind::Spherical(model.spherical_surface_arena.get(i.0))
            }
            m::SurfaceRef::ToroidalSurface(i) => {
                SurfaceKind::Toroidal(model.toroidal_surface_arena.get(i.0))
            }
            m::SurfaceRef::BSplineSurface(i) => {
                SurfaceKind::BSpline(model.b_spline_surface_arena.get(i.0))
            }
            m::SurfaceRef::BSplineSurfaceWithKnots(i) => {
                SurfaceKind::BSplineWithKnots(model.b_spline_surface_with_knots_arena.get(i.0))
            }
            m::SurfaceRef::BezierSurface(i) => {
                SurfaceKind::Bezier(model.bezier_surface_arena.get(i.0))
            }
            m::SurfaceRef::BoundedSurface(_) => SurfaceKind::Other("BOUNDED_SURFACE"),
            m::SurfaceRef::DegenerateToroidalSurface(_) => {
                SurfaceKind::Other("DEGENERATE_TOROIDAL_SURFACE")
            }
            m::SurfaceRef::ElementarySurface(_) => SurfaceKind::Other("ELEMENTARY_SURFACE"),
            m::SurfaceRef::OffsetSurface(_) => SurfaceKind::Other("OFFSET_SURFACE"),
            m::SurfaceRef::QuasiUniformSurface(i) => {
                SurfaceKind::QuasiUniform(model.quasi_uniform_surface_arena.get(i.0))
            }
            m::SurfaceRef::RationalBSplineSurface(_) => {
                SurfaceKind::Other("RATIONAL_B_SPLINE_SURFACE")
            }
            m::SurfaceRef::Surface(_) => SurfaceKind::Other("SURFACE"),
            m::SurfaceRef::SurfaceOfLinearExtrusion(i) => {
                SurfaceKind::LinearExtrusion(model.surface_of_linear_extrusion_arena.get(i.0))
            }
            m::SurfaceRef::SurfaceOfRevolution(i) => {
                SurfaceKind::Revolution(model.surface_of_revolution_arena.get(i.0))
            }
            m::SurfaceRef::SweptSurface(_) => SurfaceKind::Other("SWEPT_SURFACE"),
            m::SurfaceRef::UniformSurface(i) => {
                SurfaceKind::Uniform(model.uniform_surface_arena.get(i.0))
            }
            m::SurfaceRef::Complex(_) => SurfaceKind::Other("COMPLEX"),
        }
    }
}

// ---------------------------------------------------------------------------
// Bound (face boundary loop)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum BoundImpl {
    Inner(m::FaceBoundId),
    Outer(m::FaceOuterBoundId),
}

/// A boundary loop of a [`Face`] (`FACE_BOUND` / `FACE_OUTER_BOUND`).
#[derive(Clone, Copy)]
pub struct Bound<'m> {
    cx: Ctx<'m>,
    which: BoundImpl,
}

impl<'m> Bound<'m> {
    fn from_ref(cx: Ctx<'m>, r: &m::FaceBoundRef) -> Option<Self> {
        match r {
            m::FaceBoundRef::FaceBound(i) => Some(Bound {
                cx,
                which: BoundImpl::Inner(*i),
            }),
            m::FaceBoundRef::FaceOuterBound(i) => Some(Bound {
                cx,
                which: BoundImpl::Outer(*i),
            }),
            m::FaceBoundRef::Complex(_) => None,
        }
    }

    /// `true` for `FACE_OUTER_BOUND`.
    pub fn is_outer(&self) -> bool {
        matches!(self.which, BoundImpl::Outer(_))
    }

    /// This bound's global identity (a `Copy` key for maps / deduplication).
    pub fn key(&self) -> m::EntityKey {
        match self.which {
            BoundImpl::Inner(i) => m::EntityKey::FaceBound(i),
            BoundImpl::Outer(i) => m::EntityKey::FaceOuterBound(i),
        }
    }

    fn loop_ref(&self) -> &'m m::LoopRef {
        match self.which {
            BoundImpl::Inner(i) => &self.cx.model.face_bound_arena.get(i.0).bound,
            BoundImpl::Outer(i) => &self.cx.model.face_outer_bound_arena.get(i.0).bound,
        }
    }

    /// Whether this loop runs in its stated sense with respect to the face —
    /// the `FACE_BOUND` `orientation` flag; `false` means the loop is used
    /// reversed. One of the three orientation layers a kernel composes:
    /// [`Face::same_sense`] (surface normal), this flag (whole loop), and the
    /// per-edge flag of [`Bound::oriented_edges`].
    pub fn orientation(&self) -> bool {
        match self.which {
            BoundImpl::Inner(i) => self.cx.model.face_bound_arena.get(i.0).orientation,
            BoundImpl::Outer(i) => self.cx.model.face_outer_bound_arena.get(i.0).orientation,
        }
    }

    /// Edges of this boundary loop (only `EDGE_LOOP` loops carry edges),
    /// without their per-edge orientation — use [`Bound::oriented_edges`] to
    /// assemble a wire.
    pub fn edges(&self) -> impl Iterator<Item = Edge<'m>> + 'm {
        self.oriented_edges().map(|(e, _)| e)
    }

    /// Edges of this boundary loop with each `ORIENTED_EDGE`'s orientation:
    /// `true` means the edge is traversed in its own start → end direction at
    /// this position in the loop, `false` reversed — what a kernel needs to
    /// assemble the loop into a wire without end-matching heuristics.
    pub fn oriented_edges(&self) -> impl Iterator<Item = (Edge<'m>, bool)> + 'm {
        let cx = self.cx;
        resolve_edge_loop(cx.model, self.loop_ref())
            .into_iter()
            .flat_map(|el| el.edge_list.iter())
            .filter_map(move |oer| {
                let oe = resolve_oriented_edge(cx.model, oer)?;
                Some((Edge::from_edge_ref(cx, &oe.edge_element)?, oe.orientation))
            })
    }
}

// ---------------------------------------------------------------------------
// Edge
// ---------------------------------------------------------------------------

/// An edge of a loop (`EDGE_CURVE`) — a curve bounded by two vertices.
#[derive(Clone, Copy)]
pub struct Edge<'m> {
    cx: Ctx<'m>,
    id: m::EdgeCurveId,
}

impl<'m> Edge<'m> {
    /// Construct an edge handle from an `EDGE_CURVE` id — used by other scene
    /// modules (e.g. `pmi` feature geometry) to bridge into geometry.
    pub(crate) fn from_id(cx: Ctx<'m>, id: m::EdgeCurveId) -> Self {
        Edge { cx, id }
    }

    fn from_edge_ref(cx: Ctx<'m>, r: &m::EdgeRef) -> Option<Self> {
        match r {
            m::EdgeRef::EdgeCurve(i) => Some(Edge { cx, id: *i }),
            // Bare `Edge`, nested `OrientedEdge`, and complex edges are out of scope.
            m::EdgeRef::Edge(_) | m::EdgeRef::OrientedEdge(_) | m::EdgeRef::Complex(_) => None,
        }
    }

    fn raw(&self) -> &'m m::EdgeCurve {
        self.cx.model.edge_curve_arena.get(self.id.0)
    }

    /// This edge's global identity (a `Copy` key for maps / deduplication; a
    /// shared edge reached from two faces yields the same `key()`).
    pub fn key(&self) -> m::EntityKey {
        m::EntityKey::EdgeCurve(self.id)
    }

    pub fn start(&self) -> Option<Vertex<'m>> {
        Vertex::from_ref(self.cx, &self.raw().edge_start)
    }

    pub fn end(&self) -> Option<Vertex<'m>> {
        Vertex::from_ref(self.cx, &self.raw().edge_end)
    }

    /// The underlying (unbounded) curve this edge lies on.
    pub fn curve(&self) -> Curve<'m> {
        Curve::from_curve_ref(self.cx, &self.raw().edge_geometry)
    }

    /// Whether the edge's start → end direction agrees with its curve's
    /// parameter direction (`EDGE_CURVE` `same_sense`). [`Edge::to_nurbs`]
    /// already accounts for it; an importer consuming the exact curve via
    /// [`Edge::curve`] composes it itself.
    pub fn same_sense(&self) -> bool {
        self.raw().same_sense
    }

    /// The NURBS ([`NurbsCurve`](crate::scene::NurbsCurve)) form of this
    /// edge: its geometry bounded between the two vertex points, in the edge's
    /// direction (start → end). Contrast with [`Curve::to_nurbs`], which
    /// converts the *whole* curve — here an unbounded line becomes the finite
    /// segment, a circle the arc between the vertices, and an edge whose
    /// vertices coincide (a closed edge) the whole closed curve.
    pub fn to_nurbs(&self) -> Option<crate::scene::nurbs::NurbsCurve> {
        let cx = self.cx;
        let coords = |v: Option<Vertex<'m>>| Some(v?.point()?.xyz());
        let (Some(p0), Some(p1)) = (coords(self.start()), coords(self.end())) else {
            cx.warn("EDGE_CURVE.to_nurbs: unresolved edge vertex".to_owned());
            return None;
        };
        let raw = self.raw();
        crate::scene::nurbs::edge_to_nurbs(cx, &raw.edge_geometry, p0, p1, raw.same_sense)
    }
}

// ---------------------------------------------------------------------------
// Curve
// ---------------------------------------------------------------------------

/// The geometry under an [`Edge`], or a curve a feature designates. Call
/// [`Curve::kind`] to dispatch.
#[derive(Clone, Copy)]
pub struct Curve<'m> {
    cx: Ctx<'m>,
    which: CurveImpl,
}

/// The concrete curve geometry. Common kinds are typed; everything else is
/// `Other(keyword)`.
#[derive(Clone, Copy, Debug)]
pub enum CurveKind<'m> {
    Line(&'m m::Line),
    Circle(&'m m::Circle),
    Ellipse(&'m m::Ellipse),
    BSpline(&'m m::BSplineCurve),
    BSplineWithKnots(&'m m::BSplineCurveWithKnots),
    Trimmed(&'m m::TrimmedCurve),
    Polyline(&'m m::Polyline),
    Composite(&'m m::CompositeCurve),
    QuasiUniform(&'m m::QuasiUniformCurve),
    Uniform(&'m m::UniformCurve),
    Bezier(&'m m::BezierCurve),
    SurfaceCurve(&'m m::SurfaceCurve),
    SeamCurve(&'m m::SeamCurve),
    BoundedSurfaceCurve(&'m m::BoundedSurfaceCurve),
    IntersectionCurve(&'m m::IntersectionCurve),
    Other(&'static str),
}

// The curve reference is one id per curve subtype. An id-based cursor (like
// `Face`) keeps `Curve` `Copy` and lets it be built from an arena id — needed to
// reach a curve through a feature's `GEOMETRIC_ITEM_SPECIFIC_USAGE`, where only
// the id is available. This table generates the cursor enum, `key`, and the two
// constructors; `kind` is hand-written below.
macro_rules! curve_impl {
    ($( $V:ident, $id:ident );+ $(;)?) => {
        #[derive(Clone, Copy)]
        enum CurveImpl {
            $( $V(m::$id), )+
            Complex(m::ComplexUnitId),
        }

        impl<'m> Curve<'m> {
            /// This curve's global identity (a `Copy` key for maps / dedup; a
            /// curve shared by several edges yields the same `key()`).
            pub fn key(&self) -> m::EntityKey {
                match self.which {
                    $( CurveImpl::$V(i) => m::EntityKey::$V(i), )+
                    CurveImpl::Complex(i) => m::EntityKey::ComplexUnit(i),
                }
            }

            pub(crate) fn from_curve_ref(cx: Ctx<'m>, r: &m::CurveRef) -> Self {
                let which = match r {
                    $( m::CurveRef::$V(i) => CurveImpl::$V(*i), )+
                    m::CurveRef::Complex(i) => CurveImpl::Complex(*i),
                };
                Curve { cx, which }
            }

            /// A curve reached as a `GEOMETRIC_ITEM_SPECIFIC_USAGE` identified item
            /// (used by `pmi` feature geometry); `None` for a non-curve item.
            pub(crate) fn from_model_item(
                cx: Ctx<'m>,
                r: &m::GeometricModelItemRef,
            ) -> Option<Self> {
                let which = match r {
                    $( m::GeometricModelItemRef::$V(i) => CurveImpl::$V(*i), )+
                    _ => return None,
                };
                Some(Curve { cx, which })
            }
        }
    };
}

curve_impl! {
    BSplineCurve, BSplineCurveId;
    BSplineCurveWithKnots, BSplineCurveWithKnotsId;
    BezierCurve, BezierCurveId;
    BoundedCurve, BoundedCurveId;
    BoundedPcurve, BoundedPcurveId;
    BoundedSurfaceCurve, BoundedSurfaceCurveId;
    Circle, CircleId;
    CompositeCurve, CompositeCurveId;
    Conic, ConicId;
    Curve, CurveId;
    Ellipse, EllipseId;
    Hyperbola, HyperbolaId;
    IntersectionCurve, IntersectionCurveId;
    Line, LineId;
    Pcurve, PcurveId;
    Polyline, PolylineId;
    QuasiUniformCurve, QuasiUniformCurveId;
    RationalBSplineCurve, RationalBSplineCurveId;
    SeamCurve, SeamCurveId;
    SurfaceCurve, SurfaceCurveId;
    TrimmedCurve, TrimmedCurveId;
    UniformCurve, UniformCurveId;
}

impl<'m> Curve<'m> {
    /// The colour of this curve's line style (`CURVE_STYLE.curve_colour`), if any.
    pub fn color(&self) -> Option<crate::scene::presentation::Rgb> {
        crate::scene::presentation::curve_colour_of(self.cx, self.key())
    }

    /// The line width of this curve's style (`CURVE_STYLE.curve_width`, a raw
    /// measure value), if any.
    pub fn width(&self) -> Option<f64> {
        crate::scene::presentation::curve_width_of(self.cx, self.key())
    }

    /// The line-font name of this curve's style (`CURVE_STYLE.curve_font`, a line
    /// pattern like `continuous` / `dashed`), if any.
    pub fn line_font(&self) -> Option<&'m str> {
        crate::scene::presentation::curve_font_of(self.cx, self.key())
    }

    /// The rational-B-spline ([`NurbsCurve`](crate::scene::NurbsCurve))
    /// form of this curve, if the kind is supported (circle, ellipse, B-spline
    /// with knots). Returns the *full* analytic curve — trimming applied by an
    /// edge that uses this curve is **not** reflected here. `None` for a curve
    /// that needs bounds it alone lacks — an unbounded line, say; use
    /// [`Edge::to_nurbs`] there.
    pub fn to_nurbs(&self) -> Option<crate::scene::nurbs::NurbsCurve> {
        let cx = self.cx;
        match self.which {
            CurveImpl::Circle(i) => {
                crate::scene::nurbs::circle_to_nurbs(cx, cx.model.circle_arena.get(i.0))
            }
            CurveImpl::Ellipse(i) => {
                crate::scene::nurbs::ellipse_to_nurbs(cx, cx.model.ellipse_arena.get(i.0))
            }
            CurveImpl::BSplineCurveWithKnots(i) => crate::scene::nurbs::bspline_wk_to_nurbs(
                cx,
                cx.model.b_spline_curve_with_knots_arena.get(i.0),
            ),
            // A rational B-spline arrives as a complex instance; dig its parts.
            CurveImpl::Complex(i) => crate::scene::nurbs::complex_bspline_curve_to_nurbs(
                cx,
                &cx.model.complex_unit_arena.get(i.0).parts,
            ),
            // A trimmed line/circle/ellipse → a bounded segment/arc.
            CurveImpl::TrimmedCurve(i) => {
                crate::scene::nurbs::trimmed_to_nurbs(cx, cx.model.trimmed_curve_arena.get(i.0))
            }
            // Subtypes whose knot vector is derived by a standard rule.
            CurveImpl::UniformCurve(i) => {
                let c = cx.model.uniform_curve_arena.get(i.0);
                crate::scene::nurbs::uniform_family_curve_to_nurbs(
                    cx,
                    c.degree,
                    &c.control_points_list,
                    crate::scene::nurbs::KnotFamily::Uniform,
                )
            }
            CurveImpl::QuasiUniformCurve(i) => {
                let c = cx.model.quasi_uniform_curve_arena.get(i.0);
                crate::scene::nurbs::uniform_family_curve_to_nurbs(
                    cx,
                    c.degree,
                    &c.control_points_list,
                    crate::scene::nurbs::KnotFamily::QuasiUniform,
                )
            }
            CurveImpl::BezierCurve(i) => {
                let c = cx.model.bezier_curve_arena.get(i.0);
                crate::scene::nurbs::uniform_family_curve_to_nurbs(
                    cx,
                    c.degree,
                    &c.control_points_list,
                    crate::scene::nurbs::KnotFamily::Bezier,
                )
            }
            CurveImpl::Polyline(i) => {
                crate::scene::nurbs::polyline_to_nurbs(cx, cx.model.polyline_arena.get(i.0))
            }
            // A segment chain joined into one exact NURBS.
            CurveImpl::CompositeCurve(i) => {
                crate::scene::nurbs::composite_to_nurbs(cx, cx.model.composite_curve_arena.get(i.0))
            }
            // Surface-curve containers delegate to their 3D curve.
            CurveImpl::SurfaceCurve(i) => crate::scene::nurbs::surface_curve_3d_to_nurbs(
                cx,
                &cx.model.surface_curve_arena.get(i.0).curve_3d,
            ),
            CurveImpl::SeamCurve(i) => crate::scene::nurbs::surface_curve_3d_to_nurbs(
                cx,
                &cx.model.seam_curve_arena.get(i.0).curve_3d,
            ),
            CurveImpl::BoundedSurfaceCurve(i) => crate::scene::nurbs::surface_curve_3d_to_nurbs(
                cx,
                &cx.model.bounded_surface_curve_arena.get(i.0).curve_3d,
            ),
            CurveImpl::IntersectionCurve(i) => crate::scene::nurbs::surface_curve_3d_to_nurbs(
                cx,
                &cx.model.intersection_curve_arena.get(i.0).curve_3d,
            ),
            _ => None,
        }
    }

    pub fn kind(&self) -> CurveKind<'m> {
        let model = self.cx.model;
        match self.which {
            CurveImpl::Line(i) => CurveKind::Line(model.line_arena.get(i.0)),
            CurveImpl::Circle(i) => CurveKind::Circle(model.circle_arena.get(i.0)),
            CurveImpl::Ellipse(i) => CurveKind::Ellipse(model.ellipse_arena.get(i.0)),
            CurveImpl::BSplineCurve(i) => CurveKind::BSpline(model.b_spline_curve_arena.get(i.0)),
            CurveImpl::BSplineCurveWithKnots(i) => {
                CurveKind::BSplineWithKnots(model.b_spline_curve_with_knots_arena.get(i.0))
            }
            CurveImpl::TrimmedCurve(i) => CurveKind::Trimmed(model.trimmed_curve_arena.get(i.0)),
            CurveImpl::BezierCurve(i) => CurveKind::Bezier(model.bezier_curve_arena.get(i.0)),
            CurveImpl::BoundedCurve(_) => CurveKind::Other("BOUNDED_CURVE"),
            CurveImpl::BoundedPcurve(_) => CurveKind::Other("BOUNDED_PCURVE"),
            CurveImpl::BoundedSurfaceCurve(i) => {
                CurveKind::BoundedSurfaceCurve(model.bounded_surface_curve_arena.get(i.0))
            }
            CurveImpl::CompositeCurve(i) => {
                CurveKind::Composite(model.composite_curve_arena.get(i.0))
            }
            CurveImpl::Conic(_) => CurveKind::Other("CONIC"),
            CurveImpl::Curve(_) => CurveKind::Other("CURVE"),
            CurveImpl::Hyperbola(_) => CurveKind::Other("HYPERBOLA"),
            CurveImpl::IntersectionCurve(i) => {
                CurveKind::IntersectionCurve(model.intersection_curve_arena.get(i.0))
            }
            CurveImpl::Pcurve(_) => CurveKind::Other("PCURVE"),
            CurveImpl::Polyline(i) => CurveKind::Polyline(model.polyline_arena.get(i.0)),
            CurveImpl::QuasiUniformCurve(i) => {
                CurveKind::QuasiUniform(model.quasi_uniform_curve_arena.get(i.0))
            }
            CurveImpl::RationalBSplineCurve(_) => CurveKind::Other("RATIONAL_B_SPLINE_CURVE"),
            CurveImpl::SeamCurve(i) => CurveKind::SeamCurve(model.seam_curve_arena.get(i.0)),
            CurveImpl::SurfaceCurve(i) => {
                CurveKind::SurfaceCurve(model.surface_curve_arena.get(i.0))
            }
            CurveImpl::UniformCurve(i) => CurveKind::Uniform(model.uniform_curve_arena.get(i.0)),
            CurveImpl::Complex(_) => CurveKind::Other("COMPLEX"),
        }
    }
}

// ---------------------------------------------------------------------------
// Vertex / Point
// ---------------------------------------------------------------------------

/// A topological vertex (`VERTEX_POINT`).
#[derive(Clone, Copy)]
pub struct Vertex<'m> {
    cx: Ctx<'m>,
    id: m::VertexPointId,
}

impl<'m> Vertex<'m> {
    fn from_ref(cx: Ctx<'m>, r: &m::VertexRef) -> Option<Self> {
        match r {
            m::VertexRef::VertexPoint(i) => Some(Vertex { cx, id: *i }),
            m::VertexRef::Vertex(_) | m::VertexRef::Complex(_) => None,
        }
    }

    /// This vertex's global identity (a `Copy` key for maps / deduplication).
    pub fn key(&self) -> m::EntityKey {
        m::EntityKey::VertexPoint(self.id)
    }

    /// The vertex's geometric point (only `CARTESIAN_POINT` is resolved here).
    pub fn point(&self) -> Option<Point<'m>> {
        let r = &self
            .cx
            .model
            .vertex_point_arena
            .get(self.id.0)
            .vertex_geometry;
        match r {
            m::PointRef::CartesianPoint(i) => Some(Point {
                cx: self.cx,
                id: *i,
            }),
            m::PointRef::ApllPoint(_)
            | m::PointRef::ApllPointWithSurface(_)
            | m::PointRef::Point(_)
            | m::PointRef::Complex(_) => None,
        }
    }

    /// The edges that begin or end at this vertex (reverse: an `EDGE_CURVE` whose
    /// `edge_start` or `edge_end` is this vertex). A closed edge whose start and
    /// end are this same vertex is yielded once.
    pub fn edges(&self) -> impl Iterator<Item = Edge<'m>> + 'm {
        let cx = self.cx;
        let rg = cx.ref_graph();
        let mut out: Vec<Edge<'m>> = Vec::new();
        for &r in rg.referrers(m::EntityKey::VertexPoint(self.id)) {
            if let m::EntityKey::EdgeCurve(eid) = r {
                if !out.iter().any(|e| e.id == eid) {
                    out.push(Edge { cx, id: eid });
                }
            }
        }
        out.into_iter()
    }
}

/// A geometric point (`CARTESIAN_POINT`).
#[derive(Clone, Copy)]
pub struct Point<'m> {
    cx: Ctx<'m>,
    id: m::CartesianPointId,
}

impl<'m> Point<'m> {
    /// Construct a point handle from a `CARTESIAN_POINT` id — used by other scene
    /// modules (e.g. `pmi` feature geometry) to bridge into geometry.
    pub(crate) fn from_id(cx: Ctx<'m>, id: m::CartesianPointId) -> Self {
        Point { cx, id }
    }

    /// This point's global identity (a `Copy` key for maps / deduplication).
    pub fn key(&self) -> m::EntityKey {
        m::EntityKey::CartesianPoint(self.id)
    }

    /// The point's coordinates (`x, y[, z]`).
    /// The coordinates as a 3-vector, missing components padded with `0.0` —
    /// the form every consumer wants (see [`Point::coords`] for the raw slice).
    pub fn xyz(&self) -> [f64; 3] {
        let c = self.coords();
        [
            c.first().copied().unwrap_or(0.0),
            c.get(1).copied().unwrap_or(0.0),
            c.get(2).copied().unwrap_or(0.0),
        ]
    }

    pub fn coords(&self) -> &'m [f64] {
        &self
            .cx
            .model
            .cartesian_point_arena
            .get(self.id.0)
            .coordinates
    }
}

// ---------------------------------------------------------------------------
// Reference resolvers (hand-written for the spike)
// ---------------------------------------------------------------------------

/// The solid whose shell is this closed shell — a `MANIFOLD_SOLID_BREP` or a
/// `BREP_WITH_VOIDS`, directly or through an `ORIENTED_CLOSED_SHELL` wrapper
/// (the wrapper is how a `BREP_WITH_VOIDS` cavity references its shell). Reverse
/// over [`RefGraph`].
fn closed_shell_to_solid(rg: &RefGraph, cs: m::ClosedShellId) -> Option<SolidImpl> {
    for &r in rg.referrers(m::EntityKey::ClosedShell(cs)) {
        match r {
            m::EntityKey::ManifoldSolidBrep(msb) => return Some(SolidImpl::Manifold(msb)),
            m::EntityKey::BrepWithVoids(bwv) => return Some(SolidImpl::WithVoids(bwv)),
            m::EntityKey::OrientedClosedShell(ocs) => {
                for &r2 in rg.referrers(m::EntityKey::OrientedClosedShell(ocs)) {
                    match r2 {
                        m::EntityKey::ManifoldSolidBrep(msb) => {
                            return Some(SolidImpl::Manifold(msb));
                        }
                        m::EntityKey::BrepWithVoids(bwv) => return Some(SolidImpl::WithVoids(bwv)),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Resolve a `ClosedShellRef` to the underlying closed shell, following an
/// `OrientedClosedShell` indirection. `Complex` shells are not resolved.
fn resolve_closed_shell<'m>(
    model: &'m StepModel,
    r: &m::ClosedShellRef,
) -> Option<&'m m::ClosedShell> {
    match r {
        m::ClosedShellRef::ClosedShell(i) => Some(model.closed_shell_arena.get(i.0)),
        m::ClosedShellRef::OrientedClosedShell(i) => {
            let oc = model.oriented_closed_shell_arena.get(i.0);
            resolve_closed_shell(model, &oc.closed_shell_element)
        }
        m::ClosedShellRef::Complex(_) => None,
    }
}

/// Resolve a `LoopRef` to an `EdgeLoop` (only edge loops carry edges).
fn resolve_edge_loop<'m>(model: &'m StepModel, r: &m::LoopRef) -> Option<&'m m::EdgeLoop> {
    match r {
        m::LoopRef::EdgeLoop(i) => Some(model.edge_loop_arena.get(i.0)),
        m::LoopRef::Loop(_)
        | m::LoopRef::PolyLoop(_)
        | m::LoopRef::VertexLoop(_)
        | m::LoopRef::Complex(_) => None,
    }
}

/// Resolve an `OrientedEdgeRef` to an `OrientedEdge`.
fn resolve_oriented_edge<'m>(
    model: &'m StepModel,
    r: &m::OrientedEdgeRef,
) -> Option<&'m m::OrientedEdge> {
    match r {
        m::OrientedEdgeRef::OrientedEdge(i) => Some(model.oriented_edge_arena.get(i.0)),
        m::OrientedEdgeRef::Complex(_) => None,
    }
}
