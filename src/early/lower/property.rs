//! Property-domain `lower` fns (attribute leaf batch). See the
//! [module docs](super) for the lowering contract.

use crate::early::model::{
    EarlyDescriptionAttribute, EarlyDimensionalCharacteristicRepresentation, EarlyGeneralProperty,
    EarlyGeneralPropertyAssociation, EarlyIdAttribute, EarlyNameAttribute,
    EarlyShapeDefinitionRepresentation,
};
use crate::ir::error::ConvertError;
use crate::ir::property::{
    DerivedDefinitionItem, DescriptionAttribute, DescriptionAttributeItem,
    DimensionalCharacteristicRepresentation, GeneralProperty, GeneralPropertyAssociation,
    IdAttribute, IdAttributeItem, NameAttribute, NameAttributeItem, PropertyPool,
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

/// Lower one `DIMENSIONAL_CHARACTERISTIC_REPRESENTATION` (unresolved
/// dimension / representation = silent drop).
pub(crate) fn lower_dimensional_characteristic_representation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyDimensionalCharacteristicRepresentation,
) {
    let Some(dimension) =
        crate::entities::pmi::resolve_dimensional_characteristic(ctx, early.dimension)
    else {
        return;
    };
    let Some(representation) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(early.representation)
    else {
        return;
    };
    let property = ctx.properties.get_or_insert_with(PropertyPool::default);
    let id = property.dimensional_characteristic_representations.push(
        DimensionalCharacteristicRepresentation {
            dimension,
            representation,
        },
    );
    ctx.id_cache.insert(entity_id, id);
}

/// Lower one `GENERAL_PROPERTY_ASSOCIATION` (unresolved ends warn and drop;
/// the legacy read collapsed an empty description to `None`).
pub(crate) fn lower_general_property_association(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyGeneralPropertyAssociation,
) {
    let base_ref = early.base_definition;
    let Some(base_definition) = ctx
        .id_cache
        .get::<crate::ir::id::GeneralPropertyId>(base_ref)
    else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "GENERAL_PROPERTY_ASSOCIATION.base_definition #{base_ref} is not a GENERAL_PROPERTY"
            ),
        });
        return;
    };
    let derived_ref = early.derived_definition;
    let Some(pd_id) = ctx
        .id_cache
        .get::<crate::ir::id::PropertyDefinitionId>(derived_ref)
    else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "GENERAL_PROPERTY_ASSOCIATION.derived_definition #{derived_ref} did not resolve to a PROPERTY_DEFINITION"
            ),
        });
        return;
    };
    ctx.properties
        .get_or_insert_with(PropertyPool::default)
        .general_property_associations
        .push(GeneralPropertyAssociation {
            name: early.name,
            description: early.description.filter(|d| !d.is_empty()),
            base_definition,
            derived_definition: DerivedDefinitionItem::PropertyDefinition(pd_id),
        });
}

/// Lower one `SHAPE_DEFINITION_REPRESENTATION`. Only SDRs whose PDS resolved
/// to a product (typed one-probe) defer geometry classification to the
/// `resolve_sdr_product_geometry` post-pass; a NAUO-tagged placement PDS
/// appends its SR to the NAUO's placement list; everything else is stashed
/// raw for `resolve_sdr_links` (the PD is read by the property handler,
/// later than this SDR).
pub(crate) fn lower_shape_definition_representation(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: &EarlyShapeDefinitionRepresentation,
) {
    let pdef_shape_ref = early.definition;
    let shape_rep_ref = early.used_representation;
    let Some(pid) = ctx.product_of_pds(pdef_shape_ref) else {
        if let Some(nauo_ref) = ctx.nauo_pds_info.get(&pdef_shape_ref).map(|i| i.nauo) {
            if let Some(sr_id) = ctx
                .id_cache
                .get::<crate::ir::id::RepresentationId>(shape_rep_ref)
            {
                ctx.nauo_placement_sr
                    .entry(nauo_ref)
                    .or_default()
                    .push(sr_id);
            }
            return;
        }
        ctx.sdr_link_refs.push((pdef_shape_ref, shape_rep_ref));
        return;
    };
    // Defer the geometry classification: it follows indirection maps this SDR
    // does not reference, so under topological dispatch they may not be
    // populated yet. `resolve_sdr_product_geometry` runs once every
    // relationship and geometry representation has been read.
    ctx.pending_sdr_geometry
        .push(crate::reader::PendingSdrGeometry {
            pid,
            shape_rep_ref,
            entity_id,
            pdef_shape_ref,
        });
}
