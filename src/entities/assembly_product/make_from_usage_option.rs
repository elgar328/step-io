//! `MAKE_FROM_USAGE_OPTION` handler.
//!
//! `SUBTYPE OF PRODUCT_DEFINITION_RELATIONSHIP` — inherits the 5 base
//! attributes and adds `(ranking INTEGER, ranking_rationale STRING,
//! quantity REF→MEASURE_WITH_UNIT)`. Shares the
//! `product_definition_relationships` arena with the plain supertype.
//! `quantity` resolves through `mwu_id_map`; an
//! unmapped quantity (e.g. an unsupported MWU subtype) silently drops the
//! entity, matching the policy of other MWU consumers.

use crate::entities::SimpleEntityHandler;
use crate::entities::assembly_product::product_definition_relationship::{
    description_attr, read_optional_description,
};
use crate::ir::assembly::{MakeFromUsageOption, ProductDefinitionRelationship};
use crate::ir::attr::{check_count, read_entity_ref, read_integer, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct MakeFromUsageOptionWriteInput {
    pub(crate) mfu: MakeFromUsageOption,
    pub(crate) relating_pdef_step: u64,
    pub(crate) related_pdef_step: u64,
    pub(crate) quantity_step: u64,
}

pub(crate) struct MakeFromUsageOptionHandler;

#[step_entity(name = "MAKE_FROM_USAGE_OPTION")]
impl SimpleEntityHandler for MakeFromUsageOptionHandler {
    type WriteInput = MakeFromUsageOptionWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 8, entity_id, "MAKE_FROM_USAGE_OPTION")?;
        let id = read_string_or_unset(attrs, 0, entity_id, "id")?.to_owned();
        let name = read_string_or_unset(attrs, 1, entity_id, "name")?.to_owned();
        let description = read_optional_description(attrs, 2, entity_id)?;
        let relating_pdef = read_entity_ref(attrs, 3, entity_id, "relating_product_definition")?;
        let related_pdef = read_entity_ref(attrs, 4, entity_id, "related_product_definition")?;
        let ranking = read_integer(attrs, 5, entity_id, "ranking")?;
        let ranking_rationale =
            read_string_or_unset(attrs, 6, entity_id, "ranking_rationale")?.to_owned();
        let quantity_ref = read_entity_ref(attrs, 7, entity_id, "quantity")?;

        let relating =
            ctx.resolve_product_by_pdef(entity_id, relating_pdef, "relating_product_definition")?;
        let related =
            ctx.resolve_product_by_pdef(entity_id, related_pdef, "related_product_definition")?;
        let Some(quantity) = ctx
            .id_cache
            .get::<crate::ir::id::MeasureWithUnitId>(quantity_ref)
        else {
            // Unsupported MWU subtype — silently drop, matching other MWU consumers.
            return Ok(());
        };

        let arena_id =
            ctx.product_definition_relationships
                .push(ProductDefinitionRelationship::MakeFrom(
                    MakeFromUsageOption {
                        id,
                        name,
                        description,
                        relating,
                        related,
                        ranking,
                        ranking_rationale,
                        quantity,
                    },
                ));
        ctx.id_cache.insert(entity_id, arena_id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        MakeFromUsageOptionWriteInput {
            mfu,
            relating_pdef_step,
            related_pdef_step,
            quantity_step,
        }: MakeFromUsageOptionWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "MAKE_FROM_USAGE_OPTION",
            vec![
                Attribute::String(mfu.id),
                Attribute::String(mfu.name),
                description_attr(mfu.description),
                Attribute::EntityRef(relating_pdef_step),
                Attribute::EntityRef(related_pdef_step),
                Attribute::Integer(mfu.ranking),
                Attribute::String(mfu.ranking_rationale),
                Attribute::EntityRef(quantity_step),
            ],
        ))
    }
}
