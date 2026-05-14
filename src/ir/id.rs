use super::arena::{Arena, define_id};
use super::assembly::Product;
use super::geometry::{
    Axis1Placement, Axis2Placement2d, Axis2Placement3d, Curve, Curve2d, Direction2, Direction3,
    Point2, Point3, Surface, Vertex,
};
use super::model::UnitContext;
use super::pmi::ShapeAspect;
use super::topology::{Edge, Face, Shell, Solid, Wire};

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
