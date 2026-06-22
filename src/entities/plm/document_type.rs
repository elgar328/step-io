//! `DOCUMENT_TYPE` handler — plm metadata leaf (2-layer path: generated
//! bind/serialize + pass-through lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::DocumentType;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DocumentTypeHandler;

#[step_entity(name = "DOCUMENT_TYPE")]
impl SimpleEntityHandler for DocumentTypeHandler {
    type WriteInput = DocumentType;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_document_type(entity_id, attrs)?;
        lower::lower_document_type(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, v: DocumentType) -> Result<u64, WriteError> {
        let early = lift::lift_document_type(v);
        Ok(serialize::serialize_document_type(buf, &early))
    }
}
