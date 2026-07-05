//! Lean STEP (ISO 10303) I/O for 3D CAD kernels — reads every mainstream AP,
//! writes only modern AP242.
//!
//! > ⚠️ **Experimental** — early stage; expect breaking API changes at any time.
//!
//! - **Reads everything, writes AP242** — any mainstream AP comes in, legacy
//!   included; only AP242 edition 2 (IS) goes out.
//! - **Lean** — one universal schema model instead of a per-schema
//!   implementation; entities that never appear in real STEP files are pruned
//!   out.
//! - **Heals non-standard input** — what can be fixed is normalized; what
//!   cannot is dropped with a reason, never silently.
//! - **Ergonomic API** — [`scene()`](StepModel::scene) navigates what you read
//!   with lightweight handles; [`StepBuilder`] writes new files the way
//!   kernels hold data.
//!
//! # Design
//!
//! step-io exists to sit between a 3D CAD kernel and the STEP files it
//! exchanges.
//!
//! Most STEP libraries carry per-schema entity and read/write code, so each
//! supported schema adds source and binary size. step-io instead merges the
//! mainstream APs into one *universal* schema and reads them all through a
//! single pipeline. Entities that never appear in practice are left out,
//! judged against 50,000+ real-world files — the [entity coverage
//! matrix](https://github.com/elgar328/step-io/blob/main/docs/entities.md)
//! lists exactly what is read and written.
//!
//! Output is AP242 edition 2 (IS) only — AP203 and AP214 were merged into
//! it in 2014, and every modern tool reads it. When edition 3 reaches IS
//! and takes over, the output target moves up with it: one output schema,
//! always the current one.
//!
//! Most of the pipeline is generated from the schemas rather than written
//! by hand — no drift, little to maintain.
//!
//! # Reading STEP into a kernel or viewer
//!
//! Reading gives the result and the report together: [`read`] returns the
//! [`StepModel`] plus a [`Report`] of what was kept, normalized, or dropped
//! and why. Real-world files are rarely clean — messy content is healed or
//! dropped rather than failing the import, and the report is the record of
//! what happened on the way in.
//!
//! The model is raw and complete: one public arena per entity type,
//! schema-faithful. Everything the reader kept is directly accessible.
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
//! use step_io::scene::geometry::SurfaceKind;
//!
//! let source = std::fs::read("part.step").unwrap();
//! let (model, _report) = step_io::read(&source).unwrap(); // report: kept / normalized / dropped
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
//! # Writing STEP from a kernel
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
//! Part 21 header — see the [`build`] module docs.
//!
//! ```no_run
//! # use step_io::build::{CurveInput, FaceBoundInput, Frame, SurfaceInput};
//! # struct KernelEdge { start: usize, end: usize }
//! # struct KernelFace { same_sense: bool }
//! # struct Kernel { points: Vec<[f64; 3]>, edges: Vec<KernelEdge>, faces: Vec<KernelFace> }
//! # let kernel = Kernel { points: vec![], edges: vec![], faces: vec![] };
//! # fn curve_of(_: &KernelEdge) -> CurveInput { CurveInput::Line }
//! # fn surface_of(_: &KernelFace) -> SurfaceInput {
//! #     SurfaceInput::Plane(Frame { origin: [0.0; 3], axis: [0.0, 0.0, 1.0], ref_dir: [1.0, 0.0, 0.0] })
//! # }
//! # fn loops_of(_: &KernelFace, _es: &[step_io::generated::model::EdgeCurveId]) -> Vec<FaceBoundInput> {
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
//!     .map(|f| b.face(surface_of(f), f.same_sense, loops_of(f, &es)).unwrap())
//!     .collect();
//! b.solid(wheel, "wheel body", fs).unwrap();
//!
//! // Assemblies, display meshes, colours, layers, and PLM metadata are one
//! // call each.
//! std::fs::write("wheel.step", b.finish().unwrap()).unwrap();
//! ```

#[cfg(test)]
mod author_tests;
pub mod build;
pub mod emit;
pub mod generated;
pub mod header;
pub mod parser;
pub mod reader;
pub mod refgraph;
pub mod scene;

pub use build::StepBuilder;
pub use emit::write;
pub use generated::author::{Ap242Author, AuthorError};
pub use generated::model::{EntityKey, StepModel};
pub use header::FileHeader;
pub use reader::{DropKind, DropReason, Report, read};

// Internal round-trip oracle — kept callable for the external verification
// harness, but not part of the supported API surface.
#[doc(hidden)]
pub use emit::dump_universal;
pub use parser::{ApFamily, Error, LexError, LexErrorKind, SchemaId, Stage};
pub use refgraph::RefGraph;

/// Crate-wide result alias: the only fallible public entry point ([`read`])
/// fails with [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
