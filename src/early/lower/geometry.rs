//! Geometry-domain `lower` fns (the W-RECURSIVE pilot: leaf points/directions
//! + vertex). See the [module docs](super) for the lowering contract.
//!
//! The 2D/3D `CARTESIAN_POINT` / `DIRECTION` sister handlers both run on the
//! same entity and self-claim by coordinate count; these lowers reproduce the
//! 3D branch exactly (2-count → the 2D arena claims it, so we no-op).

use crate::early::model::{
    EarlyAxis1Placement, EarlyAxis2Placement3d, EarlyCartesianPoint, EarlyCircle,
    EarlyCircularArea, EarlyCompositeCurve, EarlyCompositeCurveSegment, EarlyConicalSurface,
    EarlyCurveBoundedSurface, EarlyCylindricalSurface, EarlyDegenerateToroidalSurface,
    EarlyDirection, EarlyEllipse, EarlyHyperbola, EarlyLine, EarlyOffsetCurve3d,
    EarlyOffsetSurface, EarlyParabola, EarlyPlanarBox, EarlyPlanarExtent, EarlyPlane,
    EarlyPolyline, EarlyRectangularTrimmedSurface, EarlySphericalSurface,
    EarlySurfaceOfLinearExtrusion, EarlySurfaceOfRevolution, EarlyToroidalSurface, EarlyVector,
    EarlyVertexPoint,
};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{
    Axis1Placement, Axis2Placement3d, Circle3, CircularArea, CircularAreaCentre, CompositeCurve,
    CompositeSegment, ConicalSurface, Curve, CurveBoundedSurface, CylindricalSurface,
    DegenerateToroidalSurface, Direction3, Ellipse3, Hyperbola, Line3, OffsetCurve3d, Parabola,
    PlanarBox, PlanarBoxPlacement, PlanarExtent, PlanarExtentData, Plane3, Point3, Polyline,
    RectangularTrimmedSurface, SphericalSurface, Surface, SurfaceOfLinearExtrusion,
    SurfaceOfOffset, SurfaceOfRevolution, ToroidalSurface, Vertex,
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

/// Lower one `SURFACE_OF_REVOLUTION` (swept curve + axis1 placement).
pub(crate) fn lower_surface_of_revolution(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlySurfaceOfRevolution,
) -> Result<(), ConvertError> {
    let swept_curve = ctx.resolve_curve(entity_id, early.swept_curve, "swept_curve")?;
    let axis_placement = ctx.resolve_axis1(entity_id, early.axis_position, "axis_position")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::Revolution(SurfaceOfRevolution {
            swept_curve,
            axis_placement,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `SURFACE_OF_LINEAR_EXTRUSION` (swept curve + extrusion VECTOR
/// resolved to direction + depth).
pub(crate) fn lower_surface_of_linear_extrusion(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlySurfaceOfLinearExtrusion,
) -> Result<(), ConvertError> {
    let swept_curve = ctx.resolve_curve(entity_id, early.swept_curve, "swept_curve")?;
    let (extrusion_direction, depth) =
        ctx.resolve_vector(entity_id, early.extrusion_axis, "extrusion_axis")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::Extrusion(SurfaceOfLinearExtrusion {
            swept_curve,
            extrusion_direction,
            depth,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `POLYLINE` (point list; 2D sister claims when the first point is
/// 2D). Each point resolves through the shared point resolver.
pub(crate) fn lower_polyline(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyPolyline,
) -> Result<(), ConvertError> {
    if let Some(first) = early.points.first()
        && ctx.id_cache.contains::<crate::ir::id::Point2dId>(*first)
    {
        return Ok(());
    }
    let mut points = Vec::with_capacity(early.points.len());
    for r in &early.points {
        points.push(ctx.resolve_point(entity_id, *r, "points")?);
    }
    let id = ctx
        .geometry
        .curves
        .push(Curve::Polyline(Polyline { points }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `PLANAR_EXTENT` (the base form; `PLANAR_BOX` stays on the legacy
/// path). `name` is preserved — the writer re-emits it.
pub(crate) fn lower_planar_extent(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyPlanarExtent,
) {
    let id = ctx
        .geometry
        .planar_extents
        .push(PlanarExtent::Itself(PlanarExtentData {
            name: early.name.clone(),
            size_in_x: early.size_in_x,
            size_in_y: early.size_in_y,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `DEGENERATE_TOROIDAL_SURFACE` (`select_outer` BOOLEAN chooses the
/// outer/inner sheet). `name` is discarded (writer synthesises `''`).
pub(crate) fn lower_degenerate_toroidal_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyDegenerateToroidalSurface,
) -> Result<(), ConvertError> {
    let position = ctx.resolve_placement(entity_id, early.position, "position")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::DegenerateToroidal(DegenerateToroidalSurface {
            position,
            major_radius: early.major_radius,
            minor_radius: early.minor_radius,
            select_outer: early.select_outer,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `CIRCULAR_AREA` — orphan (no inbound refs, no `id_cache` entry).
/// `centre` probes the local `cartesian_point` then the external `REFERENCE`;
/// unresolved drops the carrier.
pub(crate) fn lower_circular_area(
    ctx: &mut ReaderContext,
    _entity_id: u64,
    early: &EarlyCircularArea,
) {
    let centre = if let Some(point) = ctx.id_cache.get::<crate::ir::id::PointId>(early.centre) {
        CircularAreaCentre::Point(point)
    } else if let Some(ext) = ctx
        .id_cache
        .get::<crate::ir::id::ExternalRefId>(early.centre)
    {
        CircularAreaCentre::External(ext)
    } else {
        return;
    };
    ctx.geometry.circular_areas.push(CircularArea {
        name: early.name.clone(),
        centre,
        radius: early.radius,
    });
}

/// Lower one `CURVE_BOUNDED_SURFACE` (`bounded_surface` subtype). `boundaries`
/// narrows `boundary_curve` to generic `CurveId`; an unresolved basis or empty
/// boundary set drops the carrier.
pub(crate) fn lower_curve_bounded_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyCurveBoundedSurface,
) {
    let Some(basis_surface) = ctx
        .id_cache
        .get::<crate::ir::id::SurfaceId>(early.basis_surface)
    else {
        return;
    };
    let boundaries: Vec<_> = early
        .boundaries
        .iter()
        .filter_map(|r| ctx.id_cache.get::<crate::ir::id::CurveId>(*r))
        .collect();
    if boundaries.is_empty() {
        return;
    }
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::CurveBounded(CurveBoundedSurface {
            name: early.name.clone(),
            basis_surface,
            boundaries,
            implicit_outer: early.implicit_outer,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `OFFSET_SURFACE` (wraps a basis surface; `self_intersect` LOGICAL
/// passes through unchanged — L2 keeps the full `Logical`). The dup-guard skips
/// entities already interned through another resolution path.
pub(crate) fn lower_offset_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyOffsetSurface,
) -> Result<(), ConvertError> {
    if ctx.id_cache.contains::<crate::ir::id::SurfaceId>(entity_id) {
        return Ok(());
    }
    let basis = ctx.resolve_surface(entity_id, early.basis_surface, "basis_surface")?;
    let id = ctx.geometry.surfaces.push(Surface::Offset(SurfaceOfOffset {
        basis,
        distance: early.distance,
        self_intersect: early.self_intersect,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `OFFSET_CURVE_3D` (basis curve offset along `ref_direction`).
pub(crate) fn lower_offset_curve_3d(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyOffsetCurve3d,
) -> Result<(), ConvertError> {
    if ctx.id_cache.contains::<crate::ir::id::CurveId>(entity_id) {
        return Ok(());
    }
    let basis = ctx.resolve_curve(entity_id, early.basis_curve, "basis_curve")?;
    let ref_direction = ctx.resolve_direction(entity_id, early.ref_direction, "ref_direction")?;
    let id = ctx
        .geometry
        .curves
        .push(Curve::OffsetCurve3d(OffsetCurve3d {
            basis,
            distance: early.distance,
            self_intersect: early.self_intersect,
            ref_direction,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `PLANAR_BOX` (`PlanarExtent::PlanarBox` variant). `placement` is
/// the `axis2_placement` SELECT — probe the 3D then 2D placement caches; an
/// unresolved placement warns and drops the carrier (verbatim). `name` preserved.
pub(crate) fn lower_planar_box(ctx: &mut ReaderContext, entity_id: u64, early: &EarlyPlanarBox) {
    let placement = if let Some(&id) = ctx.placement_map.get(&early.placement) {
        PlanarBoxPlacement::Placement3d(id)
    } else if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::Placement2dId>(early.placement)
    {
        PlanarBoxPlacement::Placement2d(id)
    } else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "PLANAR_BOX.placement #{} did not resolve to an AXIS2_PLACEMENT",
                early.placement
            ),
        });
        return;
    };
    let id = ctx
        .geometry
        .planar_extents
        .push(PlanarExtent::PlanarBox(PlanarBox {
            name: early.name.clone(),
            size_in_x: early.size_in_x,
            size_in_y: early.size_in_y,
            placement,
        }));
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `RECTANGULAR_TRIMMED_SURFACE` (`bounded_surface` subtype:
/// parameter-space rectangle on a basis surface). Fields map by name from L1;
/// EXPRESS positional order is `u1,u2,v1,v2,usense,vsense` (the prior hand
/// handler transposed `usense` to slot 4 — fixed here via the generated bind).
pub(crate) fn lower_rectangular_trimmed_surface(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyRectangularTrimmedSurface,
) -> Result<(), ConvertError> {
    if ctx.id_cache.contains::<crate::ir::id::SurfaceId>(entity_id) {
        return Ok(());
    }
    let basis = ctx.resolve_surface(entity_id, early.basis_surface, "basis_surface")?;
    let id = ctx
        .geometry
        .surfaces
        .push(Surface::RectangularTrimmed(RectangularTrimmedSurface {
            basis,
            u1: early.u1,
            u2: early.u2,
            usense: early.usense,
            v1: early.v1,
            v2: early.v2,
            vsense: early.vsense,
        }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Lower one `COMPOSITE_CURVE_SEGMENT` — map-only (no arena/`id_cache`). Records
/// the segment metadata (raw `parent_curve` ref) in `composite_segment_map`; the
/// owning `COMPOSITE_CURVE` assembles `CompositeSegment` values from it.
pub(crate) fn lower_composite_curve_segment(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyCompositeCurveSegment,
) {
    ctx.composite_segment_map.insert(
        entity_id,
        (early.transition, early.same_sense, early.parent_curve),
    );
}

/// Lower one `COMPOSITE_CURVE`. Each segment ref resolves through
/// `composite_segment_map` (populated by the segment handler, dispatched first);
/// a missing entry is `MissingReference` (matching the legacy handler).
pub(crate) fn lower_composite_curve(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyCompositeCurve,
) -> Result<(), ConvertError> {
    let mut segments = Vec::with_capacity(early.segments.len());
    for seg_ref in &early.segments {
        let Some(&(transition, same_sense, parent_step_id)) =
            ctx.composite_segment_map.get(seg_ref)
        else {
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: *seg_ref,
                field_name: "segments",
            });
        };
        let parent_curve = ctx.resolve_curve(entity_id, parent_step_id, "parent_curve")?;
        segments.push(CompositeSegment {
            transition,
            same_sense,
            parent_curve,
        });
    }
    let id = ctx.geometry.curves.push(Curve::Composite(CompositeCurve {
        segments,
        self_intersect: early.self_intersect,
    }));
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}
