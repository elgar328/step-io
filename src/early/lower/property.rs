//! Property-domain `lower` fns (attribute leaf batch). See the
//! [module docs](super) for the lowering contract.

use crate::early::model::{
    EarlyDescriptionAttribute, EarlyDimensionalCharacteristicRepresentation, EarlyGeneralProperty,
    EarlyGeneralPropertyAssociation, EarlyIdAttribute, EarlyNameAttribute, EarlyPropertyDefinition,
    EarlyShapeDefinitionRepresentation,
};
use crate::entities::pmi::resolve_geometric_tolerance_ref;
use crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref;
use crate::ir::error::ConvertError;
use crate::ir::id::DimensionalLocationId;
use crate::ir::plm::Document;
use crate::ir::pmi::{DimensionalLocation, GeneralDatumReference};
use crate::ir::property::{
    CharacterizedDefinition, DerivedDefinitionItem, DescriptionAttribute, DescriptionAttributeItem,
    DimensionalCharacteristicRepresentation, GeneralProperty, GeneralPropertyAssociation,
    IdAttribute, IdAttributeItem, NameAttribute, NameAttributeItem, PropertyDefinition,
    PropertyDefinitionData, PropertyPool,
};
use crate::ir::shape_rep::CharacterizedObject;
use crate::ir::{ProductId, ShapeAspectRef};
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

/// Resolve the owning product for a `dimensional_location` arena entry — a
/// `shape_aspect_relationship` subtype, so the product is reached via its
/// `relating_shape_aspect` endpoint. (Moved from the `PROPERTY_DEFINITION`
/// handler when its `characterized_definition` dispatch relocated to `lower`.)
fn dimensional_location_target(
    ctx: &ReaderContext,
    id: DimensionalLocationId,
) -> Option<ProductId> {
    let pmi = ctx.pmi.as_ref()?;
    let sa_ref = match &pmi.dimensional_locations[id] {
        DimensionalLocation::Plain(d) | DimensionalLocation::Directed(d) => d.relating_shape_aspect,
        DimensionalLocation::Angular(a) => a.relating_shape_aspect,
    };
    shape_aspect_ref_target(ctx, sa_ref)
}

fn shape_aspect_ref_target(ctx: &ReaderContext, sa_ref: ShapeAspectRef) -> Option<ProductId> {
    match sa_ref {
        ShapeAspectRef::ShapeAspect(id) => Some(ctx.shape_aspects[id].target),
        ShapeAspectRef::CompositeGroupShapeAspect(id) => {
            Some(ctx.composite_group_shape_aspects[id].target)
        }
        ShapeAspectRef::CentreOfSymmetry(id) => Some(ctx.centre_of_symmetries[id].target),
        ShapeAspectRef::AllAroundShapeAspect(id) => Some(ctx.all_around_shape_aspects[id].target),
        ShapeAspectRef::Datum(id) => ctx.pmi.as_ref().map(|p| p.datums[id].target),
        ShapeAspectRef::DatumFeature(id) => {
            ctx.pmi.as_ref().map(|p| p.datum_features[id].data().target)
        }
        ShapeAspectRef::DatumSystem(id) => Some(ctx.datum_systems[id].target),
        ShapeAspectRef::DatumTarget(id) => Some(ctx.datum_targets[id].target),
        ShapeAspectRef::PlacedDatumTargetFeature(id) => {
            Some(ctx.placed_datum_target_features[id].target)
        }
        ShapeAspectRef::ToleranceZone(id) => Some(ctx.tolerance_zones[id].target),
        ShapeAspectRef::GeneralDatumReference(id) => ctx.pmi.as_ref().map(|p| {
            let (GeneralDatumReference::Compartment(d) | GeneralDatumReference::Element(d)) =
                &p.general_datum_references[id];
            d.target
        }),
    }
}

