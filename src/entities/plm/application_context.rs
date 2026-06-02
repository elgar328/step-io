//! `APPLICATION_CONTEXT` handler — Pass 9-25 plm. STEP positional
//! shape `(application)` per `AP214e3` schema. Leaf entity; refs nothing.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{ApplicationContext, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ApplicationContextHandler;

#[step_entity(name = "APPLICATION_CONTEXT")]
impl SimpleEntityHandler for ApplicationContextHandler {
    type WriteInput = ApplicationContext;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "APPLICATION_CONTEXT")?;
        let application = read_string_or_unset(attrs, 0, entity_id, "application")?.to_owned();
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool
            .application_contexts
            .push(ApplicationContext { application });
        ctx.plm_application_context_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ac: ApplicationContext) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "APPLICATION_CONTEXT",
            vec![Attribute::String(ac.application)],
        ))
    }
}
