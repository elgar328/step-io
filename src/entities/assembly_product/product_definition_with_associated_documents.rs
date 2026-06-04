//! `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS` handler.
//!
//! Standard STEP subtype of `PRODUCT_DEFINITION` (AP203 / AP242) that
//! tags the definition with a list of documentation refs. The reader
//! reuses [`super::product_definition::read_product_definition_body`] for
//! the base attrs and additionally resolves `documentation_ids` onto the
//! product's `associated_documents` loyalty field, so the writer re-emits
//! this subtype (via `emit_pdef`) instead of downgrading to plain
//! `PRODUCT_DEFINITION`.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{read_entity_ref, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::product_definition::read_product_definition_body;
use step_io_macros::step_entity;

/// Writer input for the subtype: the same formation / context refs as a plain
/// `PRODUCT_DEFINITION` plus the resolved `DOCUMENT` step ids for
/// `documentation_ids`.
pub(crate) struct ProductDefinitionWithAssociatedDocumentsWriteInput {
    pub(crate) formation: u64,
    pub(crate) pdef_ctx: u64,
    pub(crate) documentation: Vec<u64>,
}

pub(crate) struct ProductDefinitionWithAssociatedDocumentsHandler;

#[step_entity(name = "PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS")]
impl SimpleEntityHandler for ProductDefinitionWithAssociatedDocumentsHandler {
    type WriteInput = ProductDefinitionWithAssociatedDocumentsWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // Shared base read also records pdef -> product. On success the
        // formation ref is known to resolve to a product.
        read_product_definition_body(ctx, entity_id, attrs)?;

        // Capture documentation_ids (attr[4], a SET OF document) onto the
        // product so the writer re-emits the WITH_ASSOCIATED_DOCUMENTS subtype
        // rather than downgrading to a plain PRODUCT_DEFINITION.
        if attrs.len() < 5 {
            return Ok(()); // defensive: malformed subtype without the extra attr
        }
        let doc_refs = read_entity_ref_list(attrs, 4, entity_id, "documentation_ids")?;
        let mut docs = Vec::with_capacity(doc_refs.len());
        for r in doc_refs {
            if let Some(&id) = ctx.plm_document_id_map.get(&r) {
                docs.push(id);
            } else {
                // Unsupported document subtype — surface and skip that ref;
                // the remaining resolved docs still ride the subtype.
                ctx.warnings.push(ConvertError::MissingReference {
                    from: entity_id,
                    to: r,
                    field_name: "documentation_ids",
                });
            }
        }
        if docs.is_empty() {
            return Ok(()); // nothing resolved -> writer keeps plain PD
        }
        let formation_ref = read_entity_ref(attrs, 2, entity_id, "formation")?;
        if let Some(&product_ref) = ctx.formation_to_product.get(&formation_ref)
            && let Some(&pid) = ctx.product_arena_map.get(&product_ref)
        {
            ctx.assembly_products[pid].associated_documents = docs;
        }
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ProductDefinitionWithAssociatedDocumentsWriteInput,
    ) -> Result<u64, WriteError> {
        // Mirror the plain PRODUCT_DEFINITION attrs (id / description are
        // synthesised the same way) plus the documentation_ids SET.
        let docs = Attribute::List(
            input
                .documentation
                .into_iter()
                .map(Attribute::EntityRef)
                .collect(),
        );
        Ok(buf.push_simple(
            "PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS",
            vec![
                Attribute::String("design".into()),
                Attribute::String(String::new()),
                Attribute::EntityRef(input.formation),
                Attribute::EntityRef(input.pdef_ctx),
                docs,
            ],
        ))
    }
}
