//! `PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE` handler.
//!
//! Sister of `product_definition_formation`. Reads through the shared
//! body with `with_source = true` so the loyalty flag on the referenced
//! product is set. Writer emits the same first three attrs plus a
//! hard-coded `.NOT_KNOWN.` source enum (the only value seen in real
//! AP203 fixtures).

use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::product_definition_formation::read_product_definition_formation_body;
use step_io_macros::step_entity;

/// Writer input: the carrier body strings + PRODUCT ref + the `make_or_buy`
/// enum (`"NOT_KNOWN"` synthesised; faithful from the arena on the reader path).
pub(crate) struct ProductDefinitionFormationWithSourceWriteInput {
    pub(crate) id: String,
    pub(crate) description: String,
    pub(crate) prod_entity: u64,
    pub(crate) make_or_buy: String,
}

pub(crate) struct ProductDefinitionFormationWithSourceHandler;

#[step_entity(name = "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE")]
impl SimpleEntityHandler for ProductDefinitionFormationWithSourceHandler {
    type WriteInput = ProductDefinitionFormationWithSourceWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_product_definition_formation_body(ctx, entity_id, attrs, true)
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ProductDefinitionFormationWithSourceWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE",
            vec![
                Attribute::String(input.id),
                Attribute::String(input.description),
                Attribute::EntityRef(input.prod_entity),
                Attribute::Enum(input.make_or_buy),
            ],
        ))
    }
}
