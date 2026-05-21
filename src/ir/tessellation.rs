//! Tessellation IR types — `COORDINATES_LIST` + `COMPLEX_TRIANGULATED_FACE`.
//!
//! The first entry into STEP's tessellated-geometry domain (AP242). Both
//! sit in the `topology` pool per the ir.toml blueprint but form their own
//! domain; later tessellation entities (tessellated solid / shell /
//! surface-set, ...) extend this module.

use super::id::TessellatedItemId;
use super::representation_item::RepresentationItemRef;

/// `tessellated_item` `enum_base`. Only `CoordinatesList` is modelled; the
/// tessellated solid / shell / surface-set siblings are deferred.
#[derive(Debug, Clone, PartialEq)]
pub enum TessellatedItem {
    CoordinatesList(CoordinatesList),
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
