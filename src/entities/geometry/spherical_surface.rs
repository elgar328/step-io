//! `SPHERICAL_SURFACE` handler — Pass 4-1 leaf surface.

use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{SphericalSurface, Surface};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct SphericalSurfaceHandler;

impl SimpleEntityHandler for SphericalSurfaceHandler {
    const NAME: &'static str = "SPHERICAL_SURFACE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4Leaf;
    type WriteInput = SphericalSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SPHERICAL_SURFACE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let radius = read_real(attrs, 2, entity_id, "radius")?;

        let position = ctx.resolve_placement(entity_id, pos_ref, "position")?;

        let surface = SphericalSurface { position, radius };
        let id = ctx.geometry.surfaces.push(Surface::Sphere(surface));
        ctx.surface_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, s: SphericalSurface) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, s.position)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "SPHERICAL_SURFACE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(s.radius),
                ],
            },
        });
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static SPHERICAL_SURFACE_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: SphericalSurfaceHandler::NAME,
    pass_level: SphericalSurfaceHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: SphericalSurfaceHandler::read,
    },
};
