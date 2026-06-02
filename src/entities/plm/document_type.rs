//! `DOCUMENT_TYPE` handler — Pass 9-17 plm Document leaf.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{DocumentType, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "DOCUMENT_TYPE")?;
        let product_data_type =
            read_string_or_unset(attrs, 0, entity_id, "product_data_type")?.to_owned();
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.document_types.push(DocumentType { product_data_type });
        ctx.plm_document_type_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, t: DocumentType) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "DOCUMENT_TYPE",
            vec![Attribute::String(t.product_data_type)],
        ))
    }
}
