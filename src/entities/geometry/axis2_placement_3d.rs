//! `AXIS2_PLACEMENT_3D` handler — Pass 3.
//!
//! Mirrors the legacy `ReaderContext::convert_axis2_placement_3d` and
//! `WriteBuffer::emit_axis2_placement_3d`.

use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::entities::geometry::direction::DirectionHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::Placement3dId;
use crate::ir::attr::{check_count, read_entity_ref, read_optional_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Axis2Placement3d;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct Axis2Placement3dHandler;

impl SimpleEntityHandler for Axis2Placement3dHandler {
    const NAME: &'static str = "AXIS2_PLACEMENT_3D";
    const PASS_LEVEL: PassLevel = PassLevel::Pass3;
    type WriteInput = Placement3dId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "AXIS2_PLACEMENT_3D")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let loc_ref = read_entity_ref(attrs, 1, entity_id, "location")?;
        let axis_ref = read_optional_entity_ref(attrs, 2, entity_id, "axis")?;
        let ref_dir_ref = read_optional_entity_ref(attrs, 3, entity_id, "ref_direction")?;

        let location = ctx.resolve_point(entity_id, loc_ref, "location")?;
        let axis = axis_ref
            .map(|r| ctx.resolve_direction(entity_id, r, "axis"))
            .transpose()?;
        let ref_direction = ref_dir_ref
            .map(|r| ctx.resolve_direction(entity_id, r, "ref_direction"))
            .transpose()?;

        let placement = Axis2Placement3d {
            location,
            axis,
            ref_direction,
        };
        let id = ctx.geometry.placements.push(placement);
        ctx.placement_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: Placement3dId) -> Result<u64, WriteError> {
        if let Some(&n) = buf.placement_ids.get(&id) {
            return Ok(n);
        }
        let placement = buf.model.geometry.placements[id];
        let loc = CartesianPointHandler::write(buf, placement.location)?;
        let axis_attr = match placement.axis {
            Some(dir) => Attribute::EntityRef(DirectionHandler::write(buf, dir)?),
            None => Attribute::Unset,
        };
        let ref_attr = match placement.ref_direction {
            Some(dir) => Attribute::EntityRef(DirectionHandler::write(buf, dir)?),
            None => Attribute::Unset,
        };
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "AXIS2_PLACEMENT_3D".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(loc),
                    axis_attr,
                    ref_attr,
                ],
            },
        });
        buf.placement_ids.insert(id, n);
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static AXIS2_PLACEMENT_3D_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: Axis2Placement3dHandler::NAME,
    pass_level: Axis2Placement3dHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: Axis2Placement3dHandler::read,
    },
};
