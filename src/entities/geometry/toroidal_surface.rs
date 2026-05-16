//! `TOROIDAL_SURFACE` handler — Pass 4-1 leaf surface.

use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Surface, ToroidalSurface};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct ToroidalSurfaceHandler;

impl SimpleEntityHandler for ToroidalSurfaceHandler {
    const NAME: &'static str = "TOROIDAL_SURFACE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4Leaf;
    type WriteInput = ToroidalSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "TOROIDAL_SURFACE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
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
        ctx.surface_map.insert(entity_id, id);
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static TOROIDAL_SURFACE_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ToroidalSurfaceHandler::NAME,
    pass_level: ToroidalSurfaceHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ToroidalSurfaceHandler::read,
    },
};
