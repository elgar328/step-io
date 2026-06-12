//! `PRODUCT_DEFINITION_SHAPE` handler — classifier + writer.
//!
//! Reader path is a classifier: walks attr[2] (`definition`) to decide
//! whether this `PDEF_SHAPE` describes a product (`PRODUCT_DEFINITION`
//! target) or an instance (NAUO target). Populates `pdef_shape_to_pdef` or
//! `pdef_shape_to_nauo` accordingly. No IR entity is materialised — the
//! maps feed the `SHAPE_DEFINITION_REPRESENTATION` and
//! `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION` handlers downstream.
//!
//! Writer emits the standard three-attr form pointing at the PDEF.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::read_string_or_unset;
use crate::ir::error::ConvertError;
use crate::ir::property::{
    CharacterizedDefinition, ProductDefinitionShape, PropertyDefinition, PropertyDefinitionData,
    PropertyPool,
};
use crate::parser::entity::{Attribute, EntityGraph, RawEntity};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// Accepted entity-type names for the `definition` slot of a
/// `PRODUCT_DEFINITION_SHAPE` when classified as product-bearing.
/// `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS` is an AP203/AP242 subtype
/// the reader treats identically.
const PDEF_TARGET_NAMES: &[&str] = &[
    "PRODUCT_DEFINITION",
    "PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS",
];

pub(crate) struct ProductDefinitionShapeWriteInput {
    pub(crate) pdef: u64,
}

pub(crate) struct ProductDefinitionShapeHandler;

#[step_entity(name = "PRODUCT_DEFINITION_SHAPE")]
impl SimpleEntityHandler for ProductDefinitionShapeHandler {
    type WriteInput = ProductDefinitionShapeWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        if let Some(pdef_ref) = pdef_shape_target(graph, entity_id, PDEF_TARGET_NAMES) {
            ctx.pdef_shape_to_pdef.insert(entity_id, pdef_ref);
            // Mirror the product-targeted PDS into the schema-faithful
            // `property_definitions` arena so the writer's arena-driven
            // emit sees it. The NAUO-targeted PDS is materialised later, in
            // `materialize_nauo_owned_pds`, once the ACU arena exists.
            {
                if let Some(product_id) = ctx.product_of_pdef(pdef_ref) {
                    let pd_id = ctx
                        .properties
                        .get_or_insert_with(PropertyPool::default)
                        .property_definitions
                        .push(PropertyDefinition::ProductDefinitionShape(
                            ProductDefinitionShape {
                                inherited: PropertyDefinitionData {
                                    name: String::new(),
                                    description: String::new(),
                                    definition: CharacterizedDefinition::ProductDefinition(
                                        product_id,
                                    ),
                                },
                            },
                        ));
                    ctx.id_cache.insert(entity_id, pd_id);
                }
            }
        } else if let Some(nauo_ref) =
            pdef_shape_target(graph, entity_id, &["NEXT_ASSEMBLY_USAGE_OCCURRENCE"])
        {
            ctx.pdef_shape_to_nauo.insert(entity_id, nauo_ref);
            // Capture the source name/description so `materialize_nauo_owned_pds`
            // and the assembly-chain emit preserve the exporter's placement
            // label instead of the hard-coded "Placement" fallback.
            let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
            let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
            ctx.pdef_shape_nauo_name_desc
                .insert(entity_id, (name, description));
        }
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ProductDefinitionShapeWriteInput { pdef }: ProductDefinitionShapeWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PRODUCT_DEFINITION_SHAPE",
            vec![
                Attribute::String(String::new()),
                Attribute::String(String::new()),
                Attribute::EntityRef(pdef),
            ],
        ))
    }
}

/// Resolve a `PRODUCT_DEFINITION_SHAPE`'s `definition` ref if it points at
/// one of `accepts`. Returns the target's STEP id, or `None` for missing /
/// wrong-typed / malformed `PDEF_SHAPE`.
fn pdef_shape_target(graph: &EntityGraph, pdef_shape_ref: u64, accepts: &[&str]) -> Option<u64> {
    let attrs = match graph.get(pdef_shape_ref)? {
        RawEntity::Simple {
            name, attributes, ..
        } if name == "PRODUCT_DEFINITION_SHAPE" => attributes.as_slice(),
        _ => return None,
    };
    // PDEF_SHAPE attr[2] = definition (PRODUCT_DEFINITION or NAUO).
    let def_ref = match attrs.get(2) {
        Some(Attribute::EntityRef(n)) => *n,
        _ => return None,
    };
    match graph.get(def_ref)? {
        RawEntity::Simple { name, .. } if accepts.iter().any(|t| *t == name) => Some(def_ref),
        _ => None,
    }
}
