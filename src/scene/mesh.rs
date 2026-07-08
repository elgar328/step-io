//! Read-only handles over tessellated (display-mesh) geometry.
//!
//! AP242 tessellated items carry a pre-triangulated copy of the exact b-rep
//! for direct display — in some exports they are the *only* real shape. The
//! [`Mesh`] handle exposes the data a viewer consumes — vertex positions,
//! expanded triangles and normals — without touching the exact geometry;
//! [`Mesh::face`] links back to the precise b-rep face where the file
//! provides it. (This is a visualization mesh, not an analysis/FEM mesh.)

// Every public method here is a pure read accessor (see geometry.rs).
#![allow(clippy::must_use_candidate)]

use crate::generated::model as m;
use crate::scene::geometry::{Face, Solid};
use crate::scene::{Ctx, Scene};

/// Model-wide mesh enumeration.
impl Scene<'_> {
    /// Every mesh group in the model (`TESSELLATED_SOLID` and
    /// `TESSELLATED_SHELL` — one part's bundle of mesh faces, with a link to
    /// the precise b-rep solid where the file provides one).
    pub fn all_mesh_groups(&self) -> impl Iterator<Item = MeshGroup<'_>> + '_ {
        let cx = self.ctx();
        let solids = (0..cx.model.tessellated_solid_arena.items.len()).map(move |i| MeshGroup {
            cx,
            which: GroupImpl::Solid(m::TessellatedSolidId(i)),
        });
        let shells = (0..cx.model.tessellated_shell_arena.items.len()).map(move |i| MeshGroup {
            cx,
            which: GroupImpl::Shell(m::TessellatedShellId(i)),
        });
        solids.chain(shells)
    }

    /// Every tessellated mesh item in the model (`COMPLEX_TRIANGULATED_FACE`,
    /// `COMPLEX_TRIANGULATED_SURFACE_SET` and plain `TESSELLATED_FACE`).
    pub fn all_meshes(&self) -> impl Iterator<Item = Mesh<'_>> + '_ {
        let cx = self.ctx();
        let faces = (0..cx.model.complex_triangulated_face_arena.items.len()).map(move |i| Mesh {
            cx,
            which: MeshImpl::ComplexFace(m::ComplexTriangulatedFaceId(i)),
        });
        let sets =
            (0..cx.model.complex_triangulated_surface_set_arena.items.len()).map(move |i| Mesh {
                cx,
                which: MeshImpl::ComplexSurfaceSet(m::ComplexTriangulatedSurfaceSetId(i)),
            });
        let plain = (0..cx.model.tessellated_face_arena.items.len()).map(move |i| Mesh {
            cx,
            which: MeshImpl::PlainFace(m::TessellatedFaceId(i)),
        });
        faces.chain(sets).chain(plain)
    }
}

#[derive(Clone, Copy)]
enum MeshImpl {
    ComplexFace(m::ComplexTriangulatedFaceId),
    ComplexSurfaceSet(m::ComplexTriangulatedSurfaceSetId),
    PlainFace(m::TessellatedFaceId),
}

/// One tessellated mesh item: a vertex list plus triangles (expanded from the
/// stored triangle strips/fans).
#[derive(Clone, Copy)]
pub struct Mesh<'m> {
    cx: Ctx<'m>,
    which: MeshImpl,
}

/// The normals of a [`Mesh`], as the file stores them: absent, one normal for
/// the whole mesh, or one per vertex (parallel to [`Mesh::points`]).
#[derive(Clone, Debug, PartialEq)]
pub enum MeshNormals {
    None,
    Uniform([f64; 3]),
    PerVertex(Vec<[f64; 3]>),
}

/// (`name`, `coordinates`, `normals`, `pnindex`, `strips`, `fans`) — the
/// shared layout; the plain face has no index/triangle data.
type MeshParts<'m> = (
    &'m str,
    &'m m::CoordinatesListRef,
    &'m [Vec<f64>],
    &'m [i64],
    &'m [Vec<i64>],
    &'m [Vec<i64>],
);

fn row3(row: &[f64]) -> [f64; 3] {
    [
        row.first().copied().unwrap_or(0.0),
        row.get(1).copied().unwrap_or(0.0),
        row.get(2).copied().unwrap_or(0.0),
    ]
}

