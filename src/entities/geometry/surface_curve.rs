//! `SURFACE_CURVE` handler — Pass 4-3 (transparent alias to a 3D curve
//! with the wrapper preserved in a post-pass).
//!
//! Shares its read body and write body with `SEAM_CURVE` via the
//! `read_surface_or_seam_curve_body` / `write_surface_or_seam_curve_body`
//! helpers below; the sister handler in `seam_curve.rs` imports them.
//! The post-pass that captures each wrapper into `surface_curve_map`
//! (`collect_surface_curve`) also lives here so the entity module is
//! self-contained.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Pcurve;
use crate::ir::{PCurveOrSurface, PreferredSurfaceCurveRepresentation, SurfaceCurveWrapper};
use crate::parser::entity::{Attribute, EntityGraph, RawEntity};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

/// Reader body shared by `SURFACE_CURVE` and `SEAM_CURVE`. Both alias
/// the entity id to its underlying 3D curve in `curve_map`; pcurves are
/// resolved separately by `collect_surface_curve`.
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
    // `collect_surface_curve` once the 2D arenas are populated.
    // attrs[3] = master_representation enum — intentionally ignored;
    // the writer reproduces OCCT's `.PCURVE_S1.` convention.

    let curve_3d = ctx.resolve_curve(entity_id, curve_3d_ref, "curve_3d")?;
    ctx.curve_map.insert(entity_id, curve_3d);
    Ok(())
}

/// Parse a `preferred_surface_curve_representation` enum token. Shared with
/// the subtype path (`surface_curve_subtypes.rs`).
pub(crate) fn parse_master_representation(
    token: &str,
) -> Option<PreferredSurfaceCurveRepresentation> {
    match token {
        "CURVE_3D" => Some(PreferredSurfaceCurveRepresentation::Curve3d),
        "PCURVE_S1" => Some(PreferredSurfaceCurveRepresentation::PcurveS1),
        "PCURVE_S2" => Some(PreferredSurfaceCurveRepresentation::PcurveS2),
        _ => None,
    }
}

/// Serialize a `preferred_surface_curve_representation` back to its token.
pub(crate) fn master_representation_token(m: PreferredSurfaceCurveRepresentation) -> &'static str {
    match m {
        PreferredSurfaceCurveRepresentation::Curve3d => "CURVE_3D",
        PreferredSurfaceCurveRepresentation::PcurveS1 => "PCURVE_S1",
        PreferredSurfaceCurveRepresentation::PcurveS2 => "PCURVE_S2",
    }
}

/// Capture a `SURFACE_CURVE` / `SEAM_CURVE` wrapper into `surface_curve_map`,
/// resolving its `associated_geometry` members and preserving the entity kind
/// (`is_seam`), `name`, and `master_representation` so the writer reproduces
/// them verbatim. Lives outside [`SimpleEntityHandler`] because member
/// resolution requires the [`EntityGraph`] (Pass 4a must have already
/// populated `curve_2d_map`/`surface_map`).
pub(crate) fn collect_surface_curve(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    graph: &EntityGraph,
    is_seam: bool,
) {
    let name = match read_string_or_unset(attrs, 0, entity_id, "name") {
        Ok(s) => s.to_owned(),
        Err(e) => {
            ctx.warnings.push(e);
            return;
        }
    };
    let member_refs = match read_entity_ref_list(attrs, 2, entity_id, "associated_geometry") {
        Ok(refs) => refs,
        Err(e) => {
            ctx.warnings.push(e);
            return;
        }
    };
    let master_representation = match attrs.get(3) {
        Some(Attribute::Enum(tok)) => parse_master_representation(tok),
        _ => None,
    }
    .unwrap_or(PreferredSurfaceCurveRepresentation::PcurveS1);

    let mut members = Vec::with_capacity(member_refs.len());
    for &member_ref in &member_refs {
        // `associated_geometry` is a `pcurve_or_surface` SELECT: a member is
        // either a PCURVE (resolve through its definitional 2D curve) or a
        // surface directly associated with the 3D curve.
        let member_name = match graph.get(member_ref) {
            Some(RawEntity::Simple { name, .. }) => name.as_str(),
            _ => "",
        };
        if member_name == "PCURVE" {
            match ctx.resolve_pcurve(member_ref, graph) {
                Some(pc) => members.push(PCurveOrSurface::Pcurve(pc)),
                None => ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "SURFACE_CURVE.associated_geometry PCURVE #{member_ref} unresolved \
                     (definitional curve not 2D, or basis_surface missing)"
                    ),
                }),
            }
        } else if let Some(&surface_id) = ctx.surface_map.get(&member_ref) {
            members.push(PCurveOrSurface::Surface(surface_id));
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "SURFACE_CURVE.associated_geometry #{member_ref} ({member_name}) unresolved \
                     (neither a resolvable PCURVE nor a known surface)"
                ),
            });
        }
    }
    if !members.is_empty() {
        ctx.surface_curve_map.insert(
            entity_id,
            SurfaceCurveWrapper {
                name,
                is_seam,
                associated_geometry: members,
                master_representation,
            },
        );
    }
}

