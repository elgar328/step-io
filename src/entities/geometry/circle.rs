//! `CIRCLE` handler — Pass 4-1 leaf 3D circle.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Circle3, Curve};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct CircleHandler;

#[step_entity(name = "CIRCLE", pass = Pass4Leaf)]
impl SimpleEntityHandler for CircleHandler {
    type WriteInput = Circle3;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "CIRCLE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let radius = read_real(attrs, 2, entity_id, "radius")?;

        // If the placement is a known 2D placement, this CIRCLE is the
        // 2D sister variant — silently skip.
        if ctx.placement_2d_map.contains_key(&pos_ref) {
            return Ok(());
        }
        let position = ctx.resolve_placement(entity_id, pos_ref, "position")?;

        let circle = Circle3 { position, radius };
        let id = ctx.geometry.curves.push(Curve::Circle(circle));
        ctx.curve_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, circle: Circle3) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, circle.position)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "CIRCLE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(circle.radius),
                ],
            },
        });
        Ok(n)
    }
}
