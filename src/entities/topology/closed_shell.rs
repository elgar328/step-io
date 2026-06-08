//! `CLOSED_SHELL` handler.
//!
//! Shares its read body with `OPEN_SHELL` via the `read_shell_body`
//! helper. The writer keeps `emit_shell` as the dispatcher (keys off
//! `Shell::is_open`), and the handler's `write` simply delegates there.

use crate::entities::SimpleEntityHandler;
use crate::ir::ShellId;
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::topology::{Orientation, Shell};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

/// Reader body shared by `CLOSED_SHELL` and `OPEN_SHELL`. The only
/// difference is the `is_open` flag stored on the IR `Shell`.
pub(super) fn read_shell_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    entity_name: &'static str,
    is_open: bool,
) -> Result<(), ConvertError> {
    check_count(attrs, 2, entity_id, entity_name)?;
    let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
    let face_refs = read_entity_ref_list(attrs, 1, entity_id, "cfs_faces")?;

    let mut faces = Vec::with_capacity(face_refs.len());
    for &r in &face_refs {
        let face_id = ctx.resolve_face(entity_id, r, "cfs_faces")?;
        faces.push(face_id);
    }

    let shell = Shell {
        faces,
        orientation: Orientation::Forward,
        is_open,
    };
    let id = ctx.topology.shells.push(shell);
    ctx.id_cache.insert(entity_id, id);
    Ok(())
}

/// Writer body shared by `CLOSED_SHELL` and `OPEN_SHELL`. Looks up the
/// IR `Shell`, emits the matching entity name from `Shell::is_open`,
/// and caches the freshly emitted id in `shell_ids`.
pub(super) fn write_shell_body(buf: &mut WriteBuffer, id: ShellId) -> Result<u64, WriteError> {
    let cached = buf.step_id(id);
    if cached != 0 {
        return Ok(cached);
    }
    let s: Shell = buf
        .model
        .topology
        .shells
        .iter()
        .nth(id.0 as usize)
        .cloned()
        .ok_or_else(|| WriteError::DanglingId {
            detail: format!("ShellId({})", id.0),
        })?;
    // `Shell::orientation` is intentionally ignored — STEP's CLOSED_SHELL /
    // OPEN_SHELL carry no orientation attribute. Inner-void orientation is
    // attached via ORIENTED_CLOSED_SHELL inside emit_solid.
    let mut face_refs = Vec::with_capacity(s.faces.len());
    for &fid in &s.faces {
        face_refs.push(buf.emit_face(fid)?);
    }
    let name = if s.is_open {
        "OPEN_SHELL"
    } else {
        "CLOSED_SHELL"
    };
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Simple {
            name: name.into(),
            attrs: vec![
                Attribute::String(String::new()),
                Attribute::List(face_refs.into_iter().map(Attribute::EntityRef).collect()),
            ],
        },
    });
    buf.set_step_id(id, n);
    Ok(n)
}

pub(crate) struct ClosedShellHandler;

#[step_entity(name = "CLOSED_SHELL")]
impl SimpleEntityHandler for ClosedShellHandler {
    type WriteInput = ShellId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_shell_body(ctx, entity_id, attrs, "CLOSED_SHELL", false)
    }

    fn write(buf: &mut WriteBuffer, id: ShellId) -> Result<u64, WriteError> {
        write_shell_body(buf, id)
    }
}
