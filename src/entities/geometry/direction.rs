//! DIRECTION handler.
//!
//! Mirrors the legacy `ReaderContext::convert_direction` (`reader/geometry.rs`)
//! and `WriteBuffer::emit_direction` (`writer/buffer/geometry.rs`) one-to-one.
//! `SimpleEntityHandler` impl + `ReadKind::Simple` registry submission.

use crate::entities::SimpleEntityHandler;
use crate::ir::DirectionId;
use crate::ir::attr::{check_count, read_real_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Direction3;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::buffer::geometry::direction_at;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct DirectionHandler;

#[step_entity(name = "DIRECTION", pass = Pass1)]
impl SimpleEntityHandler for DirectionHandler {
    type WriteInput = DirectionId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "DIRECTION")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let ratios = read_real_list(attrs, 1, entity_id, "direction_ratios")?;
        match ratios.len() {
            3 => {}             // proceed
            2 => return Ok(()), // 2D sister handler claims this entity
            n => {
                return Err(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("DIRECTION must have 2 or 3 direction_ratios, got {n}"),
                });
            }
        }
        let dir = Direction3 {
            x: ratios[0],
            y: ratios[1],
            z: ratios[2],
        };
        let id = ctx.geometry.directions.push(dir);
        ctx.direction_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: DirectionId) -> Result<u64, WriteError> {
        if let Some(&n) = buf.direction_ids.get(&id) {
            return Ok(n);
        }
        let d = direction_at(buf.model, id)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "DIRECTION".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(vec![
                        Attribute::Real(d.x),
                        Attribute::Real(d.y),
                        Attribute::Real(d.z),
                    ]),
                ],
            },
        });
        buf.direction_ids.insert(id, n);
        Ok(n)
    }
}
