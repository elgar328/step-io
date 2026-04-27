//! `PRODUCT_DEFINITION_FORMATION` handler — Pass 6-2.
//!
//! Hosts the shared reader body that the with-source sister
//! (`product_definition_formation_with_source`) imports. Reader maps the
//! formation's `entity_id` to its `PRODUCT` ref via `formation_to_product`;
//! the with-source variant additionally flips the loyalty flag on the
//! referenced `Product`. Writer emits the bare base type (3 attrs).

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

/// Shared reader body for `PRODUCT_DEFINITION_FORMATION` and its
/// `_WITH_SPECIFIED_SOURCE` subtype. Both share the first three attrs;
/// `with_source = true` toggles the loyalty flag on the referenced
/// `Product` so the writer round-trips the same subtype.
pub(crate) fn read_product_definition_formation_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    with_source: bool,
) -> Result<(), ConvertError> {
    // At least 3 attrs for base type; the `_WITH_SPECIFIED_SOURCE` subtype
    // has 4 (extra `source` enum like `.NOT_KNOWN.`). The source enum is
    // informational only — the loyalty flag on `Product` captures the
    // choice between subtypes.
    if attrs.len() < 3 {
        return Err(ConvertError::AttributeCount {
            entity_id,
            entity_name: "PRODUCT_DEFINITION_FORMATION".into(),
            expected: 3,
            actual: attrs.len(),
        });
    }
    let _id = read_string_or_unset(attrs, 0, entity_id, "id")?;
    let _description = read_string_or_unset(attrs, 1, entity_id, "description")?;
    let product_ref = read_entity_ref(attrs, 2, entity_id, "of_product")?;
    ctx.formation_to_product.insert(entity_id, product_ref);
    if with_source && let Some(&pid) = ctx.product_arena_map.get(&product_ref) {
        ctx.assembly_products[pid].formation_with_source = true;
    }
    Ok(())
}

pub(crate) struct ProductDefinitionFormationHandler;

impl SimpleEntityHandler for ProductDefinitionFormationHandler {
    const NAME: &'static str = "PRODUCT_DEFINITION_FORMATION";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6PdefFormation;
    /// PRODUCT entity ref the formation points at.
    type WriteInput = u64;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        read_product_definition_formation_body(ctx, entity_id, attrs, false)
    }

    fn write(buf: &mut WriteBuffer, prod_entity: u64) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PRODUCT_DEFINITION_FORMATION",
            vec![
                Attribute::String(String::new()),
                Attribute::String(String::new()),
                Attribute::EntityRef(prod_entity),
            ],
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static PDF_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ProductDefinitionFormationHandler::NAME,
    pass_level: ProductDefinitionFormationHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ProductDefinitionFormationHandler::read,
    },
};
