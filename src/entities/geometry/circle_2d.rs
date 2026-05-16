//! `CIRCLE` handler — Pass 4a-3 (2D, pcurve subtree).

use crate::entities::geometry::axis2_placement_2d::Axis2Placement2dHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Circle2, Curve2d};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct Circle2dHandler;

impl SimpleEntityHandler for Circle2dHandler {
    const NAME: &'static str = "CIRCLE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4aCurve;
    type WriteInput = Circle2;

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
        // First cross-ref discriminates 2D vs 3D: if the placement is
        // absent from the 2D arena, this is the 3D CIRCLE.
        let Some(&position) = ctx.placement_2d_map.get(&pos_ref) else {
            return Ok(());
        };
        let id = ctx
            .geometry
            .curves_2d
            .push(Curve2d::Circle(Circle2 { position, radius }));
        ctx.curve_2d_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: Circle2) -> Result<u64, WriteError> {
        let pos = Axis2Placement2dHandler::write(buf, c.position)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "CIRCLE".into(),
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
static CIRCLE_2D_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: Circle2dHandler::NAME,
    pass_level: Circle2dHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: Circle2dHandler::read,
    },
};
