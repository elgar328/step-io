//! Plain `REPRESENTATION_CONTEXT(context_identifier, context_type)` handler.
//!
//! A standalone simple `REPRESENTATION_CONTEXT` (no `GLOBAL_UNIT_ASSIGNED_CONTEXT`
//! and no `GEOMETRIC_REPRESENTATION_CONTEXT` parts) — e.g.
//! `REPRESENTATION_CONTEXT('','document parameters')` used by document-property
//! representations. Stored as a unit-less context with no
//! `coordinate_space_dimension` (the GRC+PRC complex form is handled by
//! [`super::parametric_representation_context`]). Dispatch routes on entity
//! form, so this only fires for simple instances — never for the
//! `REPRESENTATION_CONTEXT` part inside a complex.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::UnitlessContext;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct RepresentationContextHandler;

#[step_entity(name = "REPRESENTATION_CONTEXT")]
impl SimpleEntityHandler for RepresentationContextHandler {
    type WriteInput = UnitlessContext;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "REPRESENTATION_CONTEXT")?;
        let identifier =
            read_string_or_unset(attrs, 0, entity_id, "context_identifier")?.to_owned();
        let context_type = read_string_or_unset(attrs, 1, entity_id, "context_type")?.to_owned();
        let id = ctx.unitless_contexts.push(UnitlessContext {
            identifier,
            context_type,
            coordinate_space_dimension: None,
        });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, uc: UnitlessContext) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "REPRESENTATION_CONTEXT",
            vec![
                Attribute::String(uc.identifier),
                Attribute::String(uc.context_type),
            ],
        ))
    }
}
