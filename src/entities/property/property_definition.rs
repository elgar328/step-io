//! `PROPERTY_DEFINITION` handler — Pass 8-2.
//!
//! Reader stores `(name, description)` in `property_def_map` keyed by STEP
//! entity id. Targets that don't resolve to a product context (e.g. an
//! unresolved `SHAPE_ASPECT`) are still dropped at read time; product-free
//! targets (`GENERAL_PROPERTY`, `DOCUMENT_FILE`) have no product binding and
//! are kept.
//! Writer emits the bare PD line; the surrounding `REPRESENTATION` + PDR are
//! handled in `buffer/property.rs::emit_property` (the orchestrator).

use crate::entities::SimpleEntityHandler;
use crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref;
use crate::ir::ProductId;
use crate::ir::ShapeAspectRef;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::id::DimensionalLocationId;
use crate::ir::plm::Document;
use crate::ir::pmi::DimensionalLocation;
use crate::ir::property::{
    CharacterizedDefinition, PropertyDefinition, PropertyDefinitionData, PropertyPool,
};
use crate::ir::shape_rep::CharacterizedObject;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// Resolve the owning product for a `dimensional_location` arena entry —
/// a `shape_aspect_relationship` subtype, so the product is reached via
/// its `relating_shape_aspect` endpoint.
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
        ShapeAspectRef::DatumFeature(id) => ctx.pmi.as_ref().map(|p| p.datum_features[id].target),
        ShapeAspectRef::DatumSystem(id) => Some(ctx.datum_systems[id].target),
        ShapeAspectRef::DatumTarget(id) => Some(ctx.datum_targets[id].target),
        ShapeAspectRef::PlacedDatumTargetFeature(id) => {
            Some(ctx.placed_datum_target_features[id].target)
        }
    }
}

pub(crate) struct PropertyDefinitionWriteInput {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) pdef_id: u64,
}

pub(crate) struct PropertyDefinitionHandler;

#[step_entity(name = "PROPERTY_DEFINITION", pass = Pass8PropertyDef)]
impl SimpleEntityHandler for PropertyDefinitionHandler {
    type WriteInput = PropertyDefinitionWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "PROPERTY_DEFINITION")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let desc_str = read_string_or_unset(attrs, 1, entity_id, "description")?;
        let description = if desc_str.is_empty() {
            None
        } else {
            Some(desc_str.to_owned())
        };
        let target_ref = read_entity_ref(attrs, 2, entity_id, "definition")?;
        // characterized_definition SELECT (subset). Pattern A:
        // PRODUCT_DEFINITION (pdef_to_product). Pattern B: SHAPE_ASPECT
        // (shape_aspect_id_map). Pattern C: PRODUCT_DEFINITION_SHAPE — its
        // arena entry is pushed by Pass6PdsClassify with the
        // ProductDefinitionShape variant; resolved through pdef_shape_to_pdef
        // → pdef_to_product. Pattern D: GENERAL_PROPERTY — standalone, no
        // product binding. Product-bound patterns still gate on a resolvable
        // product (drop when unresolved); the resolved id is not stored.
        let definition = if let Some(&product_step_id) = ctx.pdef_to_product.get(&target_ref) {
            let Some(&pid) = ctx.product_arena_map.get(&product_step_id) else {
                return Ok(());
            };
            CharacterizedDefinition::ProductDefinition(pid)
        } else if let Some(sa_ref) = resolve_shape_aspect_ref(ctx, target_ref) {
            if shape_aspect_ref_target(ctx, sa_ref).is_none() {
                return Ok(());
            }
            CharacterizedDefinition::ShapeAspect(sa_ref)
        } else if let Some(&dl_id) = ctx.dimensional_location_id_map.get(&target_ref) {
            if dimensional_location_target(ctx, dl_id).is_none() {
                return Ok(());
            }
            CharacterizedDefinition::DimensionalLocation(dl_id)
        } else if let Some(&pds_pd_id) = ctx.property_def_step_to_id.get(&target_ref) {
            let Some(pool) = ctx.properties.as_ref() else {
                return Ok(());
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
                return Ok(());
            }
            let resolved = ctx
                .pdef_shape_to_pdef
                .get(&target_ref)
                .and_then(|pdef_ref| ctx.pdef_to_product.get(pdef_ref).copied())
                .and_then(|prod_step| ctx.product_arena_map.get(&prod_step).copied());
            if resolved.is_none() {
                return Ok(());
            }
            CharacterizedDefinition::ProductDefinitionShape(pds_pd_id)
        } else if let Some(&gp_id) = ctx.general_property_id_map.get(&target_ref) {
            CharacterizedDefinition::GeneralProperty(gp_id)
        } else if let Some(&doc_id) = ctx.plm_document_id_map.get(&target_ref) {
            // DOCUMENT_FILE is a characterized_object subtype (a valid
            // characterized_definition member); plain DOCUMENT is not.
            let is_file = ctx
                .plm
                .as_ref()
                .is_some_and(|p| matches!(p.documents[doc_id], Document::DocumentFile(_)));
            if !is_file {
                eprintln!(
                    "warning: PROPERTY_DEFINITION #{entity_id} target #{target_ref} \
                     resolves to a plain DOCUMENT (not a characterized_object) — skipping"
                );
                return Ok(());
            }
            CharacterizedDefinition::Document(doc_id)
        } else if let Some(&co_id) = ctx.characterized_object_id_map.get(&target_ref) {
            // CIWR is a characterized_object subtype (a valid
            // characterized_definition member) — geometric-validation shapes.
            if !matches!(
                ctx.characterized_objects[co_id],
                CharacterizedObject::CharacterizedItemWithinRepresentation(_)
            ) {
                return Ok(());
            }
            CharacterizedDefinition::CharacterizedItemWithinRepresentation(co_id)
        } else {
            eprintln!(
                "warning: PROPERTY_DEFINITION #{entity_id} target #{target_ref} \
                     resolves to no supported characterized_definition member \
                     (PRODUCT_DEFINITION / SHAPE_ASPECT / PRODUCT_DEFINITION_SHAPE / \
                     GENERAL_PROPERTY / DOCUMENT_FILE / CHARACTERIZED_ITEM_WITHIN_REPRESENTATION) \
                     — skipping"
            );
            return Ok(());
        };
        ctx.property_def_map
            .insert(entity_id, (name.clone(), description.clone()));
        // Schema-faithful `property_definitions` arena push (the writer's
        // sole PD emit source). `description` flattens Option → empty
        // string so the carrier struct uses raw `String`.
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
        ctx.property_def_step_to_id.insert(entity_id, pd_id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        PropertyDefinitionWriteInput {
            name,
            description,
            pdef_id,
        }: PropertyDefinitionWriteInput,
    ) -> Result<u64, WriteError> {
        let desc_attr = match description {
            Some(s) => Attribute::String(s),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "PROPERTY_DEFINITION",
            vec![
                Attribute::String(name),
                desc_attr,
                Attribute::EntityRef(pdef_id),
            ],
        ))
    }
}