/// Lower one `PROPERTY_DEFINITION`: resolve the `characterized_definition`
/// SELECT (9-arm dispatch on the raw `definition` ref) into a typed
/// `CharacterizedDefinition`, then push the carrier arena entry. Product-bound
/// targets that don't resolve to a product are dropped (legacy leniency); a
/// NAUO-owned-PDS target is deferred. (Relocated verbatim from the handler
/// `read`; `bind` now does the mechanical 3-attr extraction.)
#[allow(clippy::too_many_lines)]
pub(crate) fn lower_property_definition(
    ctx: &mut ReaderContext,
    entity_id: u64,
    early: EarlyPropertyDefinition,
) {
    let name = early.name;
    // Legacy read_string_or_unset + empty-check collapsed `$` and `""` to None.
    let description = early.description.filter(|d| !d.is_empty());
    let target_ref = early.definition;
    let definition = if ctx
        .id_cache
        .get::<crate::ir::ProductDefinitionId>(target_ref)
        .is_some()
    {
        let Some(pid) = ctx.product_of_pdef(target_ref) else {
            return;
        };
        CharacterizedDefinition::ProductDefinition(pid)
    } else if let Some(sa_ref) = resolve_shape_aspect_ref(ctx, target_ref) {
        if shape_aspect_ref_target(ctx, sa_ref).is_none() {
            return;
        }
        CharacterizedDefinition::ShapeAspect(sa_ref)
    } else if let Some(dl_id) = ctx
        .id_cache
        .get::<crate::ir::DimensionalLocationId>(target_ref)
    {
        if dimensional_location_target(ctx, dl_id).is_none() {
            return;
        }
        CharacterizedDefinition::DimensionalLocation(dl_id)
    } else if let Some(pds_pd_id) = ctx
        .id_cache
        .get::<crate::ir::id::PropertyDefinitionId>(target_ref)
    {
        let Some(pool) = ctx.properties.as_ref() else {
            return;
        };
        if !matches!(
            pool.property_definitions[pds_pd_id],
            PropertyDefinition::ProductDefinitionShape(_)
        ) {
            eprintln!(
                "warning: PROPERTY_DEFINITION #{entity_id} target #{target_ref} \
                 resolves to another PROPERTY_DEFINITION (Itself), which is \
                 schema-illegal — skipping"
            );
            return;
        }
        if ctx.product_of_pds(target_ref).is_none() {
            return;
        }
        CharacterizedDefinition::ProductDefinitionShape(pds_pd_id)
    } else if let Some(gp_id) = ctx
        .id_cache
        .get::<crate::ir::id::GeneralPropertyId>(target_ref)
    {
        CharacterizedDefinition::GeneralProperty(gp_id)
    } else if let Some(doc_id) = ctx.id_cache.get::<crate::ir::DocumentId>(target_ref) {
        let is_file = ctx
            .plm
            .as_ref()
            .is_some_and(|p| matches!(p.documents[doc_id], Document::DocumentFile(_)));
        if !is_file {
            eprintln!(
                "warning: PROPERTY_DEFINITION #{entity_id} target #{target_ref} \
                 resolves to a plain DOCUMENT (not a characterized_object) — skipping"
            );
            return;
        }
        CharacterizedDefinition::Document(doc_id)
    } else if let Some(co_id) = ctx
        .id_cache
        .get::<crate::ir::id::CharacterizedObjectId>(target_ref)
    {
        // CIWR (geometric-validation shapes) and MODEL_GEOMETRIC_VIEW (a CIWR
        // subtype in the MBD draughting-model complex) both fall under the
        // `characterized_item` member → shared CIWR arm.
        match ctx.characterized_objects[co_id] {
            CharacterizedObject::CharacterizedItemWithinRepresentation(_)
            | CharacterizedObject::ModelGeometricView(_) => {
                CharacterizedDefinition::CharacterizedItemWithinRepresentation(co_id)
            }
            CharacterizedObject::Itself(_) => CharacterizedDefinition::CharacterizedObject(co_id),
        }
    } else if let Some(gt_ref) = resolve_geometric_tolerance_ref(ctx, target_ref) {
        CharacterizedDefinition::GeometricTolerance(gt_ref)
    } else if let Some(ds_id) = ctx.id_cache.get::<crate::ir::DimensionalSizeId>(target_ref) {
        CharacterizedDefinition::DimensionalSize(ds_id)
    } else if ctx.nauo_pds_info.contains_key(&target_ref) {
        // NAUO-owned PRODUCT_DEFINITION_SHAPE — its ACU id only exists after
        // `resolve_nauo_instances`. Defer; `materialize_nauo_owned_pds` replays.
        ctx.deferred_nauo_pds_pd
            .push((entity_id, name, description, target_ref));
        ctx.nauo_pds_pd_refs.insert(entity_id);
        return;
    } else {
        eprintln!(
            "warning: PROPERTY_DEFINITION #{entity_id} target #{target_ref} \
             resolves to no supported characterized_definition member \
             (PRODUCT_DEFINITION / SHAPE_ASPECT / PRODUCT_DEFINITION_SHAPE / \
             GENERAL_PROPERTY / DOCUMENT_FILE / CHARACTERIZED_ITEM_WITHIN_REPRESENTATION / \
             GEOMETRIC_TOLERANCE / DIMENSIONAL_SIZE) — skipping"
        );
        return;
    };
    ctx.property_def_map
        .insert(entity_id, (name.clone(), description.clone()));
    let arena_description = description.unwrap_or_default();
    let pd_id = ctx
        .properties
        .get_or_insert_with(PropertyPool::default)
        .property_definitions
        .push(PropertyDefinition::Itself(PropertyDefinitionData {
            name,
            description: arena_description,
            definition,
        }));
    ctx.id_cache.insert(entity_id, pd_id);
}
