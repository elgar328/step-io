//! `EXTERNAL_SOURCE` handler plm Identification leaf (2-layer path).
//! `source_id` is the AP214 `source_item` SELECT (`IDENTIFIER`/`MESSAGE`),
//! codegen via the hinted `[select.source_item]` → `EarlySourceItem`. step-io
//! models only the `Identifier` member in L2; `lower` drops a `Message`
//! occurrence (unmodelled), symmetric on re-read.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::ExternalSource;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ExternalSourceHandler;

#[step_entity(name = "EXTERNAL_SOURCE")]
impl SimpleEntityHandler for ExternalSourceHandler {
    type WriteInput = ExternalSource;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(early) = bind::bind_external_source(entity_id, attrs)? else {
            return Ok(()); // source_item did not bind — drop the entity
        };
        lower::lower_external_source(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, e: ExternalSource) -> Result<u64, WriteError> {
        let early = lift::lift_external_source(&e.source_id);
        Ok(serialize::serialize_external_source(buf, &early))
    }
}