impl ReaderContext {
    /// Resolve a `PCURVE` member of a `SURFACE_CURVE` / `SEAM_CURVE`
    /// `associated_geometry` SET into a [`Pcurve`]. Walks
    /// `PCURVE → basis_surface + DEFINITIONAL_REPRESENTATION → items[0] → 2D
    /// curve`; the `DEFINITIONAL_REPRESENTATION` is traversed but not stored in
    /// IR. Returns `None` when any link is missing or a referenced entity does
    /// not resolve, in which case [`collect_surface_curve`] emits a warning so
    /// the dropped pcurve stays visible in reader diagnostics.
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
        let Attribute::EntityRef(basis_surface_ref) = attributes.get(1)? else {
            return None;
        };
        let Attribute::EntityRef(def_repr_ref) = attributes.get(2)? else {
            return None;
        };

        let basis_surface = *self.surface_map.get(basis_surface_ref)?;

        let RawEntity::Simple {
            name: def_name,
            attributes: def_attrs,
            ..
        } = graph.get(*def_repr_ref)?
        else {
            return None;
        };
        if def_name != "DEFINITIONAL_REPRESENTATION" {
            return None;
        }
        let Attribute::List(items) = def_attrs.get(1)? else {
            return None;
        };
        let Attribute::EntityRef(first_item_ref) = items.first()? else {
            return None;
        };
        let curve_2d = *self.curve_2d_map.get(first_item_ref)?;

        Some(Pcurve {
            basis_surface,
            curve_2d,
        })
    }
}

/// Writer body shared by `SURFACE_CURVE` and `SEAM_CURVE`. Caller already
/// emitted the underlying 3D curve and supplies the preserved wrapper. The
/// entity kind comes from `wrapper.is_seam`; `name` and `master_representation`
/// are reproduced verbatim (no heuristic, no hardcoded token).
pub(super) fn write_surface_or_seam_curve_body(
    buf: &mut WriteBuffer,
    curve_3d_ref: u64,
    wrapper: &SurfaceCurveWrapper,
) -> Result<u64, WriteError> {
    let mut member_refs = Vec::with_capacity(wrapper.associated_geometry.len());
    for member in &wrapper.associated_geometry {
        let r = match member {
            PCurveOrSurface::Pcurve(pc) => buf.emit_pcurve(*pc)?,
            PCurveOrSurface::Surface(sid) => buf.emit_surface(*sid)?,
        };
        member_refs.push(r);
    }
    let name = if wrapper.is_seam {
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
                Attribute::String(wrapper.name.clone()),
                Attribute::EntityRef(curve_3d_ref),
                Attribute::List(member_refs.into_iter().map(Attribute::EntityRef).collect()),
                Attribute::Enum(master_representation_token(wrapper.master_representation).into()),
            ],
        },
    });
    Ok(n)
}

pub(crate) struct SurfaceCurveHandler;

#[step_entity(name = "SURFACE_CURVE")]
impl SimpleEntityHandler for SurfaceCurveHandler {
    /// `(curve_3d_ref, wrapper)` — caller (writer wrapper) already emitted
    /// the underlying 3D curve and hands over the preserved wrapper.
    type WriteInput = (u64, SurfaceCurveWrapper);

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
        (curve_3d_ref, wrapper): (u64, SurfaceCurveWrapper),
    ) -> Result<u64, WriteError> {
        write_surface_or_seam_curve_body(buf, curve_3d_ref, &wrapper)
    }
}
