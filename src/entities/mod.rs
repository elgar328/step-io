//! Per-entity self-contained handlers + compile-time dispatch registry.
//!
//! Each entity lives in `src/entities/<group>/<name>.rs` and impls one of
//! [`SimpleEntityHandler`] (single-line `RawEntity::Simple`) or
//! [`ComplexEntityHandler`] (multi-part `RawEntity::Complex`). Plan 3 added
//! the complex variant so registry dispatch can cover the rational B-spline
//! family. Writer dispatch still goes through hand-rolled emit methods.

use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub mod assembly_product;
pub mod geometry;
pub mod plm;
pub mod property;
pub mod shape_rep;
pub mod topology;
pub mod units;
pub mod visualization;

/// Reader pass ordering. Lower variants run first.
///
/// Plan 3 wires Pass1 (DIRECTION) / Pass2 (VECTOR) / `Pass4Rational`
/// (`RATIONAL_B_SPLINE_CURVE`). Plan 4 adds the topology Pass5 family.
/// Other passes land here as Plan 5~7 walks the remaining `run_pass!`
/// blocks in `src/reader/passes.rs`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PassLevel {
    /// `CARTESIAN_POINT`, `DIRECTION` — no entity-ref dependencies.
    Pass1,
    /// `VECTOR` — depends on Pass1 entities.
    Pass2,
    /// `RATIONAL_B_SPLINE_CURVE` / `RATIONAL_B_SPLINE_SURFACE` — Pass 4-2,
    /// complex entities depending on Pass 4-1 leaf curves/surfaces.
    Pass4Rational,

    // ----- Plan 5 (geometry_3d 3D) -----
    /// `AXIS2_PLACEMENT_3D` / `AXIS1_PLACEMENT` (Pass 3) — depend on
    /// points and directions.
    Pass3,
    /// Pass 4-1 leaf curves and surfaces — `LINE`, `CIRCLE`, `ELLIPSE`,
    /// `B_SPLINE_CURVE_WITH_KNOTS`, `PLANE`, `CYLINDRICAL_SURFACE`,
    /// `SPHERICAL_SURFACE`, `CONICAL_SURFACE`, `TOROIDAL_SURFACE`,
    /// `B_SPLINE_SURFACE_WITH_KNOTS`. All mutually independent.
    Pass4Leaf,
    /// `SURFACE_CURVE` / `SEAM_CURVE` (Pass 4-3) — alias to a 3D curve
    /// with associated pcurves resolved in a post-pass.
    Pass4_3SurfaceCurve,
    /// `TRIMMED_CURVE` + `COMPOSITE_CURVE_SEGMENT` (Pass 4-3c) — both
    /// depend on a basis curve but not on each other.
    Pass4_3cTrimSeg,
    /// `COMPOSITE_CURVE` (Pass 4-3c) — depends on
    /// `COMPOSITE_CURVE_SEGMENT`.
    Pass4_3cComp,
    /// `SURFACE_OF_REVOLUTION` / `SURFACE_OF_LINEAR_EXTRUSION`
    /// (Pass 4-4A) — derived surfaces wrapping a swept curve.
    Pass4_4Swept,

    // ----- Plan 5.5 (PCURVE definitional 2D geometry) -----
    /// 2D `DIRECTION`. Shares the entity name with its 3D counterpart;
    /// each handler self-discriminates by coordinate count and silently
    /// skips the wrong dimension, so orphan 2D directions (those not
    /// reachable from a `DEFINITIONAL_REPRESENTATION`) still survive
    /// round-trip. (2D `CARTESIAN_POINT` moved to `Pass1` for the same
    /// reason and is dispatched alongside the 3D handler.)
    Pass4aPoint,
    /// 2D `VECTOR` + `AXIS2_PLACEMENT_2D` (Pass 4a-2). Independent of
    /// each other; both depend only on `Pass4aPoint` outputs.
    Pass4aVector,
    /// 2D curves (Pass 4a-3) — `LINE` / `CIRCLE` / `ELLIPSE` /
    /// `B_SPLINE_CURVE_WITH_KNOTS`. Each handler discriminates 2D vs 3D
    /// by its first cross-reference (point / placement in the 2D arena)
    /// and silently skips when the reference is absent.
    Pass4aCurve,
    /// 2D rational `RATIONAL_B_SPLINE_CURVE` (Pass 4a-4) — complex entity
    /// living inside a PCURVE `DEFINITIONAL_REPRESENTATION`. Mirrors the
    /// 3D `Pass4Rational` but dispatched through `dispatch_registry_2d`.
    Pass4aRational,

    // ----- Plan 5.6 (units, Pass 0 — runs before geometry passes) -----
    /// 3 unit leaf complex entities (`LENGTH_UNIT` / `PLANE_ANGLE_UNIT`
    /// / `SOLID_ANGLE_UNIT` bearings). Mutually independent.
    Pass0Leaf,
    /// `UNCERTAINTY_MEASURE_WITH_UNIT` (simple). Depends on `Pass0Leaf`
    /// outputs (`length_unit_map`).
    Pass0Uncertainty,
    /// `GLOBAL_UNIT_ASSIGNED_CONTEXT` (complex orchestrator). Depends on
    /// `Pass0Leaf` + `Pass0Uncertainty` outputs.
    Pass0Context,

    // ----- Plan 4 (topology) -----
    /// `VERTEX_POINT` (Pass 5-1) — depends on `CARTESIAN_POINT`.
    Pass5Vertex,
    /// `EDGE_CURVE` (Pass 5-2) — depends on vertices and curves.
    Pass5Edge,
    /// `ORIENTED_EDGE` (Pass 5-3, intermediate map) — depends on edges.
    Pass5OrientedEdge,
    /// `EDGE_LOOP`, `VERTEX_LOOP` (Pass 5-4, intermediate map) — depend on
    /// oriented edges / vertices.
    Pass5EdgeLoop,
    /// `FACE_BOUND`, `FACE_OUTER_BOUND` (Pass 5-5, intermediate map) —
    /// depend on edge/vertex loops.
    Pass5FaceBound,
    /// `ADVANCED_FACE`, `FACE_SURFACE` (Pass 5-6) — depend on face bounds
    /// and surfaces.
    Pass5Face,
    /// `CLOSED_SHELL`, `OPEN_SHELL` (Pass 5-7a) — depend on faces.
    Pass5Shell,
    /// `ORIENTED_CLOSED_SHELL` (Pass 5-7b, intermediate map) — depends on
    /// `CLOSED_SHELL` already in arena.
    Pass5OrientedShell,
    /// `MANIFOLD_SOLID_BREP`, `BREP_WITH_VOIDS` (Pass 5-8) — depend on
    /// shells / oriented shells.
    Pass5Solid,

    // ----- Plan 6 (Pass 6: assembly + shape rep) -----
    /// `PRODUCT` (Pass 6-1) — top of the product chain.
    Pass6Product,
    /// `PRODUCT_CATEGORY` + `PRODUCT_RELATED_PRODUCT_CATEGORY`
    /// (Pass 6-1b sub-pass a). Mutually independent; both populate
    /// per-product metadata used by `ProductCategoryRelationship`.
    Pass6ProductCategory,
    /// `PRODUCT_CATEGORY_RELATIONSHIP` (Pass 6-1b sub-pass b) — depends on
    /// `Pass6ProductCategory` outputs.
    Pass6ProductCategoryRel,
    /// `PRODUCT_DEFINITION_FORMATION` +
    /// `PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE` (Pass 6-2).
    /// Independent of each other; both attach to a `Product`.
    Pass6PdefFormation,
    /// `PRODUCT_DEFINITION` (Pass 6-3) — depends on `Pass6PdefFormation`.
    Pass6Pdef,
    /// `SHELL_BASED_SURFACE_MODEL` (Pass 6-4) — must precede MSSR so the
    /// shell-list is available when the surface representation lands.
    Pass6Sbsm,
    /// `ADVANCED_BREP_SHAPE_REPRESENTATION` +
    /// `MANIFOLD_SURFACE_SHAPE_REPRESENTATION` + plain `SHAPE_REPRESENTATION`
    /// (Pass 6-4a). Three concrete entity names sharing one pass.
    Pass6ShapeRep,
    /// `GEOMETRIC_CURVE_SET` + `GEOMETRIC_SET` (Pass 6-4f). Must precede
    /// GBWSR/GBSSR so the wireframe converters can resolve the curve-set
    /// payload.
    Pass6CurveSet,
    /// `GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION` +
    /// `GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION` (Pass 6-4g).
    /// Both wrappers share the same inner shape (`convert_wireframe_*`).
    Pass6Gbsr,
    /// `SHAPE_REPRESENTATION_RELATIONSHIP` (Pass 6-4b) — must run after the
    /// shape-representation passes so the is-target lookup sees populated
    /// maps.
    Pass6SrRel,
    /// `SHAPE_DEFINITION_REPRESENTATION` (Pass 6-5) — classifies each
    /// product as Solid or Group.
    Pass6Sdr,
    /// `ITEM_DEFINED_TRANSFORMATION` (Pass 6-6) — builds `transform_map`.
    Pass6Idt,
    /// `NEXT_ASSEMBLY_USAGE_OCCURRENCE` (Pass 6-8) — pushes Instances into
    /// parent products' Group content.
    Pass6Nauo,
    /// `PRODUCT_DEFINITION_SHAPE` classifier (Pass 6-4c) — classifies each
    /// `PDEF_SHAPE` as product-owned (`PDEF` target) or instance-owned
    /// (`NAUO` target). Reads only, no IR mutation beyond two lookup maps.
    Pass6PdsClassify,
    /// `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION` (Pass 6-7) — binds each
    /// NAUO to a Transform3d. Needs `graph` to resolve the RR-complex
    /// sub-entity.
    Pass6Cdsr,

    // ----- Plan 7 (Pass 4-4B + Pass 7 visualization + Pass 8 property/PMI) -----
    /// `OFFSET_SURFACE` (Pass 4-4B) — fixpoint dispatch. Dispatched via
    /// [`Self::dispatch_registry_until_fixpoint`] because a chain of
    /// `OFFSET_SURFACE` on top of `OFFSET_SURFACE` may resolve in any order.
    Pass4_4Offset,
    /// `COLOUR_RGB` (Pass 7-1) — leaf, no entity-ref dependencies.
    Pass7Colour,
    /// `FILL_AREA_STYLE_COLOUR` (Pass 7-2) — depends on `Pass7Colour`.
    Pass7FillColour,
    /// `FILL_AREA_STYLE` (Pass 7-3) — depends on `Pass7FillColour`.
    Pass7FillArea,
    /// `SURFACE_STYLE_FILL_AREA` (Pass 7-4) — depends on `Pass7FillArea`.
    Pass7SurfaceFill,
    /// `SURFACE_STYLE_TRANSPARENT` (Pass 7-5) — leaf, populates the
    /// transparent map for `Pass7Rendering`.
    Pass7Transparent,
    /// `SURFACE_STYLE_RENDERING_WITH_PROPERTIES` (Pass 7-6) — depends on
    /// `Pass7Colour` + `Pass7Transparent`.
    Pass7Rendering,
    /// `SURFACE_SIDE_STYLE` (Pass 7-7) — depends on `Pass7SurfaceFill` +
    /// `Pass7Rendering`.
    Pass7SurfaceSide,
    /// `SURFACE_STYLE_USAGE` (Pass 7-8) — depends on `Pass7SurfaceSide`.
    Pass7Usage,
    /// `CURVE_STYLE` (Pass 7-8b) — depends on `Pass7Colour` (the
    /// `curve_colour` ref) and the font leaf populated alongside it.
    /// Runs before `Pass7Assignment` so a PSA can dispatch curve-styling
    /// refs.
    Pass7CurveStyle,
    /// `PRESENTATION_STYLE_ASSIGNMENT` (Pass 7-9) — depends on `Pass7Usage`
    /// and `Pass7CurveStyle`.
    Pass7Assignment,
    /// `STYLED_ITEM` (Pass 7-10) — depends on `Pass7Assignment` plus
    /// multi-pool item lookup (solid / face / curve / point maps).
    Pass7StyledItem,
    /// `OVER_RIDING_STYLED_ITEM` (Pass 7-10b) — depends on `Pass7StyledItem`
    /// so its `over_ridden_style` ref resolves to an existing entry in
    /// `VisualizationPool::styled_items`.
    Pass7OverRiding,
    /// `MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION` (Pass 7-11)
    /// — depends on `Pass7StyledItem` outputs.
    Pass7Mdgpr,
    /// `PRESENTATION_LAYER_ASSIGNMENT` (Pass 7-12) — depends on
    /// `Pass7StyledItem` / `Pass7OverRiding` so each `assigned_items` ref
    /// resolves to an existing `StyledItemId`.
    Pass7Pla,
    /// plm Date/Time leaves (Pass 9-1) — `CALENDAR_DATE`,
    /// `COORDINATED_UNIVERSAL_TIME_OFFSET`, `DATE_TIME_ROLE`. No external deps.
    Pass9PlmDateLeaves,
    /// `LOCAL_TIME` (Pass 9-2) — depends on `Pass9PlmDateLeaves` (UTC ref).
    Pass9PlmLocalTime,
    /// `DATE_AND_TIME` (Pass 9-3) — depends on date arena + `LocalTime` arena.
    Pass9PlmDateAndTime,
    /// `APPLIED_DATE_AND_TIME_ASSIGNMENT` /
    /// `CC_DESIGN_DATE_AND_TIME_ASSIGNMENT` (Pass 9-4) — depends on
    /// `Pass9PlmDateAndTime` (`date_and_time` + role refs) and the
    /// assembly product chain (`Pass6`) for `items` PD targets.
    Pass9PlmDta,
    /// `PERSON`, `ORGANIZATION`, `PERSON_AND_ORGANIZATION_ROLE` (Pass 9-5)
    /// — plm leaves with no plm-internal refs.
    Pass9PlmPoLeaves,
    /// `PERSON_AND_ORGANIZATION` (Pass 9-6) — depends on `Pass9PlmPoLeaves`
    /// (Person + Organization arenas).
    Pass9PlmPersonAndOrganization,
    /// `APPLIED_PERSON_AND_ORGANIZATION_ASSIGNMENT` /
    /// `CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT` (Pass 9-7) — depends
    /// on `Pass9PlmPersonAndOrganization` (P&O ref) + `Pass9PlmPoLeaves`
    /// (role) and the assembly product chain (`Pass6`) for `items` PD targets.
    Pass9PlmPoa,
    /// `APPROVAL_STATUS`, `APPROVAL_ROLE` (Pass 9-8) — plm Approval leaves.
    Pass9PlmApprovalLeaves,
    /// `APPROVAL` (Pass 9-9) — depends on `Pass9PlmApprovalLeaves` for the
    /// status ref.
    Pass9PlmApproval,
    /// `APPROVAL_DATE_TIME` / `APPROVAL_PERSON_ORGANIZATION` (Pass 9-10) —
    /// depend on `Pass9PlmApproval` plus `Pass9PlmDateAndTime` /
    /// `Pass9PlmPersonAndOrganization` for the SELECT refs.
    Pass9PlmApprovalLinkers,
    /// `SHAPE_ASPECT` (Pass 8-pre) — PMI scaffolding. Runs before the
    /// property converters so a future Pattern B PD pass can resolve its
    /// target ref through the `SHAPE_ASPECT` id map.
    Pass8ShapeAspect,
    /// `MEASURE_REPRESENTATION_ITEM` (Pass 8-1) — depends on Pass 0
    /// (unit ctx).
    Pass8Measure,
    /// `PROPERTY_DEFINITION` (Pass 8-2) — depends on Pass 6 (`pdef_to_product`)
    /// for resolving the PD's target.
    Pass8PropertyDef,
    /// `PROPERTY_DEFINITION_REPRESENTATION` (Pass 8-3) — binds a
    /// `PROPERTY_DEFINITION` to its underlying `REPRESENTATION`. Needs
    /// `graph` to walk the bound REPRESENTATION (a generic entity name
    /// shared with MDGPR / SR — a per-pass map would conflate them).
    Pass8Pdr,
}

