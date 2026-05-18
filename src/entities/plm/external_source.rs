//! `EXTERNAL_SOURCE` handler — Pass 9-15 plm Identification leaf.
//! `source_id` is the AP214 `source_item` SELECT — step-io reads the
//! `IDENTIFIER` variant only; other variants (`MESSAGE`, ...) drop.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::plm::{ExternalSource, ExternalSourceItem, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ExternalSourceHandler;

#[step_entity(name = "EXTERNAL_SOURCE", pass = Pass9PlmIdLeaves)]
impl SimpleEntityHandler for ExternalSourceHandler {
    type WriteInput = ExternalSource;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "EXTERNAL_SOURCE")?;
        let source_id = match &attrs[0] {
            Attribute::Typed { type_name, value } if type_name == "IDENTIFIER" => {
                match value.as_ref() {
                    Attribute::String(s) => ExternalSourceItem::Identifier(s.clone()),
                    _ => return Ok(()),
                }
            }
            _ => return Ok(()),
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.external_sources.push(ExternalSource { source_id });
        ctx.plm_external_source_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, e: ExternalSource) -> Result<u64, WriteError> {
        let item = match e.source_id {
            ExternalSourceItem::Identifier(s) => Attribute::Typed {
                type_name: "IDENTIFIER".to_string(),
                value: Box::new(Attribute::String(s)),
            },
        };
        Ok(buf.push_simple("EXTERNAL_SOURCE", vec![item]))
    }
}
