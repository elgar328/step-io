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
use crate::parser::entity::Attribute;
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
        early: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        // Classifier (PDEF- vs NAUO-bearing) keyed on the *target's* entity
        // type. Bind this PDS's own attrs (not a cross-walk); the target name is
        // read through the L1 facade. A malformed PDS (bind fail / unresolved
        // target) is a no-op, never an error.
        let Ok(pds) = bind::bind_product_definition_shape(entity_id, attrs) else {
            return Ok(());
        };
        let Some(target_name) = early.type_name(pds.definition) else {
            return Ok(());
        };
        if PDEF_TARGET_NAMES.contains(&target_name) {
            // L1 name/description are deliberately dropped in `lower` (the
            // writer synthesises empty strings — legacy behavior).
            lower::lower_product_definition_shape(ctx, entity_id, pds.definition);
        } else if target_name == "NEXT_ASSEMBLY_USAGE_OCCURRENCE" {
            // Capture the target NAUO plus the source name/description so
            // `materialize_nauo_owned_pds` and the assembly-chain emit preserve
            // the exporter's placement label instead of the "Placement" fallback.
            ctx.nauo_pds_info.insert(
                entity_id,
                crate::reader::NauoPdsInfo {
                    nauo: pds.definition,
                    name: pds.name,
                    description: pds.description.unwrap_or_default(),
                },
            );
        }
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ProductDefinitionShapeWriteInput { pdef }: ProductDefinitionShapeWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_product_definition_shape(String::new(), Some(String::new()), pdef);
        Ok(serialize::serialize_product_definition_shape(buf, &early))
    }
}
