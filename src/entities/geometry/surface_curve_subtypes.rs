//! `BOUNDED_SURFACE_CURVE` + `INTERSECTION_CURVE` handlers — phase scs.
//!
//! Separate from the existing `surface_curve.rs` alias path (base
//! `SURFACE_CURVE` unwraps to `curve_3d`). Both subtypes are corpus 0
//! per `ir.toml`; the alias and this arena never overlap.
//!
//! `associated_geometry` SELECT is partial — only the `surface` branch
//! is modelled. The `pcurve` branch awaits a `pcurve` id map on the BPC
//! handler.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{
    PCurveOrSurface, PreferredSurfaceCurveRepresentation, SurfaceCurve, SurfaceCurveData,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct BoundedSurfaceCurveHandler;

#[step_entity(name = "BOUNDED_SURFACE_CURVE", pass = Pass8SurfaceCurveSubtypes)]
impl SimpleEntityHandler for BoundedSurfaceCurveHandler {
    type WriteInput = SurfaceCurveData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "BOUNDED_SURFACE_CURVE")?;
        let Some(body) = read_surface_curve_body(ctx, entity_id, attrs, "BOUNDED_SURFACE_CURVE")?
        else {
            return Ok(());
        };
        ctx.geometry
            .surface_curves
            .push(SurfaceCurve::BoundedSurfaceCurve(body));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, body: SurfaceCurveData) -> Result<u64, WriteError> {
        emit_surface_curve_body(buf, "BOUNDED_SURFACE_CURVE", body)
    }
}

pub(crate) struct IntersectionCurveHandler;

#[step_entity(name = "INTERSECTION_CURVE", pass = Pass8SurfaceCurveSubtypes)]
impl SimpleEntityHandler for IntersectionCurveHandler {
    type WriteInput = SurfaceCurveData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "INTERSECTION_CURVE")?;
        let Some(body) = read_surface_curve_body(ctx, entity_id, attrs, "INTERSECTION_CURVE")?
        else {
            return Ok(());
        };
        ctx.geometry
            .surface_curves
            .push(SurfaceCurve::IntersectionCurve(body));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, body: SurfaceCurveData) -> Result<u64, WriteError> {
        emit_surface_curve_body(buf, "INTERSECTION_CURVE", body)
    }
}

fn read_surface_curve_body(
    ctx: &ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    name: &'static str,
) -> Result<Option<SurfaceCurveData>, ConvertError> {
    let _ = name;
    let sc_name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let curve_3d_ref = read_entity_ref(attrs, 1, entity_id, "curve_3d")?;
    let assoc_refs = read_entity_ref_list(attrs, 2, entity_id, "associated_geometry")?;
    let Attribute::Enum(token) = &attrs[3] else {
        return Ok(None);
    };
    let master_representation = match token.as_str() {
        "CURVE_3D" => PreferredSurfaceCurveRepresentation::Curve3d,
        "PCURVE_S1" => PreferredSurfaceCurveRepresentation::PcurveS1,
        "PCURVE_S2" => PreferredSurfaceCurveRepresentation::PcurveS2,
        _ => return Ok(None),
    };
    let Some(&curve_3d) = ctx.curve_map.get(&curve_3d_ref) else {
        return Ok(None);
    };
    let associated_geometry: Vec<_> = assoc_refs
        .iter()
        .filter_map(|r| {
            ctx.surface_map
                .get(r)
                .copied()
                .map(PCurveOrSurface::Surface)
        })
        .collect();
    if associated_geometry.is_empty() {
        return Ok(None);
    }
    Ok(Some(SurfaceCurveData {
        name: sc_name,
        curve_3d,
        associated_geometry,
        master_representation,
    }))
}

fn emit_surface_curve_body(
    buf: &mut WriteBuffer,
    type_name: &'static str,
    body: SurfaceCurveData,
) -> Result<u64, WriteError> {
    let curve_step = buf.emit_curve(body.curve_3d)?;
    let mut assoc_attrs = Vec::with_capacity(body.associated_geometry.len());
    for item in body.associated_geometry {
        match item {
            PCurveOrSurface::Surface(id) => {
                let step = buf.emit_surface(id)?;
                assoc_attrs.push(Attribute::EntityRef(step));
            }
        }
    }
    let token = match body.master_representation {
        PreferredSurfaceCurveRepresentation::Curve3d => "CURVE_3D",
        PreferredSurfaceCurveRepresentation::PcurveS1 => "PCURVE_S1",
        PreferredSurfaceCurveRepresentation::PcurveS2 => "PCURVE_S2",
    };
    Ok(buf.push_simple(
        type_name,
        vec![
            Attribute::String(body.name),
            Attribute::EntityRef(curve_step),
            Attribute::List(assoc_attrs),
            Attribute::Enum(token.into()),
        ],
    ))
}
