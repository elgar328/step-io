//! Tessellation handlers — `COORDINATES_LIST` + `COMPLEX_TRIANGULATED_FACE`
//! (phase tessellation). The first STEP tessellated-geometry support.
//!
//! `COORDINATES_LIST` is a pure scalar/grid leaf; `COMPLEX_TRIANGULATED_FACE`
//! references one by `coordinates`. Both read into their own arenas and
//! emit orphan — no modelled consumer references them yet. A CTF whose
//! `coordinates` ref does not resolve is silently dropped, symmetric on
//! re-read.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::attr::{
    check_count, read_entity_ref, read_entity_ref_list, read_integer, read_integer_grid,
    read_integer_list, read_optional_entity_ref, read_real_grid, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::tessellation::{
    ComplexTriangulatedFace, ComplexTriangulatedSurfaceSet, CoordinatesList,
    RepositionedTessellatedGeometricSet, RepositionedTessellatedItem, TessellatedCurveSet,
    TessellatedGeometricSet, TessellatedItem, TessellatedItemRef, TessellatedShell,
    TessellatedSolid,
};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::{step_entity, step_entity_complex};

pub(crate) struct CoordinatesListHandler;

#[step_entity(name = "COORDINATES_LIST")]
impl SimpleEntityHandler for CoordinatesListHandler {
    type WriteInput = CoordinatesList;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_coordinates_list(entity_id, attrs)?;
        lower::lower_coordinates_list(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, item: CoordinatesList) -> Result<u64, WriteError> {
        Ok(serialize::serialize_coordinates_list(
            buf,
            &lift::lift_coordinates_list(item),
        ))
    }
}

pub(crate) struct ComplexTriangulatedFaceHandler;

#[step_entity(name = "COMPLEX_TRIANGULATED_FACE")]
impl SimpleEntityHandler for ComplexTriangulatedFaceHandler {
    type WriteInput = ComplexTriangulatedFace;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 8, entity_id, "COMPLEX_TRIANGULATED_FACE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let coordinates_ref = read_entity_ref(attrs, 1, entity_id, "coordinates")?;
        let pnmax = read_integer(attrs, 2, entity_id, "pnmax")?;
        let normals = read_real_grid(attrs, 3, entity_id, "normals")?;
        let geometric_link_ref = read_optional_entity_ref(attrs, 4, entity_id, "geometric_link")?;
        let pnindex = read_integer_list(attrs, 5, entity_id, "pnindex")?;
        let triangle_strips = read_integer_grid(attrs, 6, entity_id, "triangle_strips")?;
        let triangle_fans = read_integer_grid(attrs, 7, entity_id, "triangle_fans")?;

        let Some(coordinates) = ctx
            .id_cache
            .get::<crate::ir::id::TessellatedItemId>(coordinates_ref)
        else {
            return Ok(()); // coordinates_list dropped — drop the face too
        };
        let geometric_link =
            geometric_link_ref.and_then(|r| resolve_representation_item_ref(ctx, r));

        let id = ctx.tessellated_faces.push(ComplexTriangulatedFace {
            name,
            coordinates,
            pnmax,
            normals,
            geometric_link,
            pnindex,
            triangle_strips,
            triangle_fans,
        });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, face: ComplexTriangulatedFace) -> Result<u64, WriteError> {
        let coordinates_step = buf.step_id(face.coordinates);
        let geometric_link = match face.geometric_link {
            Some(link) => Attribute::EntityRef(buf.emit_representation_item_ref(link)?),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "COMPLEX_TRIANGULATED_FACE",
            vec![
                Attribute::String(face.name),
                Attribute::EntityRef(coordinates_step),
                Attribute::Integer(face.pnmax),
                real_grid_attr(&face.normals),
                geometric_link,
                integer_list_attr(&face.pnindex),
                integer_grid_attr(&face.triangle_strips),
                integer_grid_attr(&face.triangle_fans),
            ],
        ))
    }
}

pub(crate) struct TessellatedCurveSetHandler;

