//! `OPEN_SHELL` handler (2-layer path; shares the shell writer with
//! `CLOSED_SHELL`).

use crate::early::{bind, lower};
use crate::entities::SimpleEntityHandler;
use crate::entities::topology::closed_shell::write_shell_body;
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
        let early = bind::bind_open_shell(entity_id, attrs)?;
        lower::lower_open_shell(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, id: ShellId) -> Result<u64, WriteError> {
        write_shell_body(buf, id)
    }
}
