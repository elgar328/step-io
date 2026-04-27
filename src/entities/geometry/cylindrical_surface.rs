//! `CYLINDRICAL_SURFACE` handler — Pass 4-1 leaf surface.

use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{CylindricalSurface, Surface};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct CylindricalSurfaceHandler;

impl SimpleEntityHandler for CylindricalSurfaceHandler {
    const NAME: &'static str = "CYLINDRICAL_SURFACE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4Leaf;
    type WriteInput = CylindricalSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "CYLINDRICAL_SURFACE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static CYLINDRICAL_SURFACE_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: CylindricalSurfaceHandler::NAME,
    pass_level: CylindricalSurfaceHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: CylindricalSurfaceHandler::read,
    },
};
