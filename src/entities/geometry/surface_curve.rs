//! `SURFACE_CURVE` / `SEAM_CURVE` handlers — 2-layer arena path.
//!
//! The base `surface_curve` family (`SURFACE_CURVE` / `SEAM_CURVE` /
//! `BOUNDED_SURFACE_CURVE` / `INTERSECTION_CURVE`) shares one `surface_curves`
//! arena; an [`Edge`](crate::ir::topology::Edge) references its surface curve
//! through [`EdgeGeometry`](crate::ir::topology::EdgeGeometry). The base/seam
//! `read` additionally registers a `CurveId` alias (the underlying `curve_3d`)
//! so the generic curve-ref consumers that may point at a `SURFACE_CURVE`
//! resolve unchanged. `resolve_pcurve` (a graph walk used by the shared
//! [`lower`](crate::early::lower)) lives here.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Pcurve, SurfaceCurveData};
use crate::parser::entity::{Attribute, EntityGraph, RawEntity};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

impl ReaderContext {
    /// Resolve a `PCURVE` member of a `SURFACE_CURVE` / `SEAM_CURVE`
    /// `associated_geometry` SET into a [`Pcurve`]. Walks
    /// `PCURVE → basis_surface + DEFINITIONAL_REPRESENTATION → items[0] → 2D
    /// curve`; the `DEFINITIONAL_REPRESENTATION` is traversed but not stored in
    /// IR. Returns `None` when any link is missing or a referenced entity does
    /// not resolve, in which case the caller emits a warning so the dropped
    /// pcurve stays visible in reader diagnostics.
    pub(crate) fn resolve_pcurve(&self, pcurve_ref: u64, graph: &EntityGraph) -> Option<Pcurve> {
        let RawEntity::Simple {
            name, attributes, ..
        } = graph.get(pcurve_ref)?
        else {
            return None;
        };
        if name != "PCURVE" {
            return None;
        }
        let pc = bind::bind_pcurve(pcurve_ref, attributes).ok()?;
        let basis_surface = self
            .id_cache
            .get::<crate::ir::id::SurfaceId>(pc.basis_surface)?;

        let RawEntity::Simple {
            name: def_name,
            attributes: def_attrs,
            ..
        } = graph.get(pc.reference_to_curve)?
        else {
            return None;
        };
        if def_name != "DEFINITIONAL_REPRESENTATION" {
            return None;
        }
        let def = bind::bind_definitional_representation(pc.reference_to_curve, def_attrs).ok()?;
        let curve_2d = self
            .id_cache
            .get::<crate::ir::id::Curve2dId>(def.items.first().copied()?)?;

        Some(Pcurve {
            basis_surface,
            curve_2d,
        })
    }
}

pub(crate) struct SurfaceCurveHandler;

#[step_entity(name = "SURFACE_CURVE")]
impl SimpleEntityHandler for SurfaceCurveHandler {
    type WriteInput = SurfaceCurveData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_surface_curve(entity_id, attrs)?;
        lower::lower_surface_curve(ctx, entity_id, early, graph);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, body: SurfaceCurveData) -> Result<u64, WriteError> {
        let early = lift::lift_surface_curve(buf, body)?;
        Ok(serialize::serialize_surface_curve(buf, &early))
    }
}
