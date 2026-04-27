//! `LINE` handler — Pass 4a-3 (2D, pcurve subtree).

use crate::entities::geometry::cartesian_point_2d::CartesianPoint2dHandler;
use crate::entities::geometry::vector_2d::Vector2dHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Curve2d, Line2};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct Line2dHandler;

impl SimpleEntityHandler for Line2dHandler {
    const NAME: &'static str = "LINE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4aCurve;
    type WriteInput = Line2;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "LINE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pnt_ref = read_entity_ref(attrs, 1, entity_id, "pnt")?;
        let vec_ref = read_entity_ref(attrs, 2, entity_id, "dir")?;
        let point = *ctx
            .point_2d_map
            .get(&pnt_ref)
            .ok_or(ConvertError::MissingReference {
                from: entity_id,
                to: pnt_ref,
                field_name: "pnt",
            })?;
        let (direction, magnitude) =
            *ctx.vector_2d_map
                .get(&vec_ref)
                .ok_or(ConvertError::MissingReference {
                    from: entity_id,
                    to: vec_ref,
                    field_name: "dir",
                })?;
        let id = ctx.geometry.curves_2d.push(Curve2d::Line(Line2 {
            point,
            direction,
            magnitude,
        }));
        ctx.curve_2d_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, line: Line2) -> Result<u64, WriteError> {
        let pnt = CartesianPoint2dHandler::write(buf, line.point)?;
        let vec = Vector2dHandler::write(buf, (line.direction, line.magnitude))?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "LINE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pnt),
                    Attribute::EntityRef(vec),
                ],
            },
        });
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static LINE_2D_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: Line2dHandler::NAME,
    pass_level: Line2dHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: Line2dHandler::read,
    },
};
