//! `DOCUMENT_REPRESENTATION_TYPE` handler plm Document
//! linker.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{DocumentRepresentationType, PlmPool};
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
        check_count(attrs, 2, entity_id, "DOCUMENT_REPRESENTATION_TYPE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let doc_ref = read_entity_ref(attrs, 1, entity_id, "represented_document")?;
        let Some(represented_document) = ctx.id_cache.get::<crate::ir::DocumentId>(doc_ref) else {
            return Ok(());
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool
            .document_representation_types
            .push(DocumentRepresentationType {
                name,
                represented_document,
            });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, d: DocumentRepresentationType) -> Result<u64, WriteError> {
        let doc_step = buf.step_id(d.represented_document);
        Ok(buf.push_simple(
            "DOCUMENT_REPRESENTATION_TYPE",
            vec![Attribute::String(d.name), Attribute::EntityRef(doc_step)],
        ))
    }
}
