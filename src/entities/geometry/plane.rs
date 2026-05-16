//! `PLANE` handler — Pass 4-1 leaf 3D plane.

use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Plane3, Surface};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct PlaneHandler;

impl SimpleEntityHandler for PlaneHandler {
    const NAME: &'static str = "PLANE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4Leaf;
    type WriteInput = Plane3;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PLANE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;

        let position = ctx.resolve_placement(entity_id, pos_ref, "position")?;

        let plane = Plane3 { position };
        let id = ctx.geometry.surfaces.push(Surface::Plane(plane));
        ctx.surface_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, p: Plane3) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, p.position)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "PLANE".into(),
                attrs: vec![Attribute::String(String::new()), Attribute::EntityRef(pos)],
            },
        });
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static PLANE_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: PlaneHandler::NAME,
    pass_level: PlaneHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: PlaneHandler::read,
    },
};
