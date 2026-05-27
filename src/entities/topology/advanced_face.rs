//! `ADVANCED_FACE` handler — Pass 5-6.
//!
//! Shares its read/write body with `FACE_SURFACE` via the
//! `read_face_body` / `write_face_body` helpers below. The sister
//! handler in `face_surface.rs` imports those helpers and only swaps
//! the `FaceKind` discriminator.

use crate::entities::SimpleEntityHandler;
use crate::ir::FaceId;
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref, read_entity_ref_list, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::topology::{Face, FaceKind};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::{ReaderContext, bool_to_orientation};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::buffer::topology::orientation_bool;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

/// Reader body shared by `ADVANCED_FACE` and `FACE_SURFACE`. The two
/// entities share the same shape; only the `FaceKind` tag differs.
pub(super) fn read_face_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    kind: FaceKind,
) -> Result<(), ConvertError> {
    let step_name = match kind {
        FaceKind::Advanced => "ADVANCED_FACE",
        FaceKind::General => "FACE_SURFACE",
    };
    check_count(attrs, 4, entity_id, step_name)?;
    let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
    let bound_refs = read_entity_ref_list(attrs, 1, entity_id, "bounds")?;
    let surface_ref = read_entity_ref(attrs, 2, entity_id, "face_geometry")?;
    let same_sense = read_bool(attrs, 3, entity_id, "same_sense")?;

    let mut bounds = Vec::with_capacity(bound_refs.len());
    for &r in &bound_refs {
        let wire_id = ctx.resolve_face_bound(entity_id, r, "bounds")?;
        bounds.push(wire_id);
    }

    let surface = ctx.resolve_surface(entity_id, surface_ref, "face_geometry")?;

    let face = Face {
        surface,
        bounds,
        orientation: bool_to_orientation(same_sense),
        kind,
    };
    let id = ctx.topology.faces.push(face);
    ctx.face_map.insert(entity_id, id);
    Ok(())
}

/// Writer body shared by `ADVANCED_FACE` and `FACE_SURFACE`. Looks up
/// the IR `Face` and emits with the entity name selected by the IR's
/// stored `FaceKind` (so callers can dispatch by id alone).
pub(super) fn write_face_body(buf: &mut WriteBuffer, id: FaceId) -> Result<u64, WriteError> {
    if let Some(&n) = buf.face_ids.get(&id) {
        return Ok(n);
    }
    let f: Face = buf
        .model
        .topology
        .faces
        .iter()
        .nth(id.0 as usize)
        .cloned()
        .ok_or_else(|| WriteError::DanglingId {
            detail: format!("FaceId({})", id.0),
        })?;
    let surface = buf.emit_surface(f.surface)?;
    let mut bound_refs = Vec::with_capacity(f.bounds.len());
    for &wid in &f.bounds {
        bound_refs.push(buf.emit_wire(wid)?);
    }
    let name = match f.kind {
        FaceKind::Advanced => "ADVANCED_FACE",
        FaceKind::General => "FACE_SURFACE",
    };
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Simple {
            name: name.into(),
            attrs: vec![
                Attribute::String(String::new()),
                Attribute::List(bound_refs.into_iter().map(Attribute::EntityRef).collect()),
                Attribute::EntityRef(surface),
                orientation_bool(f.orientation),
            ],
        },
    });
    buf.face_ids.insert(id, n);
    Ok(n)
}

pub(crate) struct AdvancedFaceHandler;

#[step_entity(name = "ADVANCED_FACE", pass = Pass5Face)]
impl SimpleEntityHandler for AdvancedFaceHandler {
    type WriteInput = FaceId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_face_body(ctx, entity_id, attrs, FaceKind::Advanced)
    }

    fn write(buf: &mut WriteBuffer, id: FaceId) -> Result<u64, WriteError> {
        write_face_body(buf, id)
    }
}
