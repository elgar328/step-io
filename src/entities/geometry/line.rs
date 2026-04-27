//! `LINE` handler — Pass 4-1 leaf 3D line.
//!
//! Mirrors `ReaderContext::convert_line` and `WriteBuffer::emit_line`.

use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::entities::geometry::vector::VectorHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Curve, Line3};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct LineHandler;

impl SimpleEntityHandler for LineHandler {
    const NAME: &'static str = "LINE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4Leaf;
    type WriteInput = Line3;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "LINE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let pnt_ref = read_entity_ref(attrs, 1, entity_id, "pnt")?;
        let dir_ref = read_entity_ref(attrs, 2, entity_id, "dir")?;

        let point = ctx.resolve_point(entity_id, pnt_ref, "pnt")?;
        let (direction, magnitude) = ctx.resolve_vector(entity_id, dir_ref, "dir")?;

        let line = Line3 {
            point,
            direction,
            magnitude,
        };
        let id = ctx.geometry.curves.push(Curve::Line(line));
        ctx.curve_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, line: Line3) -> Result<u64, WriteError> {
        let pnt = CartesianPointHandler::write(buf, line.point)?;
        let vec = VectorHandler::write(buf, (line.direction, line.magnitude))?;
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
static LINE_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: LineHandler::NAME,
    pass_level: LineHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: LineHandler::read,
    },
};
