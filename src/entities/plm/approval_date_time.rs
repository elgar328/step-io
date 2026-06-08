//! `APPROVAL_DATE_TIME` handler plm. Depends on
//! the `APPROVAL` handler (`dated_approval` ref) and the `DATE_AND_TIME`
//! handler (`date_time` SELECT).

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::plm::{ApprovalDateTime, ApprovalDateTimeSelect, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ApprovalDateTimeHandler;

#[step_entity(name = "APPROVAL_DATE_TIME")]
impl SimpleEntityHandler for ApprovalDateTimeHandler {
    type WriteInput = ApprovalDateTime;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "APPROVAL_DATE_TIME")?;
        let dt_ref = read_entity_ref(attrs, 0, entity_id, "date_time")?;
        let approval_ref = read_entity_ref(attrs, 1, entity_id, "dated_approval")?;
        let Some(dt_id) = ctx.id_cache.get::<crate::ir::DateAndTimeId>(dt_ref) else {
            // Unsupported SELECT variant (direct CALENDAR_DATE / LOCAL_TIME).
            return Ok(());
        };
        let Some(dated_approval) = ctx.id_cache.get::<crate::ir::ApprovalId>(approval_ref) else {
            return Ok(());
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.approval_date_times.push(ApprovalDateTime {
            date_time: ApprovalDateTimeSelect::DateAndTime(dt_id),
            dated_approval,
        });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, a: ApprovalDateTime) -> Result<u64, WriteError> {
        let dt_step = match a.date_time {
            ApprovalDateTimeSelect::DateAndTime(id) => buf.step_id(id),
        };
        let approval_step = buf.step_id(a.dated_approval);
        Ok(buf.push_simple(
            "APPROVAL_DATE_TIME",
            vec![
                Attribute::EntityRef(dt_step),
                Attribute::EntityRef(approval_step),
            ],
        ))
    }
}
