//! `DIRECTION` handler (2D, pcurve subtree).
//!
//! Sister handler of [`crate::entities::geometry::direction::DirectionHandler`].

use crate::entities::SimpleEntityHandler;
use crate::ir::Direction2dId;
use crate::ir::attr::{check_count, read_real_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Direction2;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct Direction2dHandler;

#[step_entity(name = "DIRECTION")]
impl SimpleEntityHandler for Direction2dHandler {
    type WriteInput = Direction2dId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "DIRECTION")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let coords = read_real_list(attrs, 1, entity_id, "direction_ratios")?;
        if coords.len() != 2 {
            // Wrong dimension for the 2D arena. The 3D sister handler
            // claims 3-component DIRECTIONs; anything else is silently
            // dropped here.
            return Ok(());
        }
        let id = ctx.geometry.directions_2d.push(Direction2 {
            x: coords[0],
            y: coords[1],
        });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: Direction2dId) -> Result<u64, WriteError> {
        if let Some(&n) = buf.direction_2d_ids.get(&id) {
            return Ok(n);
        }
        let d = buf
            .model
            .geometry
            .directions_2d
            .iter()
            .nth(id.0 as usize)
            .copied()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("Direction2dId({})", id.0),
            })?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "DIRECTION".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(vec![Attribute::Real(d.x), Attribute::Real(d.y)]),
                ],
            },
        });
        buf.direction_2d_ids.insert(id, n);
        Ok(n)
    }
}