/// Handler for a [`RawEntity::Simple`] STEP entity. Reader receives a flat
/// attribute list; writer takes a per-entity [`Self::WriteInput`].
pub(crate) trait SimpleEntityHandler {
    /// Uppercase STEP entity name (e.g. `"DIRECTION"`).
    const NAME: &'static str;

    /// Reader pass level. See [`PassLevel`].
    const PASS_LEVEL: PassLevel;

    /// Writer input. Differs per entity (e.g. `DirectionId` for a directly
    /// stored arena entry, `(DirectionId, f64)` for vectors stored as a
    /// tuple).
    type WriteInput;

    /// Read STEP attributes into the reader context. Body mirrors the
    /// legacy `convert_*` method one-to-one — `&mut self` becomes `ctx`.
    ///
    /// `graph` exposes the raw entity graph for sub-entity / ref-list
    /// resolution. Pass-produced results (other handlers' IR output, per-pass
    /// maps) must be reached through `ctx`, not through `graph`; consulting
    /// the graph for another entity's *result* (vs its raw attributes) would
    /// breach pass ordering.
    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError>;

    /// Emit a STEP entity from IR input. Returns the freshly-allocated
    /// STEP entity id.
    fn write(buf: &mut WriteBuffer, input: Self::WriteInput) -> Result<u64, WriteError>;
}

