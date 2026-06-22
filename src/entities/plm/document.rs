//! `DOCUMENT` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::DocumentData;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DocumentHandler;

#[step_entity(name = "DOCUMENT")]
impl SimpleEntityHandler for DocumentHandler {
    type WriteInput = DocumentData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_document(entity_id, attrs)?;
        lower::lower_document(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, d: DocumentData) -> Result<u64, WriteError> {
        let kind_step = buf.step_id(d.kind);
        let early = lift::lift_document(d, kind_step);
        Ok(serialize::serialize_document(buf, &early))
    }
}
