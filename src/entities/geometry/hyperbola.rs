//! `HYPERBOLA` handler — Pass 4-1 leaf 3D hyperbola.
//!
//! `SUBTYPE OF CONIC`. Attributes: `(name, position, semi_axis,
//! semi_imag_axis)`. The 2D sister variant (placement in 2D arena) is
//! silently skipped — blueprint defines `arena = "curve"` (3D only).

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Curve, Hyperbola};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct HyperbolaHandler;

#[step_entity(name = "HYPERBOLA", pass = Pass4Leaf)]
impl SimpleEntityHandler for HyperbolaHandler {
    type WriteInput = Hyperbola;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "HYPERBOLA")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let semi_axis = read_real(attrs, 2, entity_id, "semi_axis")?;
        let semi_imag_axis = read_real(attrs, 3, entity_id, "semi_imag_axis")?;

        if ctx.placement_2d_map.contains_key(&pos_ref) {
            return Ok(());
        }
        let position = ctx.resolve_placement(entity_id, pos_ref, "position")?;

        let id = ctx.geometry.curves.push(Curve::Hyperbola(Hyperbola {
            position,
            semi_axis,
            semi_imag_axis,
        }));
        ctx.curve_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, h: Hyperbola) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, h.position)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "HYPERBOLA".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(h.semi_axis),
                    Attribute::Real(h.semi_imag_axis),
                ],
            },
        });
        Ok(n)
    }
}
