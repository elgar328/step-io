//! Assembly/product-domain `lift` fns (the product 2-hop cluster). See the
//! [module docs](super) for the lifting contract.
//!
//! The writer's assembly chain hands each handler a pre-resolved `WriteInput`
//! (output step ids already allocated by `emit_assembly_chain`), so these
//! lifts are pure shape adapters onto the generated L1 types.

use crate::early::model::{
    EarlyProduct, EarlyProductDefinition, EarlyProductDefinitionFormation,
    EarlyProductDefinitionFormationWithSpecifiedSource,
    EarlyProductDefinitionWithAssociatedDocuments, EarlySource,
};
use crate::entities::assembly_product::product_definition::ProductDefinitionWriteInput;
use crate::entities::assembly_product::product_definition_formation::ProductDefinitionFormationWriteInput;
use crate::entities::assembly_product::product_definition_formation_with_source::ProductDefinitionFormationWithSourceWriteInput;
use crate::entities::assembly_product::product_definition_with_associated_documents::ProductDefinitionWithAssociatedDocumentsWriteInput;
use crate::ir::assembly::Product;
use crate::writer::buffer::assembly::AssemblyContextIds;

/// Parse the L2 `make_or_buy` token back into the L1 enum. The reader only
/// ever stores the three legal tokens (unknown input already collapsed to
/// `NOT_KNOWN`), so the fallback is defensive.
fn source_from_token(t: &str) -> EarlySource {
    match t {
        "MADE" => EarlySource::Made,
        "BOUGHT" => EarlySource::Bought,
        _ => EarlySource::NotKnown,
    }
}

/// Lift one `PRODUCT`. The legacy writer emitted a missing description as
/// `''` (not `$`), so the faithful-optional L1 field is always `Some`.
pub(crate) fn lift_product(product: Product, ctx: &AssemblyContextIds) -> EarlyProduct {
    EarlyProduct {
        id: product.id,
        name: product.name,
        description: Some(product.description.unwrap_or_default()),
        frame_of_reference: vec![ctx.product_ctx],
    }
}

/// Lift one `PRODUCT_DEFINITION_FORMATION` write input.
pub(crate) fn lift_product_definition_formation(
    input: ProductDefinitionFormationWriteInput,
) -> EarlyProductDefinitionFormation {
    EarlyProductDefinitionFormation {
        id: input.id,
        // Legacy writer emitted the (possibly empty) string, never `$`.
        description: Some(input.description),
        of_product: input.prod_entity,
    }
}

/// Lift one `PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE` write input.
pub(crate) fn lift_product_definition_formation_with_specified_source(
    input: ProductDefinitionFormationWithSourceWriteInput,
) -> EarlyProductDefinitionFormationWithSpecifiedSource {
    EarlyProductDefinitionFormationWithSpecifiedSource {
        id: input.id,
        description: Some(input.description),
        of_product: input.prod_entity,
        make_or_buy: source_from_token(&input.make_or_buy),
    }
}

/// Lift one `PRODUCT_DEFINITION` write input.
pub(crate) fn lift_product_definition(
    input: ProductDefinitionWriteInput,
) -> EarlyProductDefinition {
    EarlyProductDefinition {
        id: input.id,
        description: Some(input.description),
        formation: input.formation,
        frame_of_reference: input.pdef_ctx,
    }
}

/// Lift one `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS` write input.
pub(crate) fn lift_product_definition_with_associated_documents(
    input: ProductDefinitionWithAssociatedDocumentsWriteInput,
) -> EarlyProductDefinitionWithAssociatedDocuments {
    EarlyProductDefinitionWithAssociatedDocuments {
        id: input.id,
        description: Some(input.description),
        formation: input.formation,
        frame_of_reference: input.pdef_ctx,
        documentation_ids: input.documentation,
    }
}
