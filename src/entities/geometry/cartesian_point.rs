//! `CARTESIAN_POINT` handler leaf 3D point.
//!
//! Mirrors the legacy `ReaderContext::convert_cartesian_point` and
//! `WriteBuffer::emit_point` one-to-one. The writer entry point keeps
//! `emit_point` as a thin wrapper because callers in adjacent emit
//! functions reference it directly.

use crate::entities::SimpleEntityHandler;
use crate::ir::PointId;
use crate::ir::attr::{check_count, read_real_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Point3;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct CartesianPointHandler;

#[step_entity(name = "CARTESIAN_POINT")]
impl SimpleEntityHandler for CartesianPointHandler {
    type WriteInput = PointId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "CARTESIAN_POINT")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let coords = read_real_list(attrs, 1, entity_id, "coordinates")?;
        match coords.len() {
            3 => {}             // proceed
            2 => return Ok(()), // 2D sister handler claims this entity
            n => {
                // Outside the 2/3 range allowed by STEP — surface a
                // diagnostic so the malformed entity is not lost
                // silently between the 2D and 3D handlers.
                return Err(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("CARTESIAN_POINT must have 2 or 3 coordinates, got {n}"),
                });
            }
        }
        let point = Point3 {
            x: coords[0],
            y: coords[1],
            z: coords[2],
        };
        let id = ctx.geometry.points.push(point);
        ctx.point_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: PointId) -> Result<u64, WriteError> {
        if let Some(&n) = buf.point_ids.get(&id) {
            return Ok(n);
        }
        let p = buf
            .model
            .geometry
            .points
            .iter()
            .nth(id.0 as usize)
            .copied()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("PointId({})", id.0),
            })?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "CARTESIAN_POINT".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(vec![
                        Attribute::Real(p.x),
                        Attribute::Real(p.y),
                        Attribute::Real(p.z),
                    ]),
                ],
            },
        });
        buf.point_ids.insert(id, n);
        Ok(n)
    }
}
