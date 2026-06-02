//! `ELLIPSE` handler — Pass 4a-3 (2D, pcurve subtree).

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_2d::Axis2Placement2dHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Curve2d, Ellipse2};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct Ellipse2dHandler;

#[step_entity(name = "ELLIPSE", is_2d)]
impl SimpleEntityHandler for Ellipse2dHandler {
    type WriteInput = Ellipse2;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "ELLIPSE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let semi_axis_1 = read_real(attrs, 2, entity_id, "semi_axis_1")?;
        let semi_axis_2 = read_real(attrs, 3, entity_id, "semi_axis_2")?;
        // First cross-ref discriminates 2D vs 3D: if the placement is
        // absent from the 2D arena, this is the 3D ELLIPSE.
        let Some(&position) = ctx.placement_2d_map.get(&pos_ref) else {
            return Ok(());
        };
        let id = ctx.geometry.curves_2d.push(Curve2d::Ellipse(Ellipse2 {
            position,
            semi_axis_1,
            semi_axis_2,
        }));
        ctx.curve_2d_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, e: Ellipse2) -> Result<u64, WriteError> {
        let pos = Axis2Placement2dHandler::write(buf, e.position)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "ELLIPSE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(e.semi_axis_1),
                    Attribute::Real(e.semi_axis_2),
                ],
            },
        });
        Ok(n)
    }
}
