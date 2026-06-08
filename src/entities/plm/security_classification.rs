//! `SECURITY_CLASSIFICATION` handler plm. Depends on
//! the security-level handler for the `security_level` ref.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{PlmPool, SecurityClassification};
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
        check_count(attrs, 3, entity_id, "SECURITY_CLASSIFICATION")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let purpose = read_string_or_unset(attrs, 1, entity_id, "purpose")?.to_owned();
        let level_ref = read_entity_ref(attrs, 2, entity_id, "security_level")?;
        let Some(security_level) = ctx
            .id_cache
            .get::<crate::ir::SecurityClassificationLevelId>(level_ref)
        else {
            return Ok(());
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.security_classifications.push(SecurityClassification {
            name,
            purpose,
            security_level,
        });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, s: SecurityClassification) -> Result<u64, WriteError> {
        let level_step = buf.step_id(s.security_level);
        Ok(buf.push_simple(
            "SECURITY_CLASSIFICATION",
            vec![
                Attribute::String(s.name),
                Attribute::String(s.purpose),
                Attribute::EntityRef(level_step),
            ],
        ))
    }
}
