//! Lean STEP (ISO 10303) I/O for 3D CAD applications — reads every mainstream AP,
//! writes only modern AP242.
//!
//! > ⚠️ **Experimental** — early stage; expect breaking API changes at any time.
//!
//! - **Reads everything, writes AP242** — any mainstream AP comes in, legacy
//!   included; only AP242 edition 2 (IS) goes out.
//! - **Lean** — one universal schema model instead of a per-schema
//!   implementation; the supported entity set is curated to what real STEP
//!   files actually use.
//! - **Heals non-standard input** — what can be fixed is normalized; what
//!   cannot is dropped with a reason, never silently.
//! - **Ergonomic API** — [`scene()`](StepModel::scene) navigates what you read
//!   with lightweight handles; [`StepBuilder`] writes new files element by
//!   element, the way CAD data is held.
//!
//! # Design
//!
//! step-io sits between 3D CAD applications and the STEP files they
//! exchange.
//!
//! Most STEP libraries carry per-schema entity and read/write code, so each
//! supported schema adds source and binary size. step-io instead merges the
//! mainstream APs into one *universal* schema and reads them all through a
//! single pipeline. The supported entity set is curated, guided by what
//! 50,000+ real-world files contain — the [entity coverage
//! matrix](https://github.com/elgar328/step-io/blob/main/docs/entities.md)
//! lists exactly what is read and written.
//!
//! Output is AP242 edition 2 (IS) only — AP203 and AP214 were merged into
//! it in 2014, and every modern tool reads it. When edition 3 reaches IS
//! and takes over, the output moves up with it: one output schema,
//! always the current one.
//!
//! Most of the pipeline is generated from the schemas rather than written
//! by hand — no drift, little to maintain.
//!
//! # Reading STEP
//!
//! Reading gives the result and the report together: [`read`] returns the
//! [`StepModel`] plus a [`Report`] of what was kept, normalized, or dropped
//! and why, and what the parser repaired. Messy files are common in the
//! wild — non-standard content is healed or dropped rather than failing the
//! import, and the report is the record of what happened on the way in.
//!
//! The model is raw and complete, schema-faithful: everything the reader
//! kept is there to read, `model.all::<T>()` listing the entities of a type
//! and `model[id]` reading one (see [`model`]).
//! The Part 21 header is read too, as [`StepModel::header`] — file name,
//! timestamp, authors, the originating CAD system, and the identified
//! schema.
//!
//! [`Scene`](scene::Scene) — `model.scene()` — is the navigation layer on
//! top: lightweight `Copy` handles covering what imports touch most, not
//! every entity. In reach are b-rep geometry (solid → face → surface, edge →
//! curve → point), the assembly tree with placements, display meshes, PMI
//! (dimensions, tolerances, datum features), product metadata (versions,
//! approvals, documents), presentation (colour / layer / visibility), and
//! units — see the [`scene`] module docs. Anything beyond the handles is
//! reached through the raw model via [`Scene::model`](scene::Scene::model).
//!
//! ```no_run
//! use step_io::scene::SurfaceKind;
//!
//! let file = std::fs::File::open("part.step").unwrap();
//! let (model, report) = step_io::read(file).unwrap();
//! // What the parser repaired on the way in; the rest of `report` accounts
//! // for what was kept, normalized, or dropped.
//! for w in &report.parse_warnings {
//!     eprintln!("step-io: {w}");
//! }
//! let scene = model.scene();
//!
//! // Geometry comes in its stored analytic form first; NURBS only on demand.
//! for solid in scene.all_solids() {
//!     for face in solid.faces() {
//!         match face.surface().kind() {
//!             SurfaceKind::Plane(_p) => { /* the kernel's own plane */ }
//!             SurfaceKind::Cylindrical(_c) => { /* the kernel's own cylinder */ }
//!             // … other analytic kinds …
//!             _ => {
//!                 let _patch = face.to_nurbs(); // opt-in NURBS fallback
//!             }
//!         }
//!     }
//! }
//!
//! // Assemblies, meshes, PMI, metadata, presentation: the same handles —
//! // the full tour is in the `scene` module docs.
//!
//! // Navigation anomalies collected during the walk above — not the read-time
//! // `Report`, a second channel. Log and continue, do not fail the import.
//! for w in scene.warnings() {
//!     eprintln!("step-io: {w}");
//! }
//! ```
//!
//! # Writing STEP
//!
//! The write side is strict by construction. Its foundation is [`Ap242Author`],
//! a generated low-level layer with one constructor per AP242 entity (exposed
//! as [`StepBuilder::author`]); every argument is validated at insertion, so
//! the output always conforms to the AP242 edition 2 schema — there is no loss
//! report to check afterwards.
//!
//! [`StepBuilder`] is the convenience layer on top, covering what exports
//! touch most: parts and b-rep topology (vertex → edge → face → solid, over
//! analytic surfaces or NURBS), assembly instances with placements, display
//! meshes, presentation (colour / transparency / layer / visibility), PLM
//! metadata (contributors, approvals, documents, security), units, and the
//! Part 21 header — see the [`build`] module docs. [`StepBuilder::finish`]
//! returns the finished model, and [`write()`] sends it as Part 21 text to a
//! file or any other `std::io::Write`.
//!
//! ```no_run
//! # use step_io::Orientation;
//! # use step_io::build::{CurveInput, FaceBoundInput, Frame, SurfaceInput};
//! # struct KernelEdge { start: usize, end: usize }
//! # struct KernelFace { orientation: Orientation }
//! # struct Kernel { points: Vec<[f64; 3]>, edges: Vec<KernelEdge>, faces: Vec<KernelFace> }
//! # let kernel = Kernel { points: vec![], edges: vec![], faces: vec![] };
//! # fn curve_of(_: &KernelEdge) -> CurveInput { CurveInput::Line }
//! # fn surface_of(_: &KernelFace) -> SurfaceInput {
//! #     SurfaceInput::Plane(Frame { origin: [0.0; 3], axis: [0.0, 0.0, 1.0], ref_dir: [1.0, 0.0, 0.0] })
//! # }
//! # fn loops_of(_: &KernelFace, _es: &[step_io::model::EdgeCurveId]) -> Vec<FaceBoundInput> {
//! #     Vec::new()
//! # }
//! use step_io::StepBuilder;
//! use step_io::build::PartOptions;
//!
//! let mut b = StepBuilder::new().unwrap(); // millimetre; new_with() to choose
//! let wheel = b.part_with("wheel", &PartOptions {
//!     id: Some("W-100".into()),
//!     version: Some("B".into()),
//!     ..Default::default()
//! }).unwrap();
//!
//! // Mirror the kernel's own arrays — vertex → edge → face → solid, one
//! // handle per element.
//! let vs: Vec<_> = kernel.points.iter()
//!     .map(|p| b.vertex(*p).unwrap())
//!     .collect();
//! let es: Vec<_> = kernel.edges.iter()
//!     .map(|e| b.edge(vs[e.start], vs[e.end], curve_of(e)).unwrap())
//!     .collect();
//! let fs: Vec<_> = kernel.faces.iter()
//!     .map(|f| b.face(surface_of(f), f.orientation, loops_of(f, &es)).unwrap())
//!     .collect();
//! b.solid(wheel, "wheel body", fs).unwrap();
//!
//! // Assemblies, display meshes, colours, layers, and PLM metadata are one
//! // call each.
//! step_io::write(&b.finish().unwrap(), std::fs::File::create("wheel.step").unwrap()).unwrap();
//! ```

