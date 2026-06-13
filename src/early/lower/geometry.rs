//! Geometry-domain `lower` fns (the W-RECURSIVE pilot: leaf points/directions
//! + vertex). See the [module docs](super) for the lowering contract.
//!
//! The 2D/3D `CARTESIAN_POINT` / `DIRECTION` sister handlers both run on the
//! same entity and self-claim by coordinate count; these lowers reproduce the
//! 3D branch exactly (2-count → the 2D arena claims it, so we no-op).

use crate::early::model::{
    EarlyAxis1Placement, EarlyAxis2Placement3d, EarlyCartesianPoint, EarlyCircle,
    EarlyConicalSurface, EarlyCylindricalSurface, EarlyDirection, EarlyEllipse, EarlyHyperbola,
    EarlyLine, EarlyParabola, EarlyPlane, EarlySphericalSurface, EarlyToroidalSurface, EarlyVector,
    EarlyVertexPoint,
};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{
    Axis1Placement, Axis2Placement3d, Circle3, ConicalSurface, Curve, CylindricalSurface,
    Direction3, Ellipse3, Hyperbola, Line3, Parabola, Plane3, Point3, SphericalSurface, Surface,
    ToroidalSurface, Vertex,
};
use crate::reader::ReaderContext;

/// Lower one 3D `CARTESIAN_POINT`. A 2-coordinate point belongs to the 2D
/// sister arena (no-op here); anything but 2 or 3 is malformed (Err, matching
/// the legacy handler).
pub(crate) fn lower_cartesian_point(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyCartesianPoint,
) -> Result<(), ConvertError> {
    let coords = early.coordinates;
    match coords.len() {
        3 => {}
        2 => return Ok(()),
        n => {
            return Err(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("CARTESIAN_POINT must have 2 or 3 coordinates, got {n}"),
            });
        }
    }
    let id = ctx.geometry.points.push(Point3 {
        x: coords[0],
        y: coords[1],
        z: coords[2],
    });
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one 3D `DIRECTION` (same 2D/3D self-claim shape as the point).
pub(crate) fn lower_direction(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDirection,
) -> Result<(), ConvertError> {
    let ratios = early.direction_ratios;
    match ratios.len() {
        3 => {}
        2 => return Ok(()),
        n => {
            return Err(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("DIRECTION must have 2 or 3 direction_ratios, got {n}"),
            });
        }
    }
    let id = ctx.geometry.directions.push(Direction3 {
        x: ratios[0],
        y: ratios[1],
        z: ratios[2],
    });
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `VERTEX_POINT` (resolves `vertex_geometry` through the shared
/// point resolver; a dangling ref surfaces as `MissingReference`). Registers
/// in the `vertex_map` named cache (not `id_cache`), as the legacy handler did.
pub(crate) fn lower_vertex_point(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyVertexPoint,
) -> Result<(), ConvertError> {
    let point = ctx.resolve_point(entity_id, early.vertex_geometry, "vertex_geometry")?;
    let id = ctx.geometry.vertices.push(Vertex { point });
    ctx.vertex_map.insert(entity_id, id);
    Ok(())
}

/// Lower one `VECTOR` (2D sister claims a 2D-direction orientation; a 3D
/// orientation resolves to a `DirectionId`, stored in the `vector_map` named
/// cache as the legacy handler did).
pub(crate) fn lower_vector(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyVector,
) -> Result<(), ConvertError> {
    if ctx
        .id_cache
        .contains::<crate::ir::id::Direction2dId>(early.orientation)
    {
        return Ok(());
    }
    let dir_id = ctx.resolve_direction(entity_id, early.orientation, "orientation")?;
    ctx.vector_map.insert(entity_id, (dir_id, early.magnitude));
    Ok(())
}

/// Lower one `LINE` (2D sister claims a 2D-point pnt; otherwise resolve the
/// point and the VECTOR ref into `Line3`).
pub(crate) fn lower_line(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyLine,
) -> Result<(), ConvertError> {
    if ctx.id_cache.contains::<crate::ir::id::Point2dId>(early.pnt) {
        return Ok(());
    }
    let point = ctx.resolve_point(entity_id, early.pnt, "pnt")?;
    let (direction, magnitude) = ctx.resolve_vector(entity_id, early.dir, "dir")?;
    let id = ctx.geometry.curves.push(Curve::Line(Line3 {
        point,
        direction,
        magnitude,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `AXIS1_PLACEMENT` (location + axis direction). The schema types
/// `axis` OPTIONAL, but the L2 `Axis1Placement.axis` is required and every
/// corpus instance carries it; a missing axis is an Err (the legacy required
/// read errored too — no corpus file triggers it).
pub(crate) fn lower_axis1_placement(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAxis1Placement,
) -> Result<(), ConvertError> {
    let location = ctx.resolve_point(entity_id, early.location, "location")?;
    let Some(axis_ref) = early.axis else {
        return Err(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: "AXIS1_PLACEMENT.axis is required".into(),
        });
    };
    let axis = ctx.resolve_direction(entity_id, axis_ref, "axis")?;
    let id = ctx
        .geometry
        .placements_1d
        .push(Axis1Placement { location, axis });
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `AXIS2_PLACEMENT_3D` (location + optional `axis`/`ref_direction`; 2D
/// sister claims a 2D location). Stored in the `placement_map` named cache.
pub(crate) fn lower_axis2_placement_3d(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyAxis2Placement3d,
) -> Result<(), ConvertError> {
    if ctx
        .id_cache
        .contains::<crate::ir::id::Point2dId>(early.location)
    {
        return Ok(());
    }
    let location = ctx.resolve_point(entity_id, early.location, "location")?;
    let axis = early
        .axis
        .map(|r| ctx.resolve_direction(entity_id, r, "axis"))
        .transpose()?;
    let ref_direction = early
        .ref_direction
        .map(|r| ctx.resolve_direction(entity_id, r, "ref_direction"))
        .transpose()?;
    let id = ctx.geometry.placements.push(Axis2Placement3d {
        location,
        axis,
        ref_direction,
    });
    ctx.placement_map.insert(entity_id, id);
    Ok(())
}

/// Lower one `CIRCLE` (placement + radius; 2D sister claims a 2D placement).
pub(crate) fn lower_circle(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyCircle,
) -> Result<(), ConvertError> {
    if ctx
        .id_cache
        .contains::<crate::ir::id::Placement2dId>(early.position)
    {
        return Ok(());
    }
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx.geometry.curves.push(Curve::Circle(Circle3 {
        position,
        radius: early.radius,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `PLANE` (placement → surface).
pub(crate) fn lower_plane(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyPlane,
) -> Result<(), ConvertError> {
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::Plane(Plane3 { position }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `ELLIPSE` (placement + measures → `Curve::Ellipse`).
pub(crate) fn lower_ellipse(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyEllipse,
) -> Result<(), ConvertError> {
    if ctx
        .id_cache
        .contains::<crate::ir::id::Placement2dId>(early.position)
    {
        return Ok(());
    }
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx.geometry.curves.push(Curve::Ellipse(Ellipse3 {
        position,
        semi_axis_1: early.semi_axis_1,
        semi_axis_2: early.semi_axis_2,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `PARABOLA` (placement + measures → `Curve::Parabola`).
pub(crate) fn lower_parabola(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyParabola,
) -> Result<(), ConvertError> {
    if ctx
        .id_cache
        .contains::<crate::ir::id::Placement2dId>(early.position)
    {
        return Ok(());
    }
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx.geometry.curves.push(Curve::Parabola(Parabola {
        position,
        focal_dist: early.focal_dist,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `HYPERBOLA` (placement + measures → `Curve::Hyperbola`).
pub(crate) fn lower_hyperbola(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyHyperbola,
) -> Result<(), ConvertError> {
    if ctx
        .id_cache
        .contains::<crate::ir::id::Placement2dId>(early.position)
    {
        return Ok(());
    }
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx.geometry.curves.push(Curve::Hyperbola(Hyperbola {
        position,
        semi_axis: early.semi_axis,
        semi_imag_axis: early.semi_imag_axis,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `CONICAL_SURFACE` (placement + measures → `Surface::Cone`).
pub(crate) fn lower_conical_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyConicalSurface,
) -> Result<(), ConvertError> {
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx.geometry.surfaces.push(Surface::Cone(ConicalSurface {
        position,
        radius: early.radius,
        semi_angle: early.semi_angle,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `CYLINDRICAL_SURFACE` (placement + measures → `Surface::Cylinder`).
pub(crate) fn lower_cylindrical_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyCylindricalSurface,
) -> Result<(), ConvertError> {
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::Cylinder(CylindricalSurface {
            position,
            radius: early.radius,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `SPHERICAL_SURFACE` (placement + measures → `Surface::Sphere`).
pub(crate) fn lower_spherical_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlySphericalSurface,
) -> Result<(), ConvertError> {
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::Sphere(SphericalSurface {
            position,
            radius: early.radius,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `TOROIDAL_SURFACE` (placement + measures → `Surface::Torus`).
pub(crate) fn lower_toroidal_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyToroidalSurface,
) -> Result<(), ConvertError> {
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx.geometry.surfaces.push(Surface::Torus(ToroidalSurface {
        position,
        major_radius: early.major_radius,
        minor_radius: early.minor_radius,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}
