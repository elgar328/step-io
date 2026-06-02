//! `LINE` handler — Pass 4-1 leaf 3D line.
//!
//! Mirrors `ReaderContext::convert_line` and `WriteBuffer::emit_line`.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::entities::geometry::vector::VectorHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Curve, Line3};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct LineHandler;

#[step_entity(name = "LINE")]
impl SimpleEntityHandler for LineHandler {
    type WriteInput = Line3;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "LINE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let pnt_ref = read_entity_ref(attrs, 1, entity_id, "pnt")?;
        let dir_ref = read_entity_ref(attrs, 2, entity_id, "dir")?;

        // If the referenced point is a known 2D point, this LINE is
        // the 2D sister variant — silently skip.
        if ctx.point_2d_map.contains_key(&pnt_ref) {
            return Ok(());
        }
        let point = ctx.resolve_point(entity_id, pnt_ref, "pnt")?;
        let (direction, magnitude) = ctx.resolve_vector(entity_id, dir_ref, "dir")?;

        let line = Line3 {
            point,
            direction,
            magnitude,
        };
        let id = ctx.geometry.curves.push(Curve::Line(line));
        ctx.curve_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, line: Line3) -> Result<u64, WriteError> {
        let pnt = CartesianPointHandler::write(buf, line.point)?;
        let vec = VectorHandler::write(buf, (line.direction, line.magnitude))?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "LINE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pnt),
                    Attribute::EntityRef(vec),
                ],
            },
        });
        Ok(n)
    }
}
