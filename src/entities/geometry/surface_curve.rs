//! `SURFACE_CURVE` handler — Pass 4-3 (transparent alias to a 3D curve
//! with associated pcurves resolved in a post-pass).
//!
//! Shares its read body and write body with `SEAM_CURVE` via the
//! `read_surface_or_seam_curve_body` / `write_surface_or_seam_curve_body`
//! helpers below; the sister handler in `seam_curve.rs` imports them.
//! The post-pass that collects pcurves into
//! `surface_curve_pcurves_map` (formerly `ReaderContext::collect_surface_curve_pcurves`)
//! also lives here so the entity module is self-contained.

use crate::entities::SimpleEntityHandler;
use crate::ir::Pcurve;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

/// Reader body shared by `SURFACE_CURVE` and `SEAM_CURVE`. Both alias
/// the entity id to its underlying 3D curve in `curve_map`; pcurves are
/// resolved separately by `collect_surface_curve_pcurves`.
pub(super) fn read_surface_or_seam_curve_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    tag: &'static str,
) -> Result<(), ConvertError> {
    check_count(attrs, 4, entity_id, tag)?;
    let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
    let curve_3d_ref = read_entity_ref(attrs, 1, entity_id, "curve_3d")?;
    // attrs[2] = associated_geometry (pcurves) — resolved separately by
    // `collect_surface_curve_pcurves` once the 2D arenas are populated.
    // attrs[3] = master_representation enum — intentionally ignored;
    // the writer reproduces OCCT's `.PCURVE_S1.` convention.

    let curve_3d = ctx.resolve_curve(entity_id, curve_3d_ref, "curve_3d")?;
    ctx.curve_map.insert(entity_id, curve_3d);
    Ok(())
}

/// Read the `associated_geometry` list off a `SURFACE_CURVE` /
/// `SEAM_CURVE` entity and stash resolved [`Pcurve`]s into
/// `surface_curve_pcurves_map`. Lives outside [`SimpleEntityHandler`]
/// because pcurve resolution requires the [`EntityGraph`] (Pass 4a must
/// have already populated `curve_2d_map`/`surface_map`).
pub(crate) fn collect_surface_curve_pcurves(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    graph: &EntityGraph,
) {
    let Ok(pcurve_refs) = read_entity_ref_list(attrs, 2, entity_id, "associated_geometry") else {
        return;
    };
    let mut pcurves = Vec::with_capacity(pcurve_refs.len());
    for &pcurve_ref in &pcurve_refs {
        if let Some(pc) = ctx.resolve_pcurve(pcurve_ref, graph) {
            pcurves.push(pc);
        }
    }
    if !pcurves.is_empty() {
        ctx.surface_curve_pcurves_map.insert(entity_id, pcurves);
    }
}

/// Writer body shared by `SURFACE_CURVE` and `SEAM_CURVE`. Caller
/// already emitted the underlying 3D curve and supplies the pcurve list.
/// `is_seam` selects the entity name; the body is otherwise identical.
pub(super) fn write_surface_or_seam_curve_body(
    buf: &mut WriteBuffer,
    curve_3d_ref: u64,
    pcurves: &[Pcurve],
    is_seam: bool,
) -> Result<u64, WriteError> {
    let mut pcurve_refs = Vec::with_capacity(pcurves.len());
    for pc in pcurves {
        pcurve_refs.push(buf.emit_pcurve(*pc)?);
    }
    let name = if is_seam {
        "SEAM_CURVE"
    } else {
        "SURFACE_CURVE"
    };
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Simple {
            name: name.into(),
            attrs: vec![
                Attribute::String(String::new()),
                Attribute::EntityRef(curve_3d_ref),
                Attribute::List(pcurve_refs.into_iter().map(Attribute::EntityRef).collect()),
                Attribute::Enum("PCURVE_S1".into()),
            ],
        },
    });
    Ok(n)
}

pub(crate) struct SurfaceCurveHandler;

#[step_entity(name = "SURFACE_CURVE", pass = Pass4_3SurfaceCurve)]
impl SimpleEntityHandler for SurfaceCurveHandler {
    /// `(curve_3d_ref, pcurves)` — caller (writer wrapper) already
    /// emitted the underlying 3D curve and hands over the resolved
    /// pcurve list.
    type WriteInput = (u64, Vec<Pcurve>);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_surface_or_seam_curve_body(ctx, entity_id, attrs, "SURFACE_CURVE")
    }

    fn write(
        buf: &mut WriteBuffer,
        (curve_3d_ref, pcurves): (u64, Vec<Pcurve>),
    ) -> Result<u64, WriteError> {
        write_surface_or_seam_curve_body(buf, curve_3d_ref, &pcurves, false)
    }
}
