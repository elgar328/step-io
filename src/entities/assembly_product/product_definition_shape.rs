//! `PRODUCT_DEFINITION_SHAPE` handler — classifier + writer.
//!
//! Reader path is a classifier: walks attr[2] (`definition`) to decide
//! whether this `PDEF_SHAPE` describes a product (`PRODUCT_DEFINITION`
//! target) or an instance (NAUO target). The product branch lowers to the
//! `property_definitions` arena and records the typed `product_of_pds`
//! correspondence; the NAUO branch populates `nauo_pds_info`. These
//! feed the `SHAPE_DEFINITION_REPRESENTATION` and
//! `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION` handlers downstream.
//!
//! Writer emits the standard three-attr form pointing at the PDEF.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
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
        // The classification (PDEF- vs NAUO-bearing) needs the *target's*
        // entity type, so it stays a lenient graph walk; a malformed PDS
        // (missing / wrong-typed definition) is a no-op, never an error.
        // The branches then run the 2-layer path on the well-formed attrs.
        if let Some(pdef_ref) = pdef_shape_target(graph, entity_id, PDEF_TARGET_NAMES) {
            // L1 name/description are deliberately dropped in `lower` (the
            // writer synthesises empty strings — legacy behavior).
            lower::lower_product_definition_shape(ctx, entity_id, pdef_ref);
        } else if let Some(nauo_ref) =
            pdef_shape_target(graph, entity_id, &["NEXT_ASSEMBLY_USAGE_OCCURRENCE"])
        {
            // Capture the target NAUO plus the source name/description so
            // `materialize_nauo_owned_pds` and the assembly-chain emit
            // preserve the exporter's placement label instead of the
            // hard-coded "Placement" fallback. (Bind is safe here: the branch
            // guarantees a well-formed entity ref at attr[2]; `$` descriptions
            // collapse to "" as before.)
            let early = bind::bind_product_definition_shape(entity_id, attrs)?;
            ctx.nauo_pds_info.insert(
                entity_id,
                crate::reader::NauoPdsInfo {
                    nauo: nauo_ref,
                    name: early.name,
                    description: early.description.unwrap_or_default(),
                },
            );
        }
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ProductDefinitionShapeWriteInput { pdef }: ProductDefinitionShapeWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_product_definition_shape(pdef);
        Ok(serialize::serialize_product_definition_shape(buf, &early))
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
