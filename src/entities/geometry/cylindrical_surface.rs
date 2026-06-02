//! `CYLINDRICAL_SURFACE` handler leaf surface.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{CylindricalSurface, Surface};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct CylindricalSurfaceHandler;

#[step_entity(name = "CYLINDRICAL_SURFACE")]
impl SimpleEntityHandler for CylindricalSurfaceHandler {
    type WriteInput = CylindricalSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "CYLINDRICAL_SURFACE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let radius = read_real(attrs, 2, entity_id, "radius")?;

        let position = ctx.resolve_placement(entity_id, pos_ref, "position")?;

        let surface = CylindricalSurface { position, radius };
        let id = ctx.geometry.surfaces.push(Surface::Cylinder(surface));
        ctx.surface_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: CylindricalSurface) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, c.position)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "CYLINDRICAL_SURFACE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(c.radius),
                ],
            },
        });
        Ok(n)
    }
}
