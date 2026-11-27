//! Lean STEP (ISO 10303) I/O for 3D CAD applications — reads every mainstream AP,
//! writes only modern AP242.
//!
//! > ⚠️ **Experimental** — early stage; expect breaking API changes at any time.
//!
//! - **Reads everything, writes AP242** — any mainstream AP comes in, legacy
//!   included; only AP242 edition 4 (IS) goes out.
//! - **Lean** — one universal schema model instead of a per-schema
//!   implementation; the supported entity set is curated to what real STEP
//!   files actually use.
//! - **Heals non-standard input** — what can be fixed is normalized; what
//!   cannot is dropped with a reason, or kept where dropping it would lose
//!   real data — never silently.
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
//! Output is AP242 edition 4 (ISO 10303-242:2025, the current IS) only —
//! AP203 and AP214 were merged into AP242 in 2014. When a later edition
//! takes over, the output moves up with it: one output schema, always the
//! current one.
//!
//! Most of the pipeline is generated from the schemas rather than written
//! by hand — no drift, little to maintain.
//!
//! # Reading STEP
//!
//! Reading gives the result and the report together: [`read`] returns the
//! [`StepModel`] plus a [`Report`] of what was kept, normalized, or dropped
//! and why, and what the parser repaired. Messy files are common in the
//! wild — non-standard content is healed, dropped, or (a list outside its
//! size bounds) kept as read rather than failing the import, and the report
//! is the record of what happened on the way in. Reading checks each
//! entity's form against the schema — attributes, reference types, list
//! sizes, complex-entity parts — but not its `WHERE` rules, global `RULE`s,
//! or `UNIQUE` and `INVERSE` constraints (see [`read`]).
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
//! a generated low-level layer with one constructor per AP242 entity step-io
//! models that is not `ABSTRACT` (exposed as [`StepBuilder::author`]); every
//! argument is validated at insertion against the AP242 edition 4 schema —
//! its types, aggregate sizes, references, rules for combining types, and the
//! `WHERE` rules that limit which types an attribute, each of its elements, or
//! each element of a set it holds references, whether for every instance or
//! by the instance's own type (not those on one indexed element, on two
//! attributes together or on a path through another entity, nor its other
//! `WHERE` rules) — so there is no loss report to check afterwards.
//!
//! Some of the model's reference types are wider than AP242 edition 4 allows.
//! The model reads every edition, so an attribute a subtype narrows
//! (`SELF\supertype.attribute : narrower;`) only from AP242 on, or a SELECT
//! with members AP242 dropped, takes the wider type; and it does not hold an
//! attribute to a `WHERE` rule such as
//! `'LENGTH_UNIT' IN TYPEOF(SELF\measure_with_unit.unit_component)`. There a
//! constructor takes a narrower type and stores it as the model's, so a
//! reference AP242 does not allow does not compile:
//!
//! ```
//! # use step_io::{Ap242Author, model as m};
//! # fn f(a: &mut Ap242Author, shape: m::ProductDefinitionShapeId, rep: m::ShapeRepresentationId) {
//! a.add_shape_definition_representation(
//!     m::PropertyDefinitionRef::ProductDefinitionShape(shape),
//!     m::ShapeRepresentationRef::ShapeRepresentation(rep),
//! );
//! # }
//! ```
//!
//! ```compile_fail,E0308
//! # use step_io::{Ap242Author, model as m};
//! # fn f(a: &mut Ap242Author, aspect: m::ShapeAspectId, rep: m::ShapeRepresentationId) {
//! // AP214 lets a shape aspect have a shape representation; AP242 does not.
//! a.add_shape_definition_representation(
//!     m::RepresentedDefinitionRef::ShapeAspect(aspect),
//!     m::ShapeRepresentationRef::ShapeRepresentation(rep),
//! );
//! # }
//! ```
//!
//! Where an argument's type cannot rule it out — a complex instance's parts,
//! the struct a self-referencing constructor takes — the check runs on
//! insertion and returns [`AuthorError::NarrowedReference`]. A reference to
//! an entity type AP242 does not have at all compiles where the model's type
//! names it, and is refused on insertion with [`AuthorError::NotAp242`].
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
//! let mut b = StepBuilder::new(); // millimetre; new_with() to choose
//! let wheel = b.part_with("wheel", &PartOptions {
//!     id: Some("W-100".into()),
//!     version: Some("B".into()),
//!     ..Default::default()
//! });
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
//! step_io::write(&b.finish(), std::fs::File::create("wheel.step").unwrap()).unwrap();
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
    //! A read model holds what the schema allows, with one exception: a list
    //! with more or fewer elements than its schema bounds allow (a face with
    //! no bounds) is kept as read, since dropping it would drop the solid it
    //! bounds too. [`Report::kept_nonstandard`](crate::Report::kept_nonstandard)
    //! lists each; `Ap242Author` builds no such list.
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

