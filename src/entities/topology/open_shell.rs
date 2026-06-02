//! `OPEN_SHELL` handler — Pass 5-7a.
//!
//! Sister handler of `CLOSED_SHELL`. Both share the read/write body in
//! `closed_shell.rs`; only `is_open` flips and the entity name differs.

use crate::entities::SimpleEntityHandler;
use crate::entities::topology::closed_shell::{read_shell_body, write_shell_body};
use crate::ir::ShellId;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct OpenShellHandler;

#[step_entity(name = "OPEN_SHELL")]
impl SimpleEntityHandler for OpenShellHandler {
    type WriteInput = ShellId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_shell_body(ctx, entity_id, attrs, "OPEN_SHELL", true)
    }

    fn write(buf: &mut WriteBuffer, id: ShellId) -> Result<u64, WriteError> {
        write_shell_body(buf, id)
    }
}
