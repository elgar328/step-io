//! `DOCUMENT_PRODUCT_EQUIVALENCE` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::{DocumentProductEquivalence, DocumentProductItem};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DocumentProductEquivalenceHandler;

#[step_entity(name = "DOCUMENT_PRODUCT_EQUIVALENCE")]
impl SimpleEntityHandler for DocumentProductEquivalenceHandler {
    type WriteInput = DocumentProductEquivalence;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_document_product_equivalence(entity_id, attrs)?;
        lower::lower_document_product_equivalence(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, d: DocumentProductEquivalence) -> Result<u64, WriteError> {
        let doc_step = buf.step_id(d.relating_document);
        let product_step = match d.related_product {
            DocumentProductItem::Product(pid) => buf.product_def_ids[&pid],
            DocumentProductItem::Formation(fid) => {
                // The faithful formation step id, filled by the assembly chain
                // (which runs before the plm cluster). A `0` slot means the
                // formation wasn't emitted — fall back to its product ref.
                let step = buf.step_id(fid);
                if step != 0 {
                    step
                } else {
                    let of_product = buf
                        .model
                        .assembly
                        .as_ref()
                        .map(|a| a.product_definition_formations[fid].data().of_product);
                    of_product
                        .and_then(|pid| buf.product_def_ids.get(&pid).copied())
                        .unwrap_or(0)
                }
            }
        };
        let early =
            lift::lift_document_product_equivalence(d.name, d.description, doc_step, product_step);
        Ok(serialize::serialize_document_product_equivalence(
            buf, &early,
        ))
    }
}
