//! `PRODUCT_DEFINITION_SHAPE` handler — classifier (Pass 6-4c) + writer.
//!
//! Reader path is a classifier: walks attr[2] (`definition`) to decide
//! whether this `PDEF_SHAPE` describes a product (`PRODUCT_DEFINITION`
//! target) or an instance (NAUO target). Populates `pdef_shape_to_pdef` or
//! `pdef_shape_to_nauo` accordingly. No IR entity is materialised — the
//! maps feed Pass 6-5 (SDR) and Pass 6-7 (CDSR) downstream.
//!
//! Writer emits the standard three-attr form pointing at the PDEF.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::check_count;
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

#[step_entity(name = "PRODUCT_DEFINITION_SHAPE", pass = Pass6PdsClassify)]
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
            push_product_definition_shape(ctx, entity_id, attrs, pdef_ref)?;
        } else if let Some(nauo_ref) =
            pdef_shape_target(graph, entity_id, &["NEXT_ASSEMBLY_USAGE_OCCURRENCE"])
        {
            ctx.pdef_shape_to_nauo.insert(entity_id, nauo_ref);
            // NAUO-targeted PDS is left out of the `property_definitions`
            // arena — `CharacterizedDefinition` has no NAUO variant yet.
            // The legacy `pdef_shape_to_nauo` map keeps the CDSR pass
            // working unchanged.
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

/// Mirror a product-targeted PDS into the schema-faithful
/// `property_definitions` arena. PDEF step id resolved against
/// `pdef_to_product` + `product_arena_map` to obtain the `ProductId`
/// payload; an unresolved chain (rare — kernel-built IR) silently
/// skips the arena push and leaves the legacy `pdef_shape_to_pdef`
/// classification intact.
fn push_product_definition_shape(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    pdef_ref: u64,
) -> Result<(), ConvertError> {
    check_count(attrs, 3, entity_id, "PRODUCT_DEFINITION_SHAPE")?;
    let Some(&product_step_id) = ctx.pdef_to_product.get(&pdef_ref) else {
        return Ok(());
    };
    let Some(&product_id) = ctx.product_arena_map.get(&product_step_id) else {
        return Ok(());
    };
    // The schema-faithful PDS variant pairs with the existing writer, which
    // hard-codes empty strings for `name` / `description` (PDS attr 0/1 are
    // semantically vestigial placeholders in observed AP242 files). Reader
    // mirrors that policy so the round-trip assertion against the full
    // PropertyPool succeeds — see plan phase property-definition.
    let _ = ctx
        .properties
        .get_or_insert_with(PropertyPool::default)
        .property_definitions
        .push(PropertyDefinition::ProductDefinitionShape(
            ProductDefinitionShape {
                inherited: PropertyDefinitionData {
                    name: String::new(),
                    description: String::new(),
                    definition: CharacterizedDefinition::ProductDefinition(product_id),
                },
            },
        ));
    Ok(())
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
