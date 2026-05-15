//! `CARTESIAN_POINT` handler — Pass 1 leaf 3D point.
//!
//! Mirrors the legacy `ReaderContext::convert_cartesian_point` and
//! `WriteBuffer::emit_point` one-to-one. The writer entry point keeps
//! `emit_point` as a thin wrapper because callers in adjacent emit
//! functions reference it directly.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::PointId;
use crate::ir::attr::{check_count, read_real_list, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Point3;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct CartesianPointHandler;

impl SimpleEntityHandler for CartesianPointHandler {
    const NAME: &'static str = "CARTESIAN_POINT";
    const PASS_LEVEL: PassLevel = PassLevel::Pass1;
    type WriteInput = PointId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "CARTESIAN_POINT")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let coords = read_real_list(attrs, 1, entity_id, "coordinates")?;
        if coords.len() != 3 {
            // Wrong dimension for the 3D arena. The 2D sister handler
            // claims 2-coordinate points; anything else is silently
            // dropped here.
            return Ok(());
        }
        let point = Point3 {
            x: coords[0],
            y: coords[1],
            z: coords[2],
        };
        let id = ctx.geometry.points.push(point);
        ctx.point_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: PointId) -> Result<u64, WriteError> {
        if let Some(&n) = buf.point_ids.get(&id) {
            return Ok(n);
        }
        let p = buf
            .model
            .geometry
            .points
            .iter()
            .nth(id.0 as usize)
            .copied()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("PointId({})", id.0),
            })?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "CARTESIAN_POINT".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(vec![
                        Attribute::Real(p.x),
                        Attribute::Real(p.y),
                        Attribute::Real(p.z),
                    ]),
                ],
            },
        });
        buf.point_ids.insert(id, n);
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static CARTESIAN_POINT_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: CartesianPointHandler::NAME,
    pass_level: CartesianPointHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: CartesianPointHandler::read,
    },
};
