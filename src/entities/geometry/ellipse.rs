//! `ELLIPSE` handler leaf 3D ellipse.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Curve, Ellipse3};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct EllipseHandler;

#[step_entity(name = "ELLIPSE")]
impl SimpleEntityHandler for EllipseHandler {
    type WriteInput = Ellipse3;

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

        // If the placement is a known 2D placement, this ELLIPSE is
        // the 2D sister variant — silently skip.
        if ctx.placement_2d_map.contains_key(&pos_ref) {
            return Ok(());
        }
        let position = ctx.resolve_placement(entity_id, pos_ref, "position")?;

        let ellipse = Ellipse3 {
            position,
            semi_axis_1,
            semi_axis_2,
        };
        let id = ctx.geometry.curves.push(Curve::Ellipse(ellipse));
        ctx.curve_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ellipse: Ellipse3) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, ellipse.position)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "ELLIPSE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(ellipse.semi_axis_1),
                    Attribute::Real(ellipse.semi_axis_2),
                ],
            },
        });
        Ok(n)
    }
}
