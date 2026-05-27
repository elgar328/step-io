//! `AXIS2_PLACEMENT_3D` handler — Pass 3.
//!
//! Mirrors the legacy `ReaderContext::convert_axis2_placement_3d` and
//! `WriteBuffer::emit_axis2_placement_3d`.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::entities::geometry::direction::DirectionHandler;
use crate::ir::Placement3dId;
use crate::ir::attr::{
    check_count, read_entity_ref, read_optional_entity_ref, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Axis2Placement3d;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct Axis2Placement3dHandler;

#[step_entity(name = "AXIS2_PLACEMENT_3D", pass = Pass3)]
impl SimpleEntityHandler for Axis2Placement3dHandler {
    type WriteInput = Placement3dId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "AXIS2_PLACEMENT_3D")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let loc_ref = read_entity_ref(attrs, 1, entity_id, "location")?;
        let axis_ref = read_optional_entity_ref(attrs, 2, entity_id, "axis")?;
        let ref_dir_ref = read_optional_entity_ref(attrs, 3, entity_id, "ref_direction")?;

        // If the location is a known 2D point, this is the 2D sister
        // placement variant — silently skip.
        if ctx.point_2d_map.contains_key(&loc_ref) {
            return Ok(());
        }
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
