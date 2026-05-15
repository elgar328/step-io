//! `CARTESIAN_POINT` handler — Pass 1 (2D variant).
//!
//! Sister handler of [`crate::entities::geometry::cartesian_point::CartesianPointHandler`].
//! Same STEP entity name; both run in Pass 1 alongside each other and
//! select which arena receives the entity by coordinate count. A
//! 2-coordinate point goes to `geometry.points_2d`; a 3-coordinate point
//! goes to `geometry.points`. Wrong-dimension or malformed inputs land
//! in no arena (silent skip).

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::Point2dId;
use crate::ir::attr::{check_count, read_real_list, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Point2;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct CartesianPoint2dHandler;

impl SimpleEntityHandler for CartesianPoint2dHandler {
    const NAME: &'static str = "CARTESIAN_POINT";
    const PASS_LEVEL: PassLevel = PassLevel::Pass1;
    type WriteInput = Point2dId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "CARTESIAN_POINT")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let coords = read_real_list(attrs, 1, entity_id, "coordinates")?;
        if coords.len() != 2 {
            // Wrong dimension for the 2D arena. The 3D sister handler
            // claims 3-coordinate points; anything else is silently
            // dropped here.
            return Ok(());
        }
        let id = ctx.geometry.points_2d.push(Point2 {
            x: coords[0],
            y: coords[1],
        });
        ctx.point_2d_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: Point2dId) -> Result<u64, WriteError> {
        if let Some(&n) = buf.point_2d_ids.get(&id) {
            return Ok(n);
        }
        let p = buf
            .model
            .geometry
            .points_2d
            .iter()
            .nth(id.0 as usize)
            .copied()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("Point2dId({})", id.0),
            })?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "CARTESIAN_POINT".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(vec![Attribute::Real(p.x), Attribute::Real(p.y)]),
                ],
            },
        });
        buf.point_2d_ids.insert(id, n);
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static CARTESIAN_POINT_2D_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: CartesianPoint2dHandler::NAME,
    pass_level: CartesianPoint2dHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: CartesianPoint2dHandler::read,
    },
};
