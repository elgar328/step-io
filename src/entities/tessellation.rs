//! Tessellation handlers — `COORDINATES_LIST` + `COMPLEX_TRIANGULATED_FACE`
//! (phase tessellation). The first STEP tessellated-geometry support.
//!
//! `COORDINATES_LIST` is a pure scalar/grid leaf; `COMPLEX_TRIANGULATED_FACE`
//! references one by `coordinates`. Both read into their own arenas and
//! emit orphan — no modelled consumer references them yet. A CTF whose
//! `coordinates` ref does not resolve is silently dropped, symmetric on
//! re-read.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::error::ConvertError;
use crate::ir::tessellation::{
    ComplexTriangulatedFace, ComplexTriangulatedSurfaceSet, CoordinatesList,
    RepositionedTessellatedItem, TessellatedCurveSet, TessellatedGeometricSet, TessellatedItemRef,
    TessellatedShell, TessellatedSolid,
};
use crate::parser::entity::{Attribute, RawEntityPart};
use crate::reader::ReaderContext;
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
        _: crate::early::EarlyGraph<'_>,
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
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_complex_triangulated_face(entity_id, attrs)?;
        lower::lower_complex_triangulated_face(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, face: ComplexTriangulatedFace) -> Result<u64, WriteError> {
        let early = lift::lift_complex_triangulated_face(buf, face)?;
        Ok(serialize::serialize_complex_triangulated_face(buf, &early))
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
        _: crate::early::EarlyGraph<'_>,
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
        _: crate::early::EarlyGraph<'_>,
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
        _: crate::early::EarlyGraph<'_>,
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
        _: crate::early::EarlyGraph<'_>,
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
        _: crate::early::EarlyGraph<'_>,
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

pub(crate) struct RepositionedTessellatedItemHandler;

#[step_entity(name = "REPOSITIONED_TESSELLATED_ITEM")]
impl SimpleEntityHandler for RepositionedTessellatedItemHandler {
    type WriteInput = RepositionedTessellatedItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
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
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_repositioned_tessellated_geometric_set(entity_id, parts)?;
        lower::lower_repositioned_tessellated_geometric_set(ctx, entity_id, early);
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _input: Self::WriteInput) -> Result<u64, WriteError> {
        // Emitted by emit_tessellation's container pass, not here.
        unreachable!("RepositionedTessellatedGeometricSet is emitted by emit_tessellation")
    }
}
