//! Tessellation handlers — `COORDINATES_LIST` + `COMPLEX_TRIANGULATED_FACE`
//! (phase tessellation). The first STEP tessellated-geometry support.
//!
//! `COORDINATES_LIST` is a pure scalar/grid leaf; `COMPLEX_TRIANGULATED_FACE`
//! references one by `coordinates`. Both read into their own arenas and
//! emit orphan — no modelled consumer references them yet. A CTF whose
//! `coordinates` ref does not resolve is silently dropped, symmetric on
//! re-read.

use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{
    check_count, read_entity_ref, read_entity_ref_list, read_integer, read_integer_grid,
    read_integer_list, read_optional_entity_ref, read_real_grid, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::tessellation::{
    ComplexTriangulatedFace, ComplexTriangulatedSurfaceSet, CoordinatesList,
    RepositionedTessellatedItem, TessellatedCurveSet, TessellatedGeometricSet, TessellatedItem,
    TessellatedItemRef, TessellatedShell, TessellatedSolid,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CoordinatesListHandler;

#[step_entity(name = "COORDINATES_LIST", pass = Pass6CoordinatesList)]
impl SimpleEntityHandler for CoordinatesListHandler {
    type WriteInput = CoordinatesList;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "COORDINATES_LIST")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let npoints = read_integer(attrs, 1, entity_id, "npoints")?;
        let position_coords = read_real_grid(attrs, 2, entity_id, "position_coords")?;

        let id = ctx
            .tessellated_items
            .push(TessellatedItem::CoordinatesList(CoordinatesList {
                name,
                npoints,
                position_coords,
            }));
        ctx.tessellated_item_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, item: CoordinatesList) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "COORDINATES_LIST",
            vec![
                Attribute::String(item.name),
                Attribute::Integer(item.npoints),
                real_grid_attr(&item.position_coords),
            ],
        ))
    }
}

pub(crate) struct ComplexTriangulatedFaceHandler;

#[step_entity(name = "COMPLEX_TRIANGULATED_FACE", pass = Pass6ComplexTriangulatedFace)]
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

        let Some(&coordinates) = ctx.tessellated_item_id_map.get(&coordinates_ref) else {
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
        ctx.tessellated_face_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, face: ComplexTriangulatedFace) -> Result<u64, WriteError> {
        let coordinates_step = buf.tessellated_item_step_ids[face.coordinates.0 as usize];
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

#[step_entity(name = "TESSELLATED_CURVE_SET", pass = Pass6ComplexTriangulatedFace)]
impl SimpleEntityHandler for TessellatedCurveSetHandler {
    type WriteInput = TessellatedCurveSet;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "TESSELLATED_CURVE_SET")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let coordinates_ref = read_entity_ref(attrs, 1, entity_id, "coordinates")?;
        let line_strips = read_integer_grid(attrs, 2, entity_id, "line_strips")?;

        let Some(&coordinates) = ctx.tessellated_item_id_map.get(&coordinates_ref) else {
            return Ok(()); // coordinates_list dropped — drop the curve set too
        };

        let id = ctx
            .tessellated_items
            .push(TessellatedItem::TessellatedCurveSet(TessellatedCurveSet {
                name,
                coordinates,
                line_strips,
            }));
        ctx.tessellated_item_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, item: TessellatedCurveSet) -> Result<u64, WriteError> {
        let coordinates_step = buf.tessellated_item_step_ids[item.coordinates.0 as usize];
        Ok(buf.push_simple(
            "TESSELLATED_CURVE_SET",
            vec![
                Attribute::String(item.name),
                Attribute::EntityRef(coordinates_step),
                integer_grid_attr(&item.line_strips),
            ],
        ))
    }
}

pub(crate) struct ComplexTriangulatedSurfaceSetHandler;

#[step_entity(name = "COMPLEX_TRIANGULATED_SURFACE_SET", pass = Pass6ComplexTriangulatedFace)]
impl SimpleEntityHandler for ComplexTriangulatedSurfaceSetHandler {
    type WriteInput = ComplexTriangulatedSurfaceSet;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 7, entity_id, "COMPLEX_TRIANGULATED_SURFACE_SET")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let coordinates_ref = read_entity_ref(attrs, 1, entity_id, "coordinates")?;
        let pnmax = read_integer(attrs, 2, entity_id, "pnmax")?;
        let normals = read_real_grid(attrs, 3, entity_id, "normals")?;
        let pnindex = read_integer_list(attrs, 4, entity_id, "pnindex")?;
        let triangle_strips = read_integer_grid(attrs, 5, entity_id, "triangle_strips")?;
        let triangle_fans = read_integer_grid(attrs, 6, entity_id, "triangle_fans")?;

        let Some(&coordinates) = ctx.tessellated_item_id_map.get(&coordinates_ref) else {
            return Ok(()); // coordinates_list dropped — drop the surface set too
        };

        let id = ctx
            .tessellated_surface_sets
            .push(ComplexTriangulatedSurfaceSet {
                name,
                coordinates,
                pnmax,
                normals,
                pnindex,
                triangle_strips,
                triangle_fans,
            });
        ctx.tessellated_surface_set_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, set: ComplexTriangulatedSurfaceSet) -> Result<u64, WriteError> {
        let coordinates_step = buf.tessellated_item_step_ids[set.coordinates.0 as usize];
        Ok(buf.push_simple(
            "COMPLEX_TRIANGULATED_SURFACE_SET",
            vec![
                Attribute::String(set.name),
                Attribute::EntityRef(coordinates_step),
                Attribute::Integer(set.pnmax),
                real_grid_attr(&set.normals),
                integer_list_attr(&set.pnindex),
                integer_grid_attr(&set.triangle_strips),
                integer_grid_attr(&set.triangle_fans),
            ],
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
    if let Some(&id) = ctx.tessellated_item_id_map.get(&item_ref) {
        return Some(TessellatedItemRef::Item(id));
    }
    if let Some(&id) = ctx.tessellated_face_id_map.get(&item_ref) {
        return Some(TessellatedItemRef::Face(id));
    }
    if let Some(&id) = ctx.tessellated_surface_set_id_map.get(&item_ref) {
        return Some(TessellatedItemRef::SurfaceSet(id));
    }
    None
}

pub(crate) struct TessellatedGeometricSetHandler;

#[step_entity(name = "TESSELLATED_GEOMETRIC_SET", pass = Pass6TessellatedGeometricSet)]
impl SimpleEntityHandler for TessellatedGeometricSetHandler {
    type WriteInput = TessellatedGeometricSet;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "TESSELLATED_GEOMETRIC_SET")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let child_refs = read_entity_ref_list(attrs, 1, entity_id, "children")?;
        // Unresolved children (targets step-io does not model) are dropped
        // from the set — symmetric on re-read.
        let children: Vec<TessellatedItemRef> = child_refs
            .iter()
            .filter_map(|&r| resolve_tessellated_item_ref(ctx, r))
            .collect();

        let id = ctx
            .tessellated_items
            .push(TessellatedItem::TessellatedGeometricSet(
                TessellatedGeometricSet { name, children },
            ));
        ctx.tessellated_item_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, tgs: TessellatedGeometricSet) -> Result<u64, WriteError> {
        let children: Vec<Attribute> = tgs
            .children
            .iter()
            .map(|&r| Attribute::EntityRef(buf.emit_tessellated_item_ref(r)))
            .collect();
        Ok(buf.push_simple(
            "TESSELLATED_GEOMETRIC_SET",
            vec![Attribute::String(tgs.name), Attribute::List(children)],
        ))
    }
}

