//! `PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE` handler.
//!
//! Sister of `product_definition_formation` — same shared `lower` with the
//! `make_or_buy` source present, which also flips the loyalty flag on the
//! referenced product. Writer synthesises `.NOT_KNOWN.` (the only value seen
//! in real AP203 fixtures); the reader path round-trips the faithful value.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

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
        // 2-layer path: bind → L1, then the shared formation lower (with the
        // source present → loyalty flag + WithSpecifiedSource variant).
        let early =
            bind::bind_product_definition_formation_with_specified_source(entity_id, attrs)?;
        lower::lower_product_definition_formation(
            ctx,
            entity_id,
            early.id,
            early.description,
            early.of_product,
            Some(early.make_or_buy),
        );
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ProductDefinitionFormationWithSourceWriteInput,
    ) -> Result<u64, WriteError> {
        // 2-layer write path: lift the write input → L1, serialize L1 → text.
        let early = lift::lift_product_definition_formation_with_specified_source(input);
        Ok(serialize::serialize_product_definition_formation_with_specified_source(buf, &early))
    }
}
