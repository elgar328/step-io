//! `SECURITY_CLASSIFICATION` handler — Pass 9-13 plm. Depends on
//! `Pass9PlmSecLevel` for the `security_level` ref.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::ir::plm::{PlmPool, SecurityClassification};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SecurityClassificationHandler;

#[step_entity(name = "SECURITY_CLASSIFICATION", pass = Pass9PlmSecClass)]
impl SimpleEntityHandler for SecurityClassificationHandler {
    type WriteInput = SecurityClassification;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SECURITY_CLASSIFICATION")?;
        let name = read_string(attrs, 0, entity_id, "name")?.to_owned();
        let purpose = read_string(attrs, 1, entity_id, "purpose")?.to_owned();
        let level_ref = read_entity_ref(attrs, 2, entity_id, "security_level")?;
        let Some(&security_level) = ctx.plm_security_level_id_map.get(&level_ref) else {
            return Ok(());
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.security_classifications.push(SecurityClassification {
            name,
            purpose,
            security_level,
        });
        ctx.plm_security_classification_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, s: SecurityClassification) -> Result<u64, WriteError> {
        let level_step = buf.plm_security_level_step_ids[s.security_level.0 as usize];
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
