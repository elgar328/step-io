//! `PRODUCT_DEFINITION_RELATIONSHIP` handler.
//!
//! Reader resolves both `relating` / `related` PDEFs to [`ProductId`] via
//! [`ReaderContext::resolve_product_by_pdef`] and pushes a
//! `ProductDefinitionRelationship::Plain` entry into the assembly arena.
//! `MAKE_FROM_USAGE_OPTION` is the in-enum subtype, handled by its own
//! sibling handler with the same pass.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::PlainProductDefinitionRelationship;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ProductDefinitionRelationshipWriteInput {
    pub(crate) plain: PlainProductDefinitionRelationship,
    pub(crate) relating_pdef_step: u64,
    pub(crate) related_pdef_step: u64,
}

pub(crate) struct ProductDefinitionRelationshipHandler;

#[step_entity(name = "PRODUCT_DEFINITION_RELATIONSHIP")]
impl SimpleEntityHandler for ProductDefinitionRelationshipHandler {
    type WriteInput = ProductDefinitionRelationshipWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_product_definition_relationship(entity_id, attrs)?;
        lower::lower_product_definition_relationship(ctx, entity_id, early)
    }

    fn write(
        buf: &mut WriteBuffer,
        ProductDefinitionRelationshipWriteInput {
            plain,
            relating_pdef_step,
            related_pdef_step,
        }: ProductDefinitionRelationshipWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_product_definition_relationship(
            plain,
            relating_pdef_step,
            related_pdef_step,
        );
        Ok(serialize::serialize_product_definition_relationship(
            buf, &early,
        ))
    }
}
