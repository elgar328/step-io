//! `ADVANCED_FACE` handler + the shared face writer (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::FaceId;
use crate::ir::error::ConvertError;
use crate::ir::topology::Face;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// Shared writer for `ADVANCED_FACE` / `FACE_SURFACE`: emit the surface and bound
/// wires (recursion), then route the push through the generated serialize for
/// the variant.
pub(super) fn write_face_body(buf: &mut WriteBuffer, id: FaceId) -> Result<u64, WriteError> {
    let cached = buf.step_id(id);
    if cached != 0 {
        return Ok(cached);
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
    let is_advanced = matches!(f, Face::AdvancedFace(_));
    let d = f.data();
    let surface = buf.emit_surface(d.surface)?;
    let mut bounds = Vec::with_capacity(d.bounds.len());
    for &wid in &d.bounds {
        bounds.push(buf.emit_wire(wid)?);
    }
    let n = if is_advanced {
        serialize::serialize_advanced_face(
            buf,
            &lift::lift_advanced_face(bounds, surface, d.orientation),
        )
    } else {
        serialize::serialize_face_surface(
            buf,
            &lift::lift_face_surface(bounds, surface, d.orientation),
        )
    };
    buf.set_step_id(id, n);
    Ok(n)
}

pub(crate) struct AdvancedFaceHandler;

#[step_entity(name = "ADVANCED_FACE")]
impl SimpleEntityHandler for AdvancedFaceHandler {
    type WriteInput = FaceId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_advanced_face(entity_id, attrs)?;
        lower::lower_advanced_face(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, id: FaceId) -> Result<u64, WriteError> {
        write_face_body(buf, id)
    }
}