impl<'m> Mesh<'m> {
    fn parts(&self) -> MeshParts<'m> {
        match self.which {
            MeshImpl::ComplexFace(i) => {
                let f = self.cx.model.complex_triangulated_face_arena.get(i.0);
                (
                    &f.name,
                    &f.coordinates,
                    &f.normals,
                    &f.pnindex,
                    &f.triangle_strips,
                    &f.triangle_fans,
                )
            }
            MeshImpl::ComplexSurfaceSet(i) => {
                let s = self
                    .cx
                    .model
                    .complex_triangulated_surface_set_arena
                    .get(i.0);
                (
                    &s.name,
                    &s.coordinates,
                    &s.normals,
                    &s.pnindex,
                    &s.triangle_strips,
                    &s.triangle_fans,
                )
            }
            MeshImpl::PlainFace(i) => {
                let f = self.cx.model.tessellated_face_arena.get(i.0);
                (&f.name, &f.coordinates, &f.normals, &[], &[], &[])
            }
        }
    }

    pub fn name(&self) -> &'m str {
        self.parts().0
    }

    /// This mesh's global identity (a `Copy` key for maps / deduplication).
    pub fn key(&self) -> m::EntityKey {
        match self.which {
            MeshImpl::ComplexFace(i) => m::EntityKey::ComplexTriangulatedFace(i),
            MeshImpl::ComplexSurfaceSet(i) => m::EntityKey::ComplexTriangulatedSurfaceSet(i),
            MeshImpl::PlainFace(i) => m::EntityKey::TessellatedFace(i),
        }
    }

    /// The vertex positions, `pnindex` resolved: entry `n` is the position of
    /// the vertex that [`Mesh::triangles`] refers to as `n` (0-based).
    pub fn points(&self) -> Vec<[f64; 3]> {
        let (_, coords, _, pnindex, ..) = self.parts();
        let m::CoordinatesListRef::CoordinatesList(id) = coords;
        let all = &self
            .cx
            .model
            .coordinates_list_arena
            .get(id.0)
            .position_coords;
        if pnindex.is_empty() {
            all.iter().map(|row| row3(row)).collect()
        } else {
            pnindex
                .iter()
                .filter_map(|&i| {
                    let idx = usize::try_from(i).ok()?.checked_sub(1)?;
                    all.get(idx).map(|row| row3(row))
                })
                .collect()
        }
    }

    /// The triangles as 0-based indices into [`Mesh::points`], expanded from
    /// the stored triangle strips and fans (the same expansion OCCT applies;
    /// only the emission order differs). Degenerate entries (repeated
    /// indices) are skipped per the encoding; an out-of-range index skips the
    /// triangle with a warning. A plain `TESSELLATED_FACE` stores no
    /// triangles, so its list is empty.
    pub fn triangles(&self) -> Vec<[usize; 3]> {
        let (_, _, _, _, strips, fans) = self.parts();
        let n = self.points().len();
        let cx = self.cx;
        let mut out = Vec::new();
        let mut push = |a: i64, b: i64, c: i64| {
            let tri: Option<Vec<usize>> = [a, b, c]
                .iter()
                .map(|&v| {
                    usize::try_from(v)
                        .ok()
                        .and_then(|v| v.checked_sub(1))
                        .filter(|&v| v < n)
                })
                .collect();
            if let Some(t) = tri {
                out.push([t[0], t[1], t[2]]);
            } else {
                cx.warn("MESH.triangles: vertex index out of range".to_owned());
            }
        };
        for strip in strips {
            for i in 2..strip.len() {
                let (a, b, c) = (strip[i - 2], strip[i - 1], strip[i]);
                if c == a || c == b {
                    continue; // a degenerate restart entry
                }
                // Strips alternate winding so every triangle faces the same way.
                if i % 2 == 0 {
                    push(a, c, b);
                } else {
                    push(a, b, c);
                }
            }
        }
        for fan in fans {
            for i in 2..fan.len() {
                let (centre, prev, cur) = (fan[0], fan[i - 1], fan[i]);
                if cur == fan[i - 2] || prev == fan[i - 2] {
                    continue; // a degenerate restart entry
                }
                push(centre, cur, prev);
            }
        }
        out
    }

    /// The normals, as stored: none, one for the whole mesh, or one per
    /// vertex. Any other count is malformed and reported as a warning.
    pub fn normals(&self) -> MeshNormals {
        let (_, _, normals, ..) = self.parts();
        match normals.len() {
            0 => MeshNormals::None,
            1 => MeshNormals::Uniform(row3(&normals[0])),
            k if k == self.points().len() => {
                MeshNormals::PerVertex(normals.iter().map(|row| row3(row)).collect())
            }
            _ => {
                self.cx
                    .warn("MESH.normals: count is neither 1 nor per-vertex".to_owned());
                MeshNormals::None
            }
        }
    }

    /// The precise b-rep face this mesh tessellates, where the file links one
    /// (`geometric_link`); surface links and surface sets have no face.
    pub fn face(&self) -> Option<Face<'m>> {
        let link = match self.which {
            MeshImpl::ComplexFace(i) => self
                .cx
                .model
                .complex_triangulated_face_arena
                .get(i.0)
                .geometric_link
                .as_ref(),
            MeshImpl::PlainFace(i) => self
                .cx
                .model
                .tessellated_face_arena
                .get(i.0)
                .geometric_link
                .as_ref(),
            MeshImpl::ComplexSurfaceSet(_) => None,
        }?;
        match link {
            m::FaceOrSurfaceRef::AdvancedFace(id) => Some(Face::from_advanced_face(self.cx, *id)),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// MeshGroup (TESSELLATED_SOLID / TESSELLATED_SHELL containers)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum GroupImpl {
    Solid(m::TessellatedSolidId),
    Shell(m::TessellatedShellId),
}

