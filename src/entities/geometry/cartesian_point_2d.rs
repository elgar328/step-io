//! `CARTESIAN_POINT` handler — Pass 1 (2D variant).
//!
//! Sister handler of [`crate::entities::geometry::cartesian_point::CartesianPointHandler`].
//! Same STEP entity name; both run in Pass 1 alongside each other and
//! select which arena receives the entity by coordinate count. A
//! 2-coordinate point goes to `geometry.points_2d`; a 3-coordinate point
//! goes to `geometry.points`. Wrong-dimension or malformed inputs land
//! in no arena (silent skip).

use crate::entities::SimpleEntityHandler;
use crate::ir::Point2dId;
use crate::ir::attr::{check_count, read_real_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Point2;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct CartesianPoint2dHandler;

#[step_entity(name = "CARTESIAN_POINT", pass = Pass1)]
impl SimpleEntityHandler for CartesianPoint2dHandler {
    type WriteInput = Point2dId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "CARTESIAN_POINT")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let coords = read_real_list(attrs, 1, entity_id, "coordinates")?;
        if coords.len() != 2 {
            // Wrong dimension for the 2D arena. The 3D sister handler
            // claims 3-coordinate points; anything else is silently
            // dropped here.
            return Ok(());
        }
        let id = ctx.geometry.points_2d.push(Point2 {
            x: coords[0],
            y: coords[1],
        });
        ctx.point_2d_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: Point2dId) -> Result<u64, WriteError> {
        if let Some(&n) = buf.point_2d_ids.get(&id) {
            return Ok(n);
        }
        let p = buf
            .model
            .geometry
            .points_2d
            .iter()
            .nth(id.0 as usize)
            .copied()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("Point2dId({})", id.0),
            })?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "CARTESIAN_POINT".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(vec![Attribute::Real(p.x), Attribute::Real(p.y)]),
                ],
            },
        });
        buf.point_2d_ids.insert(id, n);
        Ok(n)
    }
}
