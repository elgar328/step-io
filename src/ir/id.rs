use super::arena::{Arena, define_id};
use super::assembly::Product;
use super::geometry::{
    Axis1Placement, Axis2Placement2d, Axis2Placement3d, Curve, Curve2d, Direction2, Direction3,
    Point2, Point3, Surface, Vertex,
};
use super::plm::{
    CalendarDate, CoordinatedUniversalTimeOffset, DateAndTime, DateAndTimeAssignment, DateTimeRole,
    LocalTime, Organization, Person, PersonAndOrganization, PersonAndOrganizationRole,
};
use super::shape_rep::{ShapeAspect, UnitContext};
use super::topology::{Edge, Face, Shell, Solid, Wire};
use super::visualization::{
    Colour, CurveFont, CurveStyle, FoundedItem, PresentationLayerAssignment,
    PresentationStyleAssignment, StyledItem, SurfaceStyleRendering,
};

// Geometry Ids (3D)
define_id!(PointId, Point3);
define_id!(DirectionId, Direction3);
define_id!(SurfaceId, Surface);
define_id!(CurveId, Curve);
define_id!(Placement3dId, Axis2Placement3d);
define_id!(Placement1dId, Axis1Placement);

// Geometry Ids (2D — PCURVE parametric space)
define_id!(Point2dId, Point2);
define_id!(Direction2dId, Direction2);
define_id!(Curve2dId, Curve2d);
define_id!(Placement2dId, Axis2Placement2d);

// Topology Ids
define_id!(VertexId, Vertex);
define_id!(EdgeId, Edge);
define_id!(WireId, Wire);
define_id!(FaceId, Face);
define_id!(ShellId, Shell);
define_id!(SolidId, Solid);

// Assembly Ids
define_id!(ProductId, Product);

// Unit context Ids — multi-context support (one entry per
// REPRESENTATION_CONTEXT in the source file).
define_id!(UnitContextId, UnitContext);

// PMI Ids — anchor for future Tolerance / Datum / GD&T work.
define_id!(ShapeAspectId, ShapeAspect);

// Visualization Ids — Colour enum arena (ColourRgb + DraughtingPreDefinedColour).
define_id!(ColourId, Colour);

// Visualization Ids — CURVE_STYLE chain.
define_id!(CurveFontId, CurveFont);
define_id!(CurveStyleId, CurveStyle);

// Visualization Ids — STYLED_ITEM enum arena (Plain + future variants).
define_id!(StyledItemId, StyledItem);

// Visualization Ids — PRESENTATION_STYLE_ASSIGNMENT enum arena
// (Itself + PresentationStyleByContext).
define_id!(PresentationStyleAssignmentId, PresentationStyleAssignment);

// Visualization Ids — SURFACE_STYLE_RENDERING enum arena
// (Itself + SurfaceStyleRenderingWithProperties).
define_id!(SurfaceStyleRenderingId, SurfaceStyleRendering);

// Visualization Ids — founded_item enum arena (AP214 founded-item
// supertype; E1 covers FillAreaStyle + SurfaceStyleFillArea).
define_id!(FoundedItemId, FoundedItem);

// Visualization Ids — PRESENTATION_LAYER_ASSIGNMENT arena (top-level;
// no other entity refs it, the id exists for blueprint symmetry).
define_id!(PresentationLayerAssignmentId, PresentationLayerAssignment);

// plm Ids — Date/Time primitives (Phase plm-1a).
define_id!(DateId, CalendarDate);
define_id!(LocalTimeId, LocalTime);
define_id!(
    CoordinatedUniversalTimeOffsetId,
    CoordinatedUniversalTimeOffset
);
define_id!(DateAndTimeId, DateAndTime);
define_id!(DateTimeRoleId, DateTimeRole);
define_id!(DateAndTimeAssignmentId, DateAndTimeAssignment);
define_id!(PersonId, Person);
define_id!(OrganizationId, Organization);
define_id!(PersonAndOrganizationId, PersonAndOrganization);
define_id!(PersonAndOrganizationRoleId, PersonAndOrganizationRole);