/// One part's bundle of mesh faces — a `TESSELLATED_SOLID` or
/// `TESSELLATED_SHELL` (parallel containers; a shell is not nested in a
/// solid). [`MeshGroup::solid`] reaches the precise b-rep part.
#[derive(Clone, Copy)]
pub struct MeshGroup<'m> {
    cx: Ctx<'m>,
    which: GroupImpl,
}

impl<'m> MeshGroup<'m> {
    fn items(&self) -> &'m [m::TessellatedStructuredItemRef] {
        match self.which {
            GroupImpl::Solid(i) => &self.cx.model.tessellated_solid_arena.get(i.0).items,
            GroupImpl::Shell(i) => &self.cx.model.tessellated_shell_arena.get(i.0).items,
        }
    }

    pub fn name(&self) -> &'m str {
        match self.which {
            GroupImpl::Solid(i) => &self.cx.model.tessellated_solid_arena.get(i.0).name,
            GroupImpl::Shell(i) => &self.cx.model.tessellated_shell_arena.get(i.0).name,
        }
    }

    /// This group's global identity (a `Copy` key for maps / deduplication).
    pub fn key(&self) -> m::EntityKey {
        match self.which {
            GroupImpl::Solid(i) => m::EntityKey::TessellatedSolid(i),
            GroupImpl::Shell(i) => m::EntityKey::TessellatedShell(i),
        }
    }

    /// The mesh faces bundled in this group.
    pub fn meshes(&self) -> Vec<Mesh<'m>> {
        let cx = self.cx;
        self.items()
            .iter()
            .filter_map(|it| {
                let which = match it {
                    m::TessellatedStructuredItemRef::ComplexTriangulatedFace(id) => {
                        MeshImpl::ComplexFace(*id)
                    }
                    m::TessellatedStructuredItemRef::TessellatedFace(id) => {
                        MeshImpl::PlainFace(*id)
                    }
                    _ => return None,
                };
                Some(Mesh { cx, which })
            })
            .collect()
    }

    /// The precise b-rep solid this group tessellates, where the file links
    /// one: a tessellated solid links it directly (`geometric_link`); a
    /// tessellated shell links the closed shell (`topological_link`), reached
    /// back through the solid that owns it.
    pub fn solid(&self) -> Option<Solid<'m>> {
        let cx = self.cx;
        match self.which {
            GroupImpl::Solid(i) => {
                let link = cx
                    .model
                    .tessellated_solid_arena
                    .get(i.0)
                    .geometric_link
                    .as_ref()?;
                match link {
                    m::ManifoldSolidBrepRef::ManifoldSolidBrep(id) => Some(Solid::from_id(cx, *id)),
                    m::ManifoldSolidBrepRef::BrepWithVoids(id) => {
                        Some(Solid::from_void_id(cx, *id))
                    }
                    m::ManifoldSolidBrepRef::Complex(_) => None,
                }
            }
            GroupImpl::Shell(i) => {
                let link = cx
                    .model
                    .tessellated_shell_arena
                    .get(i.0)
                    .topological_link
                    .as_ref()?;
                let m::ConnectedFaceSetRef::ClosedShell(shell_id) = link else {
                    return None;
                };
                let rg = cx.ref_graph();
                let outer_is = |outer: &m::ClosedShellRef| matches!(outer, m::ClosedShellRef::ClosedShell(s) if s == shell_id);
                for r in rg.referrers(m::EntityKey::ClosedShell(*shell_id)) {
                    match r {
                        m::EntityKey::ManifoldSolidBrep(sid) => {
                            let brep = cx.model.manifold_solid_brep_arena.get(sid.0);
                            if outer_is(&brep.outer) {
                                return Some(Solid::from_id(cx, *sid));
                            }
                        }
                        m::EntityKey::BrepWithVoids(sid) => {
                            let brep = cx.model.brep_with_voids_arena.get(sid.0);
                            if outer_is(&brep.outer) {
                                return Some(Solid::from_void_id(cx, *sid));
                            }
                        }
                        _ => {}
                    }
                }
                None
            }
        }
    }
}

impl<'m> Mesh<'m> {
    /// The group (part bundle) containing this mesh, if any — the first
    /// `TESSELLATED_SOLID` / `TESSELLATED_SHELL` whose items include it.
    pub fn group(&self) -> Option<MeshGroup<'m>> {
        let cx = self.cx;
        let rg = cx.ref_graph();
        for r in rg.referrers(self.key()) {
            let which = match r {
                m::EntityKey::TessellatedSolid(id) => GroupImpl::Solid(*id),
                m::EntityKey::TessellatedShell(id) => GroupImpl::Shell(*id),
                _ => continue,
            };
            return Some(MeshGroup { cx, which });
        }
        None
    }
}
