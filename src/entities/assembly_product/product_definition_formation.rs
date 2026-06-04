//! `PRODUCT_DEFINITION_FORMATION` handler.
//!
//! Hosts the shared reader body that the with-source sister
//! (`product_definition_formation_with_source`) imports. Reader maps the
//! formation's `entity_id` to its `PRODUCT` ref via `formation_to_product`;
//! the with-source variant additionally flips the loyalty flag on the
//! referenced `Product`. Writer emits the bare base type (3 attrs).

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::{
    ProductDefinitionFormation, ProductDefinitionFormationData,
    ProductDefinitionFormationWithSpecifiedSource,
};
use crate::ir::attr::{read_entity_ref, read_enum, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

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
    // has 4 (extra `make_or_buy` enum like `.NOT_KNOWN.`).
    if attrs.len() < 3 {
        return Err(ConvertError::AttributeCount {
            entity_id,
            entity_name: "PRODUCT_DEFINITION_FORMATION".into(),
            expected: 3,
            actual: attrs.len(),
        });
    }
    let id = read_string_or_unset(attrs, 0, entity_id, "id")?.to_owned();
    let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
    let product_ref = read_entity_ref(attrs, 2, entity_id, "of_product")?;
    ctx.formation_to_product.insert(entity_id, product_ref);

    // Store the formation in the carrier arena (source of truth for version
    // metadata) and link it onto the product. If the product hasn't been read
    // (malformed input), keep the chain map but skip the arena entry — the
    // writer then synthesises, matching the legacy leniency.
    let Some(&pid) = ctx.product_arena_map.get(&product_ref) else {
        return Ok(());
    };
    let data = ProductDefinitionFormationData {
        id,
        description,
        of_product: pid,
    };
    let formation = if with_source {
        ctx.assembly_products[pid].formation_with_source = true;
        let make_or_buy = read_enum(attrs, 3, entity_id, "make_or_buy")
            .unwrap_or("NOT_KNOWN")
            .to_owned();
        ProductDefinitionFormation::WithSpecifiedSource(
            ProductDefinitionFormationWithSpecifiedSource {
                inherited: data,
                make_or_buy,
            },
        )
    } else {
        ProductDefinitionFormation::Itself(data)
    };
    let fid = ctx.product_definition_formations.push(formation);
    ctx.formation_arena_map.insert(entity_id, fid);
    ctx.assembly_products[pid].formation = Some(fid);
    Ok(())
}

/// Writer input for both formation handlers: the carrier body strings + the
/// emitted PRODUCT step ref. The synthesis (kernel/empty-arena) path passes
/// empty `id`/`description`; the reader-arena path passes the faithful values.
pub(crate) struct ProductDefinitionFormationWriteInput {
    pub(crate) id: String,
    pub(crate) description: String,
    pub(crate) prod_entity: u64,
}

pub(crate) struct ProductDefinitionFormationHandler;

#[step_entity(name = "PRODUCT_DEFINITION_FORMATION")]
impl SimpleEntityHandler for ProductDefinitionFormationHandler {
    type WriteInput = ProductDefinitionFormationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_product_definition_formation_body(ctx, entity_id, attrs, false)
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ProductDefinitionFormationWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PRODUCT_DEFINITION_FORMATION",
            vec![
                Attribute::String(input.id),
                Attribute::String(input.description),
                Attribute::EntityRef(input.prod_entity),
            ],
        ))
    }
}
