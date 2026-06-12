//! `SECURITY_CLASSIFICATION_LEVEL` handler — plm metadata leaf (2-layer path: generated
//! bind/serialize + pass-through lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::SecurityClassificationLevel;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SecurityClassificationLevelHandler;

#[step_entity(name = "SECURITY_CLASSIFICATION_LEVEL")]
impl SimpleEntityHandler for SecurityClassificationLevelHandler {
    type WriteInput = SecurityClassificationLevel;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_security_classification_level(entity_id, attrs)?;
        lower::lower_security_classification_level(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, v: SecurityClassificationLevel) -> Result<u64, WriteError> {
        let early = lift::lift_security_classification_level(v);
        Ok(serialize::serialize_security_classification_level(
            buf, &early,
        ))
    }
}
