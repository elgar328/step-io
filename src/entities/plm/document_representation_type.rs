//! `DOCUMENT_REPRESENTATION_TYPE` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::DocumentRepresentationType;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DocumentRepresentationTypeHandler;

#[step_entity(name = "DOCUMENT_REPRESENTATION_TYPE")]
impl SimpleEntityHandler for DocumentRepresentationTypeHandler {
    type WriteInput = DocumentRepresentationType;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_document_representation_type(entity_id, attrs)?;
        lower::lower_document_representation_type(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, d: DocumentRepresentationType) -> Result<u64, WriteError> {
        let doc_step = buf.step_id(d.represented_document);
        let early = lift::lift_document_representation_type(d.name, doc_step);
        Ok(serialize::serialize_document_representation_type(
            buf, &early,
        ))
    }
}
