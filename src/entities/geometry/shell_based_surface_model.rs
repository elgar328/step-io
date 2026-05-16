//! `SHELL_BASED_SURFACE_MODEL` handler — Pass 6-4.
//!
//! Reader resolves the boundary list into `ShellId`s via `shell_map` and
//! caches the per-SBSM list in `sbsm_shells_map` so MSSR can flatten one
//! or more SBSMs into a `SurfaceBody`. Writer wraps the IR shell list in
//! the standard SBSM line; shell entities themselves are emitted by
//! `emit_shell` upstream.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref_list, read_string};
use crate::ir::error::ConvertError;
use crate::ir::id::ShellId;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct ShellBasedSurfaceModelHandler;

impl SimpleEntityHandler for ShellBasedSurfaceModelHandler {
    const NAME: &'static str = "SHELL_BASED_SURFACE_MODEL";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6Sbsm;
    type WriteInput = Vec<ShellId>;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SHELL_BASED_SURFACE_MODEL")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let shell_refs = read_entity_ref_list(attrs, 1, entity_id, "sbsm_boundary")?;
        let shells: Vec<ShellId> = shell_refs
            .iter()
            .filter_map(|r| ctx.shell_map.get(r).copied())
            .collect();
        ctx.sbsm_shells_map.insert(entity_id, shells);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, shells: Vec<ShellId>) -> Result<u64, WriteError> {
        let mut shell_refs = Vec::with_capacity(shells.len());
        for s in shells {
            shell_refs.push(Attribute::EntityRef(buf.emit_shell(s)?));
        }
        Ok(buf.push_simple(
            "SHELL_BASED_SURFACE_MODEL",
            vec![
                Attribute::String(String::new()),
                Attribute::List(shell_refs),
            ],
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static SBSM_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ShellBasedSurfaceModelHandler::NAME,
    pass_level: ShellBasedSurfaceModelHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ShellBasedSurfaceModelHandler::read,
    },
};