// Internal round-trip oracle — kept callable for testing a read model end to
// end, but not part of the supported API surface.
#[doc(hidden)]
pub use emit::dump_universal;
pub use parser::entity::{Error, ParseWarning};
pub use parser::lexer::{LexError, LexErrorKind};
pub use parser::schema::{ApFamily, SchemaId, Stage};
pub use refgraph::RefGraph;

/// Compile-time checks of the AP242 edition 4 argument types and
/// constructors, most paired with a version that compiles, so a
/// `compile_fail` fails for its reason.
///
/// AP242's `approved_item` drops `PRODUCT_DEFINITION`, which AP203 allows:
///
/// ```
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, approval: m::ApprovalId, product: m::ProductId) {
/// let _ = a.add_cc_design_approval(
///     m::ApprovalRef::Approval(approval),
///     vec![m::Ap242ApprovedItemRef::Product(product)],
/// );
/// # }
/// ```
///
/// ```compile_fail,E0599
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, approval: m::ApprovalId, pd: m::ProductDefinitionId) {
/// let _ = a.add_cc_design_approval(
///     m::ApprovalRef::Approval(approval),
///     vec![m::Ap242ApprovedItemRef::ProductDefinition(pd)],
/// );
/// # }
/// ```
///
/// AP242's `styled_item_target` leaves out a descriptive item, which AP214
/// lets a styled item style:
///
/// ```
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, style: m::PresentationStyleAssignmentId, point: m::CartesianPointId) {
/// let _ = a.add_styled_item(
///     "",
///     vec![m::PresentationStyleAssignmentRef::PresentationStyleAssignment(style)],
///     m::Ap242StyledItemItemRef::CartesianPoint(point),
/// );
/// # }
/// ```
///
/// ```compile_fail,E0599
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, style: m::PresentationStyleAssignmentId, note: m::DescriptiveRepresentationItemId) {
/// let _ = a.add_styled_item(
///     "",
///     vec![m::PresentationStyleAssignmentRef::PresentationStyleAssignment(style)],
///     m::Ap242StyledItemItemRef::DescriptiveRepresentationItem(note),
/// );
/// # }
/// ```
///
/// `STYLED_ITEM.WR3` lets a styled item style only a mapped item, a
/// geometric item, or a set of them; edition 4 applies this to a
/// context-dependent over-riding styled item too:
///
/// ```compile_fail,E0599
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, style: m::PresentationStyleAssignmentId, shell: m::ClosedShellId) {
/// let _ = a.add_styled_item(
///     "",
///     vec![m::PresentationStyleAssignmentRef::PresentationStyleAssignment(style)],
///     m::Ap242StyledItemItemRef::ClosedShell(shell),
/// );
/// # }
/// ```
///
/// ```
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, style: m::PresentationStyleAssignmentId, face: m::AdvancedFaceId) {
/// let _ = a.add_styled_item(
///     "",
///     vec![m::PresentationStyleAssignmentRef::PresentationStyleAssignment(style)],
///     m::Ap242StyledItemItemRef::SetRepresentationItem(vec![
///         m::Ap242StyledItemItemSetRepresentationItemRef::AdvancedFace(face),
///     ]),
/// );
/// # }
/// ```
///
/// ```compile_fail,E0599
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, style: m::PresentationStyleAssignmentId, shell: m::ClosedShellId) {
/// let _ = a.add_styled_item(
///     "",
///     vec![m::PresentationStyleAssignmentRef::PresentationStyleAssignment(style)],
///     m::Ap242StyledItemItemRef::SetRepresentationItem(vec![
///         m::Ap242StyledItemItemSetRepresentationItemRef::ClosedShell(shell),
///     ]),
/// );
/// # }
/// ```
///
/// ```compile_fail,E0599
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, style: m::PresentationStyleAssignmentId, shell: m::ClosedShellId,
/// #      styled: m::StyledItemId, face: m::AdvancedFaceId) {
/// let _ = a.add_context_dependent_over_riding_styled_item(
///     "",
///     vec![m::PresentationStyleAssignmentRef::PresentationStyleAssignment(style)],
///     m::Ap242StyledItemItemRef::ClosedShell(shell),
///     m::StyledItemRef::StyledItem(styled),
///     vec![m::StyleContextSelectRef::AdvancedFace(face)],
/// );
/// # }
/// ```
///
/// An over-riding styled item references the styled item it overrides, so it
/// has a self-referencing constructor; AP242's `styled_item_target` leaves
/// out styled items, so a plain styled item, whose only reference to one
/// would be its `item`, has none:
///
/// ```
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, style: m::PresentationStyleAssignmentRef, point: m::CartesianPointId) {
/// let name = a.text("");
/// let _ = a.add_over_riding_styled_item_cyclic(|id| m::OverRidingStyledItem {
///     name,
///     styles: vec![style],
///     item: m::StyledItemTargetOrRepresentationItemRef::CartesianPoint(point),
///     over_ridden_style: m::StyledItemRef::OverRidingStyledItem(id),
/// });
/// # }
/// ```
///
/// ```compile_fail,E0599
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, style: m::PresentationStyleAssignmentRef, point: m::CartesianPointId) {
/// let name = a.text("");
/// let _ = a.add_styled_item_cyclic(|_| m::StyledItem {
///     name,
///     styles: vec![style],
///     item: m::StyledItemTargetOrRepresentationItemRef::CartesianPoint(point),
/// });
/// # }
/// ```
///
/// `ADVANCED_FACE.WR1` lets a face lie only on an elementary, B-spline or
/// swept surface:
///
/// ```
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, bounds: Vec<m::FaceBoundRef>, plane: m::PlaneId) {
/// let _ = a.add_advanced_face(
///     "",
///     bounds,
///     m::Ap242AdvancedFaceGeometrySelectRef::Plane(plane),
///     true,
/// );
/// # }
/// ```
///
/// ```compile_fail,E0599
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, bounds: Vec<m::FaceBoundRef>, offset: m::OffsetSurfaceId) {
/// let _ = a.add_advanced_face(
///     "",
///     bounds,
///     m::Ap242AdvancedFaceGeometrySelectRef::OffsetSurface(offset),
///     true,
/// );
/// # }
/// ```
///
/// `CONSTRUCTIVE_GEOMETRY_REPRESENTATION.WR2` lets each item be one of the
/// types it lists. Edition 4 asks for exactly one, though it lists
/// supertypes with their subtypes (a cartesian point is also a `point`, a
/// line a `curve`, a circle a `curve` and a `conic`, a plane a `surface` and
/// an `elementary_surface`), which would leave out a cartesian point, a line,
/// a circle or a plane; this is a known issue of the edition, and the rule is
/// read as asking for at least one. A mapped item is not listed:
///
/// ```
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, point: m::CartesianPointId, ctx: m::GeometricRepresentationContextId) {
/// let _ = a.add_constructive_geometry_representation(
///     "",
///     vec![m::Ap242ConstructiveGeometryRepresentationItemsRef::CartesianPoint(point)],
///     m::GeometricRepresentationContextRef::GeometricRepresentationContext(ctx),
/// );
/// # }
/// ```
///
/// ```compile_fail,E0599
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, item: m::MappedItemId, ctx: m::GeometricRepresentationContextId) {
/// let _ = a.add_constructive_geometry_representation(
///     "",
///     vec![m::Ap242ConstructiveGeometryRepresentationItemsRef::MappedItem(item)],
///     m::GeometricRepresentationContextRef::GeometricRepresentationContext(ctx),
/// );
/// # }
/// ```
///
/// `LENGTH_MEASURE_WITH_UNIT.WR1` asks for a length unit:
///
/// ```
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, value: m::MeasureValue, unit: m::LengthUnitId) {
/// let _ = a.add_length_measure_with_unit(value, m::Ap242LengthUnitRef::LengthUnit(unit));
/// # }
/// ```
///
/// ```compile_fail,E0599
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, value: m::MeasureValue, unit: m::MassUnitId) {
/// let _ = a.add_length_measure_with_unit(value, m::Ap242LengthUnitRef::MassUnit(unit));
/// # }
/// ```
///
/// AP242's `datum_or_common_datum` takes datum reference elements as a list
/// of two or more (`common_datum_list`), not one alone:
///
/// ```
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, pds: m::ProductDefinitionShapeRef, e: m::DatumReferenceElementId, g: m::DatumReferenceElementId) {
/// let _ = a.add_datum_reference_compartment(
///     "",
///     None,
///     pds,
///     m::Logical::False,
///     m::Ap242DatumOrCommonDatumRef::CommonDatumList(vec![
///         m::DatumReferenceElementRef::DatumReferenceElement(e),
///         m::DatumReferenceElementRef::DatumReferenceElement(g),
///     ]),
///     None,
/// );
/// # }
/// ```
///
/// ```compile_fail,E0599
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, pds: m::ProductDefinitionShapeRef, e: m::DatumReferenceElementId) {
/// let _ = a.add_datum_reference_compartment(
///     "",
///     None,
///     pds,
///     m::Logical::False,
///     m::Ap242DatumOrCommonDatumRef::DatumReferenceElement(e),
///     None,
/// );
/// # }
/// ```
///
/// An entity AP242 declares `ABSTRACT` has no constructor of its own; a
/// subtype's constructor makes one:
///
/// ```
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, aspect: m::ShapeAspectId) {
/// let _ = a.add_position_tolerance(
///     "",
///     None,
///     None,
///     m::GeometricToleranceTargetRef::ShapeAspect(aspect),
/// );
/// # }
/// ```
///
/// ```compile_fail,E0599
/// # use step_io::{Ap242Author, model as m};
/// # fn f(a: &mut Ap242Author, aspect: m::ShapeAspectId) {
/// let _ = a.add_geometric_tolerance(
///     "",
///     None,
///     None,
///     m::GeometricToleranceTargetRef::ShapeAspect(aspect),
/// );
/// # }
/// ```
#[cfg(doctest)]
mod ap242_narrowed_arguments {}
