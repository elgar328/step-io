//! `APPLIED_DOCUMENT_REFERENCE` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::{AppliedDocumentReference, DocumentReferenceItem};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
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
        let early = bind::bind_applied_document_reference(entity_id, attrs)?;
        lower::lower_applied_document_reference(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, a: AppliedDocumentReference) -> Result<u64, WriteError> {
        let assigned_document_step = buf.step_id(a.assigned_document);
        let items: Vec<u64> = a
            .items
            .into_iter()
            .map(|item| match item {
                DocumentReferenceItem::Product(pid) => buf.product_def_ids[&pid],
            })
            .collect();
        let early = lift::lift_applied_document_reference(assigned_document_step, a.source, items);
        Ok(serialize::serialize_applied_document_reference(buf, &early))
    }
}
