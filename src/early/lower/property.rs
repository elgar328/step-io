//! Property-domain `lower` fns (attribute leaf batch). See the
//! [module docs](super) for the lowering contract.

use crate::early::model::{
    EarlyDescriptionAttribute, EarlyGeneralProperty, EarlyIdAttribute, EarlyNameAttribute,
};
use crate::ir::error::ConvertError;
use crate::ir::property::{
    DescriptionAttribute, DescriptionAttributeItem, GeneralProperty, IdAttribute, IdAttributeItem,
    NameAttribute, NameAttributeItem, PropertyPool,
};
use crate::reader::ReaderContext;

/// Lower one `GENERAL_PROPERTY`. The legacy read collapsed an empty
/// description to `None` (both `$` and `""`) — the empty-string filter
/// reproduces that on the faithful L1 `Option`.
pub(crate) fn lower_general_property(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyGeneralProperty,
) {
    let gp_id = ctx
        .properties
        .get_or_insert_with(PropertyPool::default)
        .general_properties
        .push(GeneralProperty {
            id: early.id,
            name: early.name,
            description: early.description.filter(|d| !d.is_empty()),
        });
    ctx.id_cache.insert(entity_id, gp_id);
}

/// Lower one `NAME_ATTRIBUTE` (unsupported `named_item` target = warning +
/// drop, legacy leniency; no `id_cache` registration — the arena's only
/// consumer is the writer's emit loop).
pub(crate) fn lower_name_attribute(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyNameAttribute,
) {
    let item_ref = early.named_item;
    let named_item = if let Some(product_id) = ctx.product_of_pdef(item_ref) {
        NameAttributeItem::ProductDefinition(product_id)
    } else if let Some(du_id) = ctx.id_cache.get::<crate::ir::id::DerivedUnitId>(item_ref) {
        NameAttributeItem::DerivedUnit(du_id)
    } else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!("NAME_ATTRIBUTE.named_item #{item_ref} target type unsupported"),
        });
        return;
    };
    let pool = ctx.properties.get_or_insert_with(PropertyPool::default);
    pool.name_attributes.push(NameAttribute {
        attribute_value: early.attribute_value,
        named_item,
    });
}

/// Lower one `DESCRIPTION_ATTRIBUTE` (same leniencies as `NAME_ATTRIBUTE`).
pub(crate) fn lower_description_attribute(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyDescriptionAttribute,
) {
    let item_ref = early.described_item;
    let described_item = if let Some(pao_id) = ctx
        .id_cache
        .get::<crate::ir::PersonAndOrganizationId>(item_ref)
    {
        DescriptionAttributeItem::PersonAndOrganization(pao_id)
    } else if let Some(repr_id) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(item_ref)
    {
        DescriptionAttributeItem::Representation(repr_id)
    } else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "DESCRIPTION_ATTRIBUTE.described_item #{item_ref} target type unsupported"
            ),
        });
        return;
    };
    let pool = ctx.properties.get_or_insert_with(PropertyPool::default);
    pool.description_attributes.push(DescriptionAttribute {
        attribute_value: early.attribute_value,
        described_item,
    });
}

/// Lower one `ID_ATTRIBUTE`. `identified_item` is a SELECT; its
/// `shape_aspect` member covers every `shape_aspect` subtype, unified through
/// `ShapeAspectRef` (same leniencies as `NAME_ATTRIBUTE`).
pub(crate) fn lower_id_attribute(ctx: &mut ReaderContext, entity_id: u64, early: EarlyIdAttribute) {
    let item_ref = early.identified_item;
    let identified_item = if let Some(sa_ref) =
        crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref(
            ctx, item_ref,
        ) {
        IdAttributeItem::ShapeAspect(sa_ref)
    } else if let Some(pd_id) = ctx
        .id_cache
        .get::<crate::ir::id::PropertyDefinitionId>(item_ref)
    {
        IdAttributeItem::PropertyDefinition(pd_id)
    } else if let Some(g_id) = ctx.id_cache.get::<crate::ir::GroupId>(item_ref) {
        IdAttributeItem::Group(g_id)
    } else if let Some(a_id) = ctx.id_cache.get::<crate::ir::AddressId>(item_ref) {
        IdAttributeItem::Address(a_id)
    } else if let Some(ac_id) = ctx
        .id_cache
        .get::<crate::ir::ApplicationContextId>(item_ref)
    {
        IdAttributeItem::ApplicationContext(ac_id)
    } else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!("ID_ATTRIBUTE.identified_item #{item_ref} target type unsupported"),
        });
        return;
    };
    let pool = ctx.properties.get_or_insert_with(PropertyPool::default);
    pool.id_attributes.push(IdAttribute {
        attribute_value: early.attribute_value,
        identified_item,
    });
}
