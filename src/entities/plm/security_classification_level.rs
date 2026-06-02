//! `SECURITY_CLASSIFICATION_LEVEL` handler — Pass 9-12 plm Security leaf.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{PlmPool, SecurityClassificationLevel};
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
        check_count(attrs, 1, entity_id, "SECURITY_CLASSIFICATION_LEVEL")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool
            .security_classification_levels
            .push(SecurityClassificationLevel { name });
        ctx.plm_security_level_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, l: SecurityClassificationLevel) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "SECURITY_CLASSIFICATION_LEVEL",
            vec![Attribute::String(l.name)],
        ))
    }
}
