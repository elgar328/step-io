//! `DOCUMENT_FILE` handler — plm (2-layer path: generated bind/serialize
//! and hand-written lower/lift). Multiple inheritance of `document` and
//! `characterized_object`: the flattened trailing slots surface as
//! `name_2` and `description_2` in L1 and map to the
//! `characterized_object` fields.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::DocumentFile;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DocumentFileHandler;

#[step_entity(name = "DOCUMENT_FILE")]
impl SimpleEntityHandler for DocumentFileHandler {
    type WriteInput = DocumentFile;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_document_file(entity_id, attrs)?;
        lower::lower_document_file(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, d: DocumentFile) -> Result<u64, WriteError> {
        let kind_step = buf.step_id(d.kind);
        let early = lift::lift_document_file(d, kind_step);
        Ok(serialize::serialize_document_file(buf, &early))
    }
}
