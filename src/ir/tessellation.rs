//! Tessellation IR types — `COORDINATES_LIST` + `COMPLEX_TRIANGULATED_FACE`.
//!
//! The first entry into STEP's tessellated-geometry domain (AP242). Both
//! sit in the `topology` pool per the ir.toml blueprint but form their own
//! domain; later tessellation entities (tessellated solid / shell /
//! surface-set, ...) extend this module.

use super::id::{
    Placement3dId, ShellId, SolidId, TessellatedFaceId, TessellatedItemId, TessellatedSurfaceSetId,
};
use super::representation_item::RepresentationItemRef;

/// `tessellated_item` `enum_base`. All AP242 tessellated-geometry subtypes
/// step-io models are variants here.
#[derive(Debug, Clone, PartialEq)]
pub enum TessellatedItem {
    CoordinatesList(CoordinatesList),
    TessellatedCurveSet(TessellatedCurveSet),
    TessellatedGeometricSet(TessellatedGeometricSet),
    TessellatedSolid(TessellatedSolid),
    TessellatedShell(TessellatedShell),
    /// `REPOSITIONED_TESSELLATED_ITEM` — wraps another tessellated item
    /// indirectly through a per-instance `AXIS2_PLACEMENT_3D` reference
    /// frame. Phase rti.
    RepositionedTessellatedItem(RepositionedTessellatedItem),
}

/// `REPOSITIONED_TESSELLATED_ITEM(name, location)` — a `tessellated_item`
/// SUBTYPE that supplies a local coordinate frame for downstream
/// tessellation consumers. EXPRESS WHERE rule forbids co-instantiation
/// with the seven other concrete `tessellated_item` subtypes (`curve_set`,
/// `geometric_set`, `point_set`, `surface_set`, `shell`, `solid`, `wire`);
/// the corpus carries every instance as a complex-MI part — same dispatch
/// pattern as `TESSELLATED_GEOMETRIC_SET`.
#[derive(Debug, Clone, PartialEq)]
pub struct RepositionedTessellatedItem {
    pub name: String,
    pub location: Placement3dId,
}

/// Unified reference to a STEP `tessellated_item` — an abstract supertype
/// step-io splits across three arenas. Each variant wraps the id of an
/// existing tessellation arena entry. New variants are added additively as
/// consumers require them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TessellatedItemRef {
    /// `tessellated_items` arena (`COORDINATES_LIST` / `TESSELLATED_CURVE_SET`
    /// / `TESSELLATED_GEOMETRIC_SET`).
    Item(TessellatedItemId),
    /// `tessellated_faces` arena (`COMPLEX_TRIANGULATED_FACE`).
    Face(TessellatedFaceId),
    /// `tessellated_surface_sets` arena (`COMPLEX_TRIANGULATED_SURFACE_SET`).
    SurfaceSet(TessellatedSurfaceSetId),
}

/// `TESSELLATED_GEOMETRIC_SET(name, children)` — an aggregate of
/// tessellated items. A `tessellated_item` enum member. Children that do
/// not resolve are dropped from the set, symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct TessellatedGeometricSet {
    pub name: String,
    /// `set_ref_tessellated_item`.
    pub children: Vec<TessellatedItemRef>,
}

/// `TESSELLATED_SOLID(name, items, geometric_link)` — a tessellated solid
/// volume. `items` are tessellated structured items (faces / surface sets);
/// `geometric_link` optionally points at the exact `MANIFOLD_SOLID_BREP`.
#[derive(Debug, Clone, PartialEq)]
pub struct TessellatedSolid {
    pub name: String,
    /// `set_ref_tessellated_structured_item`.
    pub items: Vec<TessellatedItemRef>,
    /// `opt_ref_manifold_solid_brep` — resolved to a solid.
    pub geometric_link: Option<SolidId>,
}

/// `TESSELLATED_SHELL(name, items, topological_link)` — a tessellated
/// shell. `topological_link` optionally points at the exact
/// `CONNECTED_FACE_SET`.
#[derive(Debug, Clone, PartialEq)]
pub struct TessellatedShell {
    pub name: String,
    /// `set_ref_tessellated_structured_item`.
    pub items: Vec<TessellatedItemRef>,
    /// `opt_ref_connected_face_set` — resolved to a shell.
    pub topological_link: Option<ShellId>,
}

/// `COORDINATES_LIST(name, npoints, position_coords)` — a shared pool of
/// 3D point coordinates referenced by tessellated faces.
#[derive(Debug, Clone, PartialEq)]
pub struct CoordinatesList {
    pub name: String,
    pub npoints: i64,
    /// `N×3` grid of point coordinates.
    pub position_coords: Vec<Vec<f64>>,
}

/// `TESSELLATED_CURVE_SET(name, coordinates, line_strips)` — a set of
/// polylines referencing a [`CoordinatesList`] for its points. A
/// `tessellated_item` enum member.
#[derive(Debug, Clone, PartialEq)]
pub struct TessellatedCurveSet {
    pub name: String,
    /// `ref_coordinates_list` — resolved to the `tessellated_item` arena.
    pub coordinates: TessellatedItemId,
    /// Ragged — line strips vary in length.
    pub line_strips: Vec<Vec<i64>>,
}

/// `COMPLEX_TRIANGULATED_SURFACE_SET(name, coordinates, pnmax, normals,
/// pnindex, triangle_strips, triangle_fans)` — a triangulated surface set
/// referencing a [`CoordinatesList`] for its points. Identical to
/// [`ComplexTriangulatedFace`] minus the optional `geometric_link`.
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexTriangulatedSurfaceSet {
    pub name: String,
    /// `ref_coordinates_list` — resolved to the `tessellated_item` arena.
    pub coordinates: TessellatedItemId,
    pub pnmax: i64,
    /// `M×3` grid of normal vectors.
    pub normals: Vec<Vec<f64>>,
    pub pnindex: Vec<i64>,
    /// Ragged — triangle strips vary in length.
    pub triangle_strips: Vec<Vec<i64>>,
    /// Ragged — triangle fans vary in length.
    pub triangle_fans: Vec<Vec<i64>>,
}

/// `COMPLEX_TRIANGULATED_FACE(name, coordinates, pnmax, normals,
/// geometric_link, pnindex, triangle_strips, triangle_fans)` — a
/// triangulated face referencing a [`CoordinatesList`] for its points.
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexTriangulatedFace {
    pub name: String,
    /// `ref_coordinates_list` — resolved to the `tessellated_item` arena.
    pub coordinates: TessellatedItemId,
    pub pnmax: i64,
    /// `M×3` grid of normal vectors.
    pub normals: Vec<Vec<f64>>,
    /// Optional link to the exact geometry the tessellation approximates.
    pub geometric_link: Option<RepresentationItemRef>,
    pub pnindex: Vec<i64>,
    /// Ragged — triangle strips vary in length.
    pub triangle_strips: Vec<Vec<i64>>,
    /// Ragged — triangle fans vary in length.
    pub triangle_fans: Vec<Vec<i64>>,
}
