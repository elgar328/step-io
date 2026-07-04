//! [`Scene`] — the convenience navigation layer over the raw [`StepModel`].
//!
//! The generated model is faithful but tedious to walk by hand (arenas and
//! reference enums); [`model.scene()`](StepModel::scene) wraps it in
//! lightweight `Copy` handles where forward *and* backward moves are plain
//! `handle.method()` calls. The backward index ([`RefGraph`]) is built
//! lazily on the first backward move — a forward-only import never pays
//! for it.
//!
//! The core is enumeration plus scoped navigation. Model-level
//! `all_<type>()` iterators ([`all_solids`](Scene::all_solids),
//! [`all_products`](Scene::all_products), …) enumerate one type across the
//! whole file; each handle then navigates its own neighbourhood
//! (`solid.faces()`, `face.surface()`, `occurrence.definition()`, …).
//! Coverage spans b-rep geometry ([`geometry`]), the product/assembly tree
//! and PLM metadata ([`product`]), display meshes ([`mesh`]), PMI ([`pmi`]),
//! presentation ([`presentation`]), units ([`units`]), and the NURBS views
//! ([`nurbs`]).
//!
//! Kinds outside the handled set come back as `*Kind::Other`; what a handle
//! cannot resolve into the shape it exposes lands in [`Scene::warnings`],
//! never silently skipped.
//!
//! Anything beyond the handles is reached through the raw model via
//! [`Scene::model`].
//!
//! ```no_run
//! let source = std::fs::read("part.step").unwrap();
//! let (model, _report) = step_io::read(&source).unwrap();
//! let scene = model.scene();
//!
//! // Assembly: definitions and their placed occurrences, top-down. Keep the
//! // placement on the instance — do not bake it into shared geometry.
//! for def in scene.root_definitions() {
//!     for occ in def.occurrences() {
//!         let _child = occ.definition(); // recurse from here
//!         let _matrix = occ.transform().and_then(|t| t.matrix());
//!     }
//! }
//!
//! // Geometry: solid → face → surface, bound → edge → curve → vertex.
//! // Presentation (colour / layer / visibility) rides on the same handles.
//! for solid in scene.all_solids() {
//!     let _colour = solid.color();
//!     for face in solid.faces() {
//!         let _surface = face.surface().kind(); // analytic form; `Other` beyond the set
//!         let _patch = face.to_nurbs(); // the NURBS view, on demand
//!         for bound in face.bounds() {
//!             for (edge, _forward) in bound.oriented_edges() {
//!                 let _curve = edge.curve().kind();
//!                 let _ends = (edge.start(), edge.end());
//!             }
//!         }
//!     }
//! }
//!
//! // Display meshes, linked back to the b-rep solid they render.
//! for group in scene.all_mesh_groups() {
//!     let _brep = group.solid();
//!     for mesh in group.meshes() {
//!         let (_pts, _tris) = (mesh.points(), mesh.triangles());
//!     }
//! }
//!
//! // PMI: dimensions, tolerances, and the annotated features.
//! for dim in scene.dimensions() {
//!     let (_kind, _value) = (dim.kind(), dim.value());
//! }
//! for tol in scene.tolerances() {
//!     let (_kind, _datums) = (tol.kind(), tol.datums());
//! }
//! for feat in scene.features() {
//!     let _geometry = feat.geometry(); // the faces/edges it annotates
//! }
//!
//! // Product metadata: versions, approvals, documents, people.
//! for product in scene.all_products() {
//!     let _versions = product.versions();
//!     let _approvals = product.approvals();
//!     let _documents = product.documents();
//!     let _people = product.contributors();
//! }
//!
//! // Coordinates come in the file's units — scaling is the consumer's job.
//! let _to_si = scene.units().length.map(|u| u.to_si);
//! for w in scene.warnings() {
//!     eprintln!("step-io: {w}");
//! }
//! ```

pub mod geometry;
pub mod mesh;
pub mod nurbs;
pub mod pmi;
pub mod presentation;
pub mod product;
pub mod units;

use std::sync::{Mutex, OnceLock};

use crate::RefGraph;
use crate::StepModel;

pub use mesh::{Mesh, MeshGroup, MeshNormals};
pub use nurbs::{NurbsCurve, NurbsSurface};
pub use pmi::{
    Datum, Dimension, DimensionKind, Feature, FeatureGeometry, FeatureKind, Tolerance,
    ToleranceKind,
};
pub use presentation::Rgb;
pub use product::{
    Approval, ApprovalDate, Approver, Contributor, Document, MappedInstance, Occurrence, Person,
    Placement, Scope, SecurityClassification, Target, Transform, Version,
};
pub use units::{Unit, Units};

/// A `Copy` reference bundle handed to every handle: the model, the lazy reverse
/// index, and the scene's warning log. All borrows share one lifetime (the scene
/// borrow), so handles stay single-lifetime.
#[derive(Clone, Copy)]
pub(crate) struct Ctx<'a> {
    pub(crate) model: &'a StepModel,
    pub(crate) rg: &'a OnceLock<RefGraph>,
    pub(crate) warnings: &'a Mutex<Vec<String>>,
}

impl<'a> Ctx<'a> {
    /// The reverse-reference index, built on first use.
    pub(crate) fn ref_graph(&self) -> &'a RefGraph {
        self.rg.get_or_init(|| self.model.ref_graph())
    }

    /// Record a navigation anomaly (something a handle could not resolve into
    /// the shape it exposes) so it is surfaced rather than silently dropped.
    pub(crate) fn warn(&self, msg: String) {
        self.warnings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(msg);
    }
}

/// The read API entry point: a [`StepModel`] paired with its (lazy) [`RefGraph`]
/// and a log of navigation warnings.
pub struct Scene<'m> {
    model: &'m StepModel,
    rg: OnceLock<RefGraph>,
    warnings: Mutex<Vec<String>>,
}

impl StepModel {
    /// Build a [`Scene`] over this model — the entry point for the navigation
    /// handles. The reverse-reference index is built lazily on first use; reuse
    /// the returned scene for all navigation.
    #[must_use]
    pub fn scene(&self) -> Scene<'_> {
        Scene {
            model: self,
            rg: OnceLock::new(),
            warnings: Mutex::new(Vec::new()),
        }
    }
}

impl<'m> Scene<'m> {
    /// The reference bundle to hand to a handle created from this scene.
    pub(crate) fn ctx(&self) -> Ctx<'_> {
        Ctx {
            model: self.model,
            rg: &self.rg,
            warnings: &self.warnings,
        }
    }

    /// The underlying raw model (escape hatch for uncovered long-tail entities).
    #[must_use]
    pub fn model(&self) -> &'m StepModel {
        self.model
    }

    /// Navigation anomalies recorded so far — cases a handle could not resolve
    /// into the shape it exposes (e.g. an occurrence transform whose item is not
    /// an `AXIS2_PLACEMENT_3D`). Surfaced here rather than silently dropped.
    #[must_use]
    pub fn warnings(&self) -> Vec<String> {
        self.warnings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}
