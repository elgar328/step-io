//! `COMPOSITE_TEXT` handler (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::CompositeText;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CompositeTextHandler;

#[step_entity(name = "COMPOSITE_TEXT")]
impl SimpleEntityHandler for CompositeTextHandler {
    type WriteInput = CompositeText;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_composite_text(entity_id, attrs)?;
        lower::lower_composite_text(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, t: CompositeText) -> Result<u64, WriteError> {
        let collected_text: Vec<u64> = t
            .collected_text
            .into_iter()
            .map(|tc| tc.emit_select(buf))
            .collect();
        let early = lift::lift_composite_text(t.name, collected_text);
        Ok(serialize::serialize_composite_text(buf, &early))
    }
}
