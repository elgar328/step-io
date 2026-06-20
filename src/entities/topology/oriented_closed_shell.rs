//! `ORIENTED_CLOSED_SHELL` handler (intermediate map).
//!
//! Records the wrapper in `oriented_closed_shell_map`; the actual
//! `CLOSED_SHELL` arena entry is reused (orientation is later applied
//! in place by `BREP_WITH_VOIDS`). Mirrors the legacy
//! `convert_oriented_closed_shell` and `emit_oriented_closed_shell`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::Orientation;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct OrientedClosedShellHandler;

#[step_entity(name = "ORIENTED_CLOSED_SHELL")]
impl SimpleEntityHandler for OrientedClosedShellHandler {
    /// `(closed_shell_ref, orientation)` — caller already emitted the
    /// underlying `CLOSED_SHELL` and supplies the orientation extracted
    /// from the parent solid's void list.
    type WriteInput = (u64, Orientation);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_oriented_closed_shell(entity_id, attrs)?;
        lower::lower_oriented_closed_shell(ctx, entity_id, &early)
    }

    fn write(
        buf: &mut WriteBuffer,
        (closed_shell_ref, orientation): (u64, Orientation),
    ) -> Result<u64, WriteError> {
        let early = lift::lift_oriented_closed_shell(closed_shell_ref, orientation);
        Ok(serialize::serialize_oriented_closed_shell(buf, &early))
    }
}
