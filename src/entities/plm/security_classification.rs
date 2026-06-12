//! `SECURITY_CLASSIFICATION` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::SecurityClassification;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SecurityClassificationHandler;

#[step_entity(name = "SECURITY_CLASSIFICATION")]
impl SimpleEntityHandler for SecurityClassificationHandler {
    type WriteInput = SecurityClassification;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_security_classification(entity_id, attrs)?;
        lower::lower_security_classification(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, s: SecurityClassification) -> Result<u64, WriteError> {
        let level_step = buf.step_id(s.security_level);
        let early = lift::lift_security_classification(s, level_step);
        Ok(serialize::serialize_security_classification(buf, &early))
    }
}