#[step_entity(name = "TESSELLATED_CURVE_SET")]
impl SimpleEntityHandler for TessellatedCurveSetHandler {
    type WriteInput = TessellatedCurveSet;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_tessellated_curve_set(entity_id, attrs)?;
        lower::lower_tessellated_curve_set(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, item: TessellatedCurveSet) -> Result<u64, WriteError> {
        Ok(serialize::serialize_tessellated_curve_set(
            buf,
            &lift::lift_tessellated_curve_set(buf, item),
        ))
    }
}

pub(crate) struct ComplexTriangulatedSurfaceSetHandler;

#[step_entity(name = "COMPLEX_TRIANGULATED_SURFACE_SET")]
impl SimpleEntityHandler for ComplexTriangulatedSurfaceSetHandler {
    type WriteInput = ComplexTriangulatedSurfaceSet;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_complex_triangulated_surface_set(entity_id, attrs)?;
        lower::lower_complex_triangulated_surface_set(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, set: ComplexTriangulatedSurfaceSet) -> Result<u64, WriteError> {
        Ok(serialize::serialize_complex_triangulated_surface_set(
            buf,
            &lift::lift_complex_triangulated_surface_set(buf, set),
        ))
    }
}

/// Resolve a STEP `tessellated_item` reference into a [`TessellatedItemRef`]
/// by probing the three tessellation arena id maps. Returns `None` for a
/// target step-io does not model.
pub(crate) fn resolve_tessellated_item_ref(
    ctx: &ReaderContext,
    item_ref: u64,
) -> Option<TessellatedItemRef> {
    if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::TessellatedItemId>(item_ref)
    {
        return Some(TessellatedItemRef::Item(id));
    }
    if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::TessellatedFaceId>(item_ref)
    {
        return Some(TessellatedItemRef::Face(id));
    }
    if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::TessellatedSurfaceSetId>(item_ref)
    {
        return Some(TessellatedItemRef::SurfaceSet(id));
    }
    None
}

pub(crate) struct TessellatedGeometricSetHandler;

#[step_entity(name = "TESSELLATED_GEOMETRIC_SET")]
impl SimpleEntityHandler for TessellatedGeometricSetHandler {
    type WriteInput = TessellatedGeometricSet;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_tessellated_geometric_set(entity_id, attrs)?;
        lower::lower_tessellated_geometric_set(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, tgs: TessellatedGeometricSet) -> Result<u64, WriteError> {
        Ok(serialize::serialize_tessellated_geometric_set(
            buf,
            &lift::lift_tessellated_geometric_set(buf, tgs),
        ))
    }
}

pub(crate) struct TessellatedSolidHandler;

#[step_entity(name = "TESSELLATED_SOLID")]
impl SimpleEntityHandler for TessellatedSolidHandler {
    type WriteInput = TessellatedSolid;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_tessellated_solid(entity_id, attrs)?;
        lower::lower_tessellated_solid(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ts: TessellatedSolid) -> Result<u64, WriteError> {
        let early = lift::lift_tessellated_solid(buf, ts)?;
        Ok(serialize::serialize_tessellated_solid(buf, &early))
    }
}

pub(crate) struct TessellatedShellHandler;

#[step_entity(name = "TESSELLATED_SHELL")]
impl SimpleEntityHandler for TessellatedShellHandler {
    type WriteInput = TessellatedShell;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_tessellated_shell(entity_id, attrs)?;
        lower::lower_tessellated_shell(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ts: TessellatedShell) -> Result<u64, WriteError> {
        let early = lift::lift_tessellated_shell(buf, ts)?;
        Ok(serialize::serialize_tessellated_shell(buf, &early))
    }
}

/// `Vec<Vec<f64>>` → a nested-list `Attribute`.
fn real_grid_attr(grid: &[Vec<f64>]) -> Attribute {
    Attribute::List(
        grid.iter()
            .map(|row| Attribute::List(row.iter().map(|&v| Attribute::Real(v)).collect()))
            .collect(),
    )
}

