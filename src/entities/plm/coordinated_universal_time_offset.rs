//! `COORDINATED_UNIVERSAL_TIME_OFFSET` handler — Pass 9-1 plm leaf.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_enum, read_integer, read_optional_integer};
use crate::ir::error::ConvertError;
use crate::ir::plm::{AheadOrBehind, CoordinatedUniversalTimeOffset, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CoordinatedUniversalTimeOffsetHandler;

#[step_entity(name = "COORDINATED_UNIVERSAL_TIME_OFFSET")]
impl SimpleEntityHandler for CoordinatedUniversalTimeOffsetHandler {
    type WriteInput = CoordinatedUniversalTimeOffset;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "COORDINATED_UNIVERSAL_TIME_OFFSET")?;
        let hour_offset = read_integer(attrs, 0, entity_id, "hour_offset")?;
        let minute_offset = read_optional_integer(attrs, 1, entity_id, "minute_offset")?;
        let sense_str = read_enum(attrs, 2, entity_id, "sense")?;
        let sense = match sense_str {
            "AHEAD" => AheadOrBehind::Ahead,
            "BEHIND" => AheadOrBehind::Behind,
            "EXACT" => AheadOrBehind::Exact,
            _ => return Ok(()), // unknown sense — drop entity
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.utc_offsets.push(CoordinatedUniversalTimeOffset {
            hour_offset,
            minute_offset,
            sense,
        });
        ctx.plm_utc_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        utc: CoordinatedUniversalTimeOffset,
    ) -> Result<u64, WriteError> {
        let sense = match utc.sense {
            AheadOrBehind::Ahead => "AHEAD",
            AheadOrBehind::Behind => "BEHIND",
            AheadOrBehind::Exact => "EXACT",
        };
        let minute_attr = match utc.minute_offset {
            Some(v) => Attribute::Integer(v),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "COORDINATED_UNIVERSAL_TIME_OFFSET",
            vec![
                Attribute::Integer(utc.hour_offset),
                minute_attr,
                Attribute::Enum(sense.into()),
            ],
        ))
    }
}