pub(crate) struct TessellatedSolidHandler;

#[step_entity(name = "TESSELLATED_SOLID", pass = Pass6TessellatedGeometricSet)]
impl SimpleEntityHandler for TessellatedSolidHandler {
    type WriteInput = TessellatedSolid;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "TESSELLATED_SOLID")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let geometric_link_ref = read_optional_entity_ref(attrs, 2, entity_id, "geometric_link")?;

        let items: Vec<TessellatedItemRef> = item_refs
            .iter()
            .filter_map(|&r| resolve_tessellated_item_ref(ctx, r))
            .collect();
        let geometric_link = geometric_link_ref.and_then(|r| ctx.solid_map.get(&r).copied());

        let id = ctx
            .tessellated_items
            .push(TessellatedItem::TessellatedSolid(TessellatedSolid {
                name,
                items,
                geometric_link,
            }));
        ctx.tessellated_item_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ts: TessellatedSolid) -> Result<u64, WriteError> {
        let items: Vec<Attribute> = ts
            .items
            .iter()
            .map(|&r| Attribute::EntityRef(buf.emit_tessellated_item_ref(r)))
            .collect();
        let geometric_link = match ts.geometric_link {
            Some(id) => Attribute::EntityRef(buf.emit_solid(id)?),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "TESSELLATED_SOLID",
            vec![
                Attribute::String(ts.name),
                Attribute::List(items),
                geometric_link,
            ],
        ))
    }
}

pub(crate) struct TessellatedShellHandler;

#[step_entity(name = "TESSELLATED_SHELL", pass = Pass6TessellatedGeometricSet)]
impl SimpleEntityHandler for TessellatedShellHandler {
    type WriteInput = TessellatedShell;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "TESSELLATED_SHELL")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let topological_link_ref =
            read_optional_entity_ref(attrs, 2, entity_id, "topological_link")?;

        let items: Vec<TessellatedItemRef> = item_refs
            .iter()
            .filter_map(|&r| resolve_tessellated_item_ref(ctx, r))
            .collect();
        let topological_link = topological_link_ref.and_then(|r| ctx.shell_map.get(&r).copied());

        let id = ctx
            .tessellated_items
            .push(TessellatedItem::TessellatedShell(TessellatedShell {
                name,
                items,
                topological_link,
            }));
        ctx.tessellated_item_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ts: TessellatedShell) -> Result<u64, WriteError> {
        let items: Vec<Attribute> = ts
            .items
            .iter()
            .map(|&r| Attribute::EntityRef(buf.emit_tessellated_item_ref(r)))
            .collect();
        let topological_link = match ts.topological_link {
            Some(id) => Attribute::EntityRef(buf.emit_shell(id)?),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "TESSELLATED_SHELL",
            vec![
                Attribute::String(ts.name),
                Attribute::List(items),
                topological_link,
            ],
        ))
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

#[step_entity(name = "REPOSITIONED_TESSELLATED_ITEM", pass = Pass6TessellatedGeometricSet)]
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
        check_count(attrs, 2, entity_id, "REPOSITIONED_TESSELLATED_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let location_ref = read_entity_ref(attrs, 1, entity_id, "location")?;
        let Some(&location) = ctx.placement_map.get(&location_ref) else {
            return Ok(());
        };
        let id = ctx
            .tessellated_items
            .push(TessellatedItem::RepositionedTessellatedItem(
                RepositionedTessellatedItem { name, location },
            ));
        ctx.tessellated_item_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, r: RepositionedTessellatedItem) -> Result<u64, WriteError> {
        let placement_ref = buf.emit_axis2_placement_3d(r.location)?;
        Ok(buf.push_simple(
            "REPOSITIONED_TESSELLATED_ITEM",
            vec![
                Attribute::String(r.name),
                Attribute::EntityRef(placement_ref),
            ],
        ))
    }
}
