//! `CLOSED_SHELL` handler + the shared shell writer (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::ShellId;
use crate::ir::error::ConvertError;
use crate::ir::topology::Shell;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// Shared writer for `OPEN_SHELL` / `CLOSED_SHELL`: emit each face (recursion),
/// then route the push through the generated serialize for the variant the
/// shell's `is_open` flag selects.
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
    let mut faces = Vec::with_capacity(s.faces.len());
    for &fid in &s.faces {
        faces.push(buf.emit_face(fid)?);
    }
    let n = if s.is_open {
        serialize::serialize_open_shell(buf, &lift::lift_open_shell(faces))
    } else {
        serialize::serialize_closed_shell(buf, &lift::lift_closed_shell(faces))
    };
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
        let early = bind::bind_closed_shell(entity_id, attrs)?;
        lower::lower_closed_shell(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, id: ShellId) -> Result<u64, WriteError> {
        write_shell_body(buf, id)
    }
}
