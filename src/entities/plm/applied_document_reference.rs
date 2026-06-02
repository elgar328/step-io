//! `APPLIED_DOCUMENT_REFERENCE` handler plm Document
//! linker. STEP positional shape is `(assigned_document, source, items)`;
//! `source` is inherited from `document_reference` supertype.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{AppliedDocumentReference, DocumentReferenceItem, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::resolve_date_time_item;
use step_io_macros::step_entity;

pub(crate) struct AppliedDocumentReferenceHandler;

#[step_entity(name = "APPLIED_DOCUMENT_REFERENCE")]
impl SimpleEntityHandler for AppliedDocumentReferenceHandler {
    type WriteInput = AppliedDocumentReference;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "APPLIED_DOCUMENT_REFERENCE")?;
        let doc_ref = read_entity_ref(attrs, 0, entity_id, "assigned_document")?;
        let source = read_string_or_unset(attrs, 1, entity_id, "source")?.to_owned();
        let item_refs = read_entity_ref_list(attrs, 2, entity_id, "items")?;
        let Some(&assigned_document) = ctx.plm_document_id_map.get(&doc_ref) else {
            return Ok(());
        };
        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(pid) = resolve_date_time_item(ctx, r) {
                items.push(DocumentReferenceItem::Product(pid));
            }
        }
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.document_references.push(AppliedDocumentReference {
            assigned_document,
            source,
            items,
        });
        ctx.plm_document_reference_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, a: AppliedDocumentReference) -> Result<u64, WriteError> {
        let doc_step = buf.plm_document_step_ids[a.assigned_document.0 as usize];
        let mut item_refs = Vec::with_capacity(a.items.len());
        for item in a.items {
            let step_id = match item {
                DocumentReferenceItem::Product(pid) => buf.product_def_ids[&pid],
            };
            item_refs.push(Attribute::EntityRef(step_id));
        }
        Ok(buf.push_simple(
            "APPLIED_DOCUMENT_REFERENCE",
            vec![
                Attribute::EntityRef(doc_step),
                Attribute::String(a.source),
                Attribute::List(item_refs),
            ],
        ))
    }
}
