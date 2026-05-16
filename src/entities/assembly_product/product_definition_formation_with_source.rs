//! `PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE` handler — Pass 6-2.
//!
//! Sister of `product_definition_formation`. Reads through the shared
//! body with `with_source = true` so the loyalty flag on the referenced
//! product is set. Writer emits the same first three attrs plus a
//! hard-coded `.NOT_KNOWN.` source enum (the only value seen in real
//! AP203 fixtures).

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::product_definition_formation::read_product_definition_formation_body;

pub(crate) struct ProductDefinitionFormationWithSourceHandler;

impl SimpleEntityHandler for ProductDefinitionFormationWithSourceHandler {
    const NAME: &'static str = "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6PdefFormation;
    /// PRODUCT entity ref the formation points at.
    type WriteInput = u64;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_product_definition_formation_body(ctx, entity_id, attrs, true)
    }

    fn write(buf: &mut WriteBuffer, prod_entity: u64) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE",
            vec![
                Attribute::String(String::new()),
                Attribute::String(String::new()),
                Attribute::EntityRef(prod_entity),
                Attribute::Enum("NOT_KNOWN".into()),
            ],
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static PDF_WITH_SOURCE_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ProductDefinitionFormationWithSourceHandler::NAME,
    pass_level: ProductDefinitionFormationWithSourceHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ProductDefinitionFormationWithSourceHandler::read,
    },
};
