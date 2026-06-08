//! `TOROIDAL_SURFACE` handler leaf surface.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Surface, ToroidalSurface};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct ToroidalSurfaceHandler;

#[step_entity(name = "TOROIDAL_SURFACE")]
impl SimpleEntityHandler for ToroidalSurfaceHandler {
    type WriteInput = ToroidalSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "TOROIDAL_SURFACE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let major_radius = read_real(attrs, 2, entity_id, "major_radius")?;
        let minor_radius = read_real(attrs, 3, entity_id, "minor_radius")?;

        let position = ctx.resolve_placement(entity_id, pos_ref, "position")?;

        let surface = ToroidalSurface {
            position,
            major_radius,
            minor_radius,
        };
        let id = ctx.geometry.surfaces.push(Surface::Torus(surface));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, t: ToroidalSurface) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, t.position)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "TOROIDAL_SURFACE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(t.major_radius),
                    Attribute::Real(t.minor_radius),
                ],
            },
        });
        Ok(n)
    }
}