/// Handler for a [`RawEntity::Complex`] STEP entity. The reader receives a
/// list of [`RawEntityPart`] and dispatch keys on [`Self::REQUIRED_PARTS`]
/// (every listed part name must be present).
pub(crate) trait ComplexEntityHandler {
    /// Metadata-only label (e.g. `"RATIONAL_B_SPLINE_CURVE"`). Dispatch
    /// keys on [`Self::REQUIRED_PARTS`], not on this name.
    const NAME: &'static str;

    /// Reader pass level. See [`PassLevel`].
    const PASS_LEVEL: PassLevel;

    /// Writer input. Same role as the simple-handler associated type.
    type WriteInput;

    /// Read the complex parts into the reader context. See
    /// [`SimpleEntityHandler::read`] for the `graph` usage contract.
    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError>;

    /// Emit a STEP entity from IR input.
    fn write(buf: &mut WriteBuffer, input: Self::WriteInput) -> Result<u64, WriteError>;
}

/// Distinguishes the two reader entry shapes inside a single
/// [`EntityHandlerEntry`] / [`ENTITY_HANDLERS`] slice.
#[allow(dead_code)] // variants and fields read by dispatch_entry in src/reader/passes.rs
pub(crate) enum ReadKind {
    /// Matches `RawEntity::Simple` whose name equals the entry's `name`.
    Simple {
        read: fn(&mut ReaderContext, u64, &[Attribute], &EntityGraph) -> Result<(), ConvertError>,
    },
    /// Matches `RawEntity::Complex` whose parts contain every name in
    /// `required_parts`.
    Complex {
        required_parts: &'static [&'static str],
        read:
            fn(&mut ReaderContext, u64, &[RawEntityPart], &EntityGraph) -> Result<(), ConvertError>,
    },
}

/// Reader-side registry entry. Each handler module submits one via
/// `#[linkme::distributed_slice(ENTITY_HANDLERS)]`. Writer is intentionally
/// absent — see Plan 3 for the type-erasure trade-off.
#[allow(dead_code)] // Fields are read by dispatch_entry in src/reader/passes.rs
pub(crate) struct EntityHandlerEntry {
    pub name: &'static str,
    pub pass_level: PassLevel,
    pub kind: ReadKind,
}

/// Compile-time registry of entity handlers contributing to reader
/// dispatch. Populated at link time by `#[distributed_slice]` on each
/// handler module's `static` entry.
#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice]
pub(crate) static ENTITY_HANDLERS: [EntityHandlerEntry] = [..];
