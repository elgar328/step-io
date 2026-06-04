//! `DOCUMENT_PRODUCT_EQUIVALENCE` handler plm Document
//! linker. `related_product` SELECT collapses to a `ProductId` via the
//! shared `resolve_date_time_item` helper.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_optional_string, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{DocumentProductEquivalence, DocumentProductItem, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::resolve_date_time_item;
use step_io_macros::step_entity;

pub(crate) struct DocumentProductEquivalenceHandler;

#[step_entity(name = "DOCUMENT_PRODUCT_EQUIVALENCE")]
impl SimpleEntityHandler for DocumentProductEquivalenceHandler {
    type WriteInput = DocumentProductEquivalence;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "DOCUMENT_PRODUCT_EQUIVALENCE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_optional_string(attrs, 1, entity_id, "description")?;
        let doc_ref = read_entity_ref(attrs, 2, entity_id, "relating_document")?;
        let product_ref = read_entity_ref(attrs, 3, entity_id, "related_product")?;
        let Some(&relating_document) = ctx.plm_document_id_map.get(&doc_ref) else {
            return Ok(());
        };
        // `related_product` may reference a PRODUCT_DEFINITION_FORMATION
        // directly (common for document-equivalence links). Preserve that so
        // the formation ref round-trips; otherwise collapse to the product.
        let related_product = if let Some(&fid) = ctx.formation_arena_map.get(&product_ref) {
            DocumentProductItem::Formation(fid)
        } else {
            let Some(pid) = resolve_date_time_item(ctx, product_ref) else {
                return Ok(());
            };
            DocumentProductItem::Product(pid)
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool
            .document_product_equivalences
            .push(DocumentProductEquivalence {
                name,
                description,
                relating_document,
                related_product,
            });
        ctx.plm_document_product_equivalence_id_map
            .insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, d: DocumentProductEquivalence) -> Result<u64, WriteError> {
        let doc_step = buf.plm_document_step_ids[d.relating_document.0 as usize];
        let product_step = match d.related_product {
            DocumentProductItem::Product(pid) => buf.product_def_ids[&pid],
            DocumentProductItem::Formation(fid) => {
                // The faithful formation step id, filled by the assembly chain
                // (which runs before the plm cluster). A `0` slot means the
                // formation wasn't emitted — fall back to its product ref.
                let step = buf.product_definition_formation_step_ids[fid.0 as usize];
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
        let desc_attr = match d.description {
            Some(s) => Attribute::String(s),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "DOCUMENT_PRODUCT_EQUIVALENCE",
            vec![
                Attribute::String(d.name),
                desc_attr,
                Attribute::EntityRef(doc_step),
                Attribute::EntityRef(product_step),
            ],
        ))
    }
}