#[cfg(test)]
mod author_tests;
pub mod build;
mod emit;
mod generated;
mod header;
mod orientation;
pub mod parser;
mod reader;
mod refgraph;
pub mod scene;

pub mod model {
    //! The schema-faithful model: [`StepModel`], with one list of entities per
    //! entity type.
    //!
    //! It is laid out compactly: an entity refers to another by its 32-bit index
    //! in that type's list (the `…Id` types, [`EntityKey`]); every string is a
    //! [`Text`] into the model's text table, read with [`StepModel::text`]; and a
    //! list of at most three reals — a point's coordinates, a direction, a mesh
    //! vertex — is a [`Reals3`] held inline.
    //!
    //! This is the low-level layer under [`scene`](crate::scene) and
    //! [`StepBuilder`](crate::StepBuilder), for the entities their handles do not
    //! cover. [`StepModel::all`] lists the entities of one type with their ids,
    //! and [`StepModel::get`] or `model[id]` reads the entity an id names. A
    //! reference field is an `…Ref` enum naming the entity types it may point
    //! at: [`resolve`](CurveRef::resolve) follows one to the entity it names,
    //! and [`entity_key`](CurveRef::entity_key) gives its key for
    //! [`RefGraph::referrers`](crate::RefGraph::referrers). The model is read
    //! only; new entities enter only through the
    //! [`Ap242Author`](crate::Ap242Author) constructors.
    //!
    //! ```no_run
    //! use step_io::model::{Axis2PlacementRef, CartesianPointRef, Circle};
    //!
    //! let (model, _report) = step_io::read(std::fs::File::open("part.step").unwrap()).unwrap();
    //! for (_id, circle) in model.all::<Circle>() {
    //!     let Axis2PlacementRef::Axis2Placement3d(frame) = circle.position else { continue };
    //!     let CartesianPointRef::CartesianPoint(origin) = model[frame].location else { continue };
    //!     let _centre = &model[origin].coordinates;
    //! }
    //! ```

    pub use crate::generated::model::*;
    pub use crate::generated::resolve::*;
}

pub use build::StepBuilder;
pub use emit::write;
pub use generated::author::{Ap242Author, AuthorError};
pub use generated::model::{EntityKey, StepModel};
pub use header::FileHeader;
pub use orientation::Orientation;
pub use reader::{DropKind, DropReason, Report, read};

// Internal round-trip oracle — kept callable for the external verification
// harness, but not part of the supported API surface.
#[doc(hidden)]
pub use emit::dump_universal;
pub use parser::entity::{Error, ParseWarning};
pub use parser::lexer::{LexError, LexErrorKind};
pub use parser::schema::{ApFamily, SchemaId, Stage};
pub use refgraph::RefGraph;

/// Crate-wide result alias: the only fallible public entry point ([`read`])
/// fails with [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
