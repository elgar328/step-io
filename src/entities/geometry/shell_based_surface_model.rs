//! `SHELL_BASED_SURFACE_MODEL` handler.
//!
//! Reader resolves the boundary list into `ShellId`s via `shell_map` and
//! caches the per-SBSM list in `sbsm_shells_map` so MSSR can flatten one
//! or more SBSMs into a `SurfaceBody`. Writer wraps the IR shell list in
//! the standard SBSM line; shell entities themselves are emitted by
//! `emit_shell` upstream.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::id::ShellId;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ShellBasedSurfaceModelHandler;

#[step_entity(name = "SHELL_BASED_SURFACE_MODEL")]
impl SimpleEntityHandler for ShellBasedSurfaceModelHandler {
    type WriteInput = Vec<ShellId>;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_shell_based_surface_model(entity_id, attrs)?;
        lower::lower_shell_based_surface_model(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, shells: Vec<ShellId>) -> Result<u64, WriteError> {
        let mut shell_refs = Vec::with_capacity(shells.len());
        for s in shells {
            shell_refs.push(buf.emit_shell(s)?);
        }
        let early = lift::lift_shell_based_surface_model(shell_refs);
        Ok(serialize::serialize_shell_based_surface_model(buf, &early))
    }
}
