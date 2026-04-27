//! `AXIS1_PLACEMENT` handler — Pass 3.
//!
//! Mirrors the legacy `ReaderContext::convert_axis1_placement` and
//! `WriteBuffer::emit_axis1_placement`.

use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::entities::geometry::direction::DirectionHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::Placement1dId;
use crate::ir::attr::{check_count, read_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Axis1Placement;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct Axis1PlacementHandler;

impl SimpleEntityHandler for Axis1PlacementHandler {
    const NAME: &'static str = "AXIS1_PLACEMENT";
    const PASS_LEVEL: PassLevel = PassLevel::Pass3;
    type WriteInput = Placement1dId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "AXIS1_PLACEMENT")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let loc_ref = read_entity_ref(attrs, 1, entity_id, "location")?;
        let axis_ref = read_entity_ref(attrs, 2, entity_id, "axis")?;

        let location = ctx.resolve_point(entity_id, loc_ref, "location")?;
        let axis = ctx.resolve_direction(entity_id, axis_ref, "axis")?;

        let placement = Axis1Placement { location, axis };
        let id = ctx.geometry.placements_1d.push(placement);
        ctx.axis1_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: Placement1dId) -> Result<u64, WriteError> {
        if let Some(&n) = buf.placement_1d_ids.get(&id) {
            return Ok(n);
        }
        let placement = buf.model.geometry.placements_1d[id];
        let loc = CartesianPointHandler::write(buf, placement.location)?;
        let dir = DirectionHandler::write(buf, placement.axis)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "AXIS1_PLACEMENT".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(loc),
                    Attribute::EntityRef(dir),
                ],
            },
        });
        buf.placement_1d_ids.insert(id, n);
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static AXIS1_PLACEMENT_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: Axis1PlacementHandler::NAME,
    pass_level: Axis1PlacementHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: Axis1PlacementHandler::read,
    },
};
