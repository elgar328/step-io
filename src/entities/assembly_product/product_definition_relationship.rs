//! `PRODUCT_DEFINITION_RELATIONSHIP` handler — Pass 6-3b.
//!
//! Reader resolves both `relating` / `related` PDEFs to [`ProductId`] via
//! [`ReaderContext::resolve_product_by_pdef`] and pushes a
//! `ProductDefinitionRelationship::Plain` entry into the assembly arena.
//! `MAKE_FROM_USAGE_OPTION` is the in-enum subtype, handled by its own
//! sibling handler with the same pass.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::{PlainProductDefinitionRelationship, ProductDefinitionRelationship};
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ProductDefinitionRelationshipWriteInput {
    pub(crate) plain: PlainProductDefinitionRelationship,
    pub(crate) relating_pdef_step: u64,
    pub(crate) related_pdef_step: u64,
}

pub(crate) struct ProductDefinitionRelationshipHandler;

#[step_entity(name = "PRODUCT_DEFINITION_RELATIONSHIP", pass = Pass6Pdr)]
impl SimpleEntityHandler for ProductDefinitionRelationshipHandler {
    type WriteInput = ProductDefinitionRelationshipWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 5, entity_id, "PRODUCT_DEFINITION_RELATIONSHIP")?;
        let id = read_string_or_unset(attrs, 0, entity_id, "id")?.to_owned();
        let name = read_string_or_unset(attrs, 1, entity_id, "name")?.to_owned();
        let description = read_optional_description(attrs, 2, entity_id)?;
        let relating_pdef = read_entity_ref(attrs, 3, entity_id, "relating_product_definition")?;
        let related_pdef = read_entity_ref(attrs, 4, entity_id, "related_product_definition")?;
        let relating =
            ctx.resolve_product_by_pdef(entity_id, relating_pdef, "relating_product_definition")?;
        let related =
            ctx.resolve_product_by_pdef(entity_id, related_pdef, "related_product_definition")?;
        let arena_id =
            ctx.product_definition_relationships
                .push(ProductDefinitionRelationship::Plain(
                    PlainProductDefinitionRelationship {
                        id,
                        name,
                        description,
                        relating,
                        related,
                    },
                ));
        ctx.pdr_id_map.insert(entity_id, arena_id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ProductDefinitionRelationshipWriteInput {
            plain,
            relating_pdef_step,
            related_pdef_step,
        }: ProductDefinitionRelationshipWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PRODUCT_DEFINITION_RELATIONSHIP",
            vec![
                Attribute::String(plain.id),
                Attribute::String(plain.name),
                description_attr(plain.description),
                Attribute::EntityRef(relating_pdef_step),
                Attribute::EntityRef(related_pdef_step),
            ],
        ))
    }
}

pub(crate) fn description_attr(desc: Option<String>) -> Attribute {
    match desc {
        Some(s) => Attribute::String(s),
        None => Attribute::Unset,
    }
}

/// Read an `OPTIONAL text` slot — `$` → `None`, string (even empty) → `Some`.
/// Preserves the unset-vs-empty distinction that `read_string_or_unset`
/// flattens.
pub(crate) fn read_optional_description(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
) -> Result<Option<String>, ConvertError> {
    match attrs.get(index) {
        Some(Attribute::Unset) => Ok(None),
        Some(Attribute::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(ConvertError::AttributeType {
            entity_id,
            field_name: "description",
            expected: "String",
            actual: crate::ir::AttributeKindTag::from_attribute(other),
        }),
        None => Err(ConvertError::AttributeIndex {
            entity_id,
            field_name: "description",
            index,
            len: attrs.len(),
        }),
    }
}
