//! Assembly/product-domain `lift` fns (the product 2-hop cluster). See the
//! [module docs](super) for the lifting contract.
//!
//! The writer's assembly chain hands each handler a pre-resolved `WriteInput`
//! (output step ids already allocated by `emit_assembly_chain`), so these
//! lifts are pure shape adapters onto the generated L1 types.

use crate::early::model::{
    EarlyMakeFromUsageOption, EarlyProduct, EarlyProductCategory, EarlyProductDefinition,
    EarlyProductDefinitionFormation, EarlyProductDefinitionFormationWithSpecifiedSource,
    EarlyProductDefinitionRelationship, EarlyProductDefinitionWithAssociatedDocuments, EarlySource,
};
use crate::entities::assembly_product::context_dependent_shape_representation::ContextDependentShapeRepresentationWriteInput;
use crate::entities::assembly_product::next_assembly_usage_occurrence::NextAssemblyUsageOccurrenceWriteInput;
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

/// Lift one `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION` write input (pure
/// two-ref shape adapter; `emit_instance_bundle` pre-resolves both step ids).
pub(crate) fn lift_context_dependent_shape_representation(
    input: ContextDependentShapeRepresentationWriteInput,
) -> crate::early::model::EarlyContextDependentShapeRepresentation {
    crate::early::model::EarlyContextDependentShapeRepresentation {
        representation_relation: input.rrwt,
        represented_product_relation: input.nauo_pds,
    }
}

/// Lift one `NEXT_ASSEMBLY_USAGE_OCCURRENCE` write input. Both legacy emit
/// paths always wrote `description` as a String (`''` for empty, never `$`),
/// so the faithful-optional L1 field is always `Some`; `reference_designator`
/// passes through (`None` → `$`, matching the legacy Unset).
pub(crate) fn lift_next_assembly_usage_occurrence(
    input: NextAssemblyUsageOccurrenceWriteInput,
) -> crate::early::model::EarlyNextAssemblyUsageOccurrence {
    crate::early::model::EarlyNextAssemblyUsageOccurrence {
        id: input.id,
        name: input.name,
        description: Some(input.description),
        relating_product_definition: input.relating,
        related_product_definition: input.related,
        reference_designator: input.reference_designator,
    }
}

/// Lift one `PRODUCT_DEFINITION_SHAPE` write input (just the PDEF step ref —
/// the legacy writer synthesises empty `name`/`description`, and the
/// faithful-optional L1 description is therefore always `Some("")`).
pub(crate) fn lift_product_definition_shape(
    pdef: u64,
) -> crate::early::model::EarlyProductDefinitionShape {
    crate::early::model::EarlyProductDefinitionShape {
        name: String::new(),
        description: Some(String::new()),
        definition: pdef,
    }
}

/// Lift one `PRODUCT_CATEGORY` (faithful optional description — the legacy
/// writer emitted `None` as `$`).
pub(crate) fn lift_product_category(
    name: String,
    description: Option<String>,
) -> EarlyProductCategory {
    EarlyProductCategory { name, description }
}

/// Lift one `MAKE_FROM_USAGE_OPTION` (refs pre-resolved).
pub(crate) fn lift_make_from_usage_option(
    mfu: crate::ir::assembly::MakeFromUsageOption,
    relating_product_definition: u64,
    related_product_definition: u64,
    quantity: u64,
) -> EarlyMakeFromUsageOption {
    EarlyMakeFromUsageOption {
        id: mfu.id,
        name: mfu.name,
        description: mfu.description,
        relating_product_definition,
        related_product_definition,
        ranking: mfu.ranking,
        ranking_rationale: mfu.ranking_rationale,
        quantity,
    }
}

/// Lift one plain `PRODUCT_DEFINITION_RELATIONSHIP` (refs pre-resolved).
pub(crate) fn lift_product_definition_relationship(
    plain: crate::ir::assembly::PlainProductDefinitionRelationship,
    relating_product_definition: u64,
    related_product_definition: u64,
) -> EarlyProductDefinitionRelationship {
    EarlyProductDefinitionRelationship {
        id: plain.id,
        name: plain.name,
        description: plain.description,
        relating_product_definition,
        related_product_definition,
    }
}
