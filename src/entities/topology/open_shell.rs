//! `OPEN_SHELL` handler — Pass 5-7a.
//!
//! Sister handler of `CLOSED_SHELL`. Both share the read/write body in
//! `closed_shell.rs`; only `is_open` flips and the entity name differs.

use crate::entities::topology::closed_shell::{read_shell_body, write_shell_body};
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::ShellId;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct OpenShellHandler;

impl SimpleEntityHandler for OpenShellHandler {
    const NAME: &'static str = "OPEN_SHELL";
    const PASS_LEVEL: PassLevel = PassLevel::Pass5Shell;
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static OPEN_SHELL_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: OpenShellHandler::NAME,
    pass_level: OpenShellHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: OpenShellHandler::read,
    },
};
