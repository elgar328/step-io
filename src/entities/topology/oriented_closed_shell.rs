//! `ORIENTED_CLOSED_SHELL` handler — Pass 5-7b (intermediate map).
//!
//! Records the wrapper in `oriented_closed_shell_map`; the actual
//! `CLOSED_SHELL` arena entry is reused (orientation is later applied
//! in place by `BREP_WITH_VOIDS`). Mirrors the legacy
//! `convert_oriented_closed_shell` and `emit_oriented_closed_shell`.

use crate::entities::SimpleEntityHandler;
use crate::ir::Orientation;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::{ReaderContext, bool_to_orientation};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::buffer::topology::orientation_bool;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct OrientedClosedShellHandler;

#[step_entity(name = "ORIENTED_CLOSED_SHELL", pass = Pass5OrientedShell)]
impl SimpleEntityHandler for OrientedClosedShellHandler {
    /// `(closed_shell_ref, orientation)` — caller already emitted the
    /// underlying `CLOSED_SHELL` and supplies the orientation extracted
    /// from the parent solid's void list.
    type WriteInput = (u64, Orientation);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "ORIENTED_CLOSED_SHELL")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        // attrs[1] is the derived `*` field — skip.
        let shell_ref = read_entity_ref(attrs, 2, entity_id, "closed_shell_element")?;
        let orientation = read_bool(attrs, 3, entity_id, "orientation")?;

        let shell_id = ctx.resolve_shell(entity_id, shell_ref, "closed_shell_element")?;
        ctx.oriented_closed_shell_map
            .insert(entity_id, (shell_id, bool_to_orientation(orientation)));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (closed_shell_ref, orientation): (u64, Orientation),
    ) -> Result<u64, WriteError> {
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "ORIENTED_CLOSED_SHELL".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::Derived,
                    Attribute::EntityRef(closed_shell_ref),
                    orientation_bool(orientation),
                ],
            },
        });
        Ok(n)
    }
}
