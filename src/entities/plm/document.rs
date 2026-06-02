//! `DOCUMENT` handler — Pass 9-18 plm. Variant `Itself` of the
//! `document` arena enum.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{Document, DocumentData, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "DOCUMENT")?;
        let id_field = read_string_or_unset(attrs, 0, entity_id, "id")?.to_owned();
        let name = read_string_or_unset(attrs, 1, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 2, entity_id, "description")?.to_owned();
        let kind_ref = read_entity_ref(attrs, 3, entity_id, "kind")?;
        let Some(&kind) = ctx.plm_document_type_id_map.get(&kind_ref) else {
            return Ok(());
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.documents.push(Document::Itself(DocumentData {
            id: id_field,
            name,
            description,
            kind,
        }));
        ctx.plm_document_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, d: DocumentData) -> Result<u64, WriteError> {
        let kind_step = buf.plm_document_type_step_ids[d.kind.0 as usize];
        Ok(buf.push_simple(
            "DOCUMENT",
            vec![
                Attribute::String(d.id),
                Attribute::String(d.name),
                Attribute::String(d.description),
                Attribute::EntityRef(kind_step),
            ],
        ))
    }
}