/// `Vec<Vec<i64>>` → a nested-list `Attribute`.
fn integer_grid_attr(grid: &[Vec<i64>]) -> Attribute {
    Attribute::List(
        grid.iter()
            .map(|row| Attribute::List(row.iter().map(|&v| Attribute::Integer(v)).collect()))
            .collect(),
    )
}

/// `&[i64]` → a flat-list `Attribute`.
fn integer_list_attr(list: &[i64]) -> Attribute {
    Attribute::List(list.iter().map(|&v| Attribute::Integer(v)).collect())
}

pub(crate) struct RepositionedTessellatedItemHandler;

#[step_entity(name = "REPOSITIONED_TESSELLATED_ITEM")]
impl SimpleEntityHandler for RepositionedTessellatedItemHandler {
    type WriteInput = RepositionedTessellatedItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // 2 attrs: `name` inherited from `representation_item` (flattened
        // into the part), `location` ref to AXIS2_PLACEMENT_3D. Same
        // pattern as TESSELLATED_GEOMETRIC_SET above.
        let early = bind::bind_repositioned_tessellated_item(entity_id, attrs)?;
        lower::lower_repositioned_tessellated_item(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, r: RepositionedTessellatedItem) -> Result<u64, WriteError> {
        let early = lift::lift_repositioned_tessellated_item(buf, r)?;
        Ok(serialize::serialize_repositioned_tessellated_item(
            buf, &early,
        ))
    }
}

pub(crate) struct RepositionedTessellatedGeometricSetHandler;

/// `(GEOMETRIC_REPRESENTATION_ITEM REPOSITIONED_TESSELLATED_ITEM
/// REPRESENTATION_ITEM TESSELLATED_GEOMETRIC_SET TESSELLATED_ITEM)` complex MI
/// — a PMI annotation occurrence's `item`. Only the simple subtype names had
/// handlers, so the complex was silently skipped; the writer re-emits the
/// five-part form. Each part carries only its own (non-inherited) attributes:
/// `name` lives in `REPRESENTATION_ITEM`, `location` in
/// `REPOSITIONED_TESSELLATED_ITEM`, `children` in `TESSELLATED_GEOMETRIC_SET`.
#[step_entity_complex(
    name = "TESSELLATED_GEOMETRIC_SET",
    cases = [[
        "GEOMETRIC_REPRESENTATION_ITEM",
        "REPOSITIONED_TESSELLATED_ITEM",
        "REPRESENTATION_ITEM",
        "TESSELLATED_GEOMETRIC_SET",
        "TESSELLATED_ITEM"
    ]]
)]
impl ComplexEntityHandler for RepositionedTessellatedGeometricSetHandler {
    type WriteInput = ();

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let name_attrs = require_part_attrs(parts, "REPRESENTATION_ITEM", entity_id)?;
        let name = read_string_or_unset(name_attrs, 0, entity_id, "name")?.to_owned();
        let rti_attrs = require_part_attrs(parts, "REPOSITIONED_TESSELLATED_ITEM", entity_id)?;
        let location_ref = read_entity_ref(rti_attrs, 0, entity_id, "location")?;
        let Some(&location) = ctx.placement_map.get(&location_ref) else {
            return Ok(());
        };
        let tgs_attrs = require_part_attrs(parts, "TESSELLATED_GEOMETRIC_SET", entity_id)?;
        let child_refs = read_entity_ref_list(tgs_attrs, 0, entity_id, "children")?;
        let children: Vec<TessellatedItemRef> = child_refs
            .iter()
            .filter_map(|&r| resolve_tessellated_item_ref(ctx, r))
            .collect();

        let id = ctx
            .tessellated_items
            .push(TessellatedItem::RepositionedTessellatedGeometricSet(
                RepositionedTessellatedGeometricSet {
                    name,
                    location,
                    children,
                },
            ));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _input: Self::WriteInput) -> Result<u64, WriteError> {
        // Emitted by emit_tessellation's container pass, not here.
        unreachable!("RepositionedTessellatedGeometricSet is emitted by emit_tessellation")
    }
}
