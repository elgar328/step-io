//! `AXIS2_PLACEMENT_2D` handler — Pass 4a-2.
//!
//! Mirrors the legacy `convert_axis2_placement_2d` and
//! `emit_axis2_placement_2d`. Distinct STEP entity name (no 3D
//! counterpart sharing it), so dispatch is straightforward.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::cartesian_point_2d::CartesianPoint2dHandler;
use crate::entities::geometry::direction_2d::Direction2dHandler;
use crate::ir::Placement2dId;
use crate::ir::attr::{
    check_count, read_entity_ref, read_optional_entity_ref, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Axis2Placement2d;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct Axis2Placement2dHandler;

#[step_entity(name = "AXIS2_PLACEMENT_2D", pass = Pass4aVector)]
impl SimpleEntityHandler for Axis2Placement2dHandler {
    type WriteInput = Placement2dId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "AXIS2_PLACEMENT_2D")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let loc_ref = read_entity_ref(attrs, 1, entity_id, "location")?;
        let ref_dir_ref = read_optional_entity_ref(attrs, 2, entity_id, "ref_direction")?;

        // First cross-ref discriminates 2D vs 3D: if the location point
        // is absent from the 2D arena, this is the 3D placement variant.
        let Some(&location) = ctx.point_2d_map.get(&loc_ref) else {
            return Ok(());
        };
        let ref_direction = match ref_dir_ref {
            Some(r) => Some(*ctx.direction_2d_map.get(&r).ok_or(
                ConvertError::MissingReference {
                    from: entity_id,
                    to: r,
                    field_name: "ref_direction",
                },
            )?),
            None => None,
        };
        let placement = Axis2Placement2d {
            location,
            ref_direction,
        };
        let id = ctx.geometry.placements_2d.push(placement);
        ctx.placement_2d_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: Placement2dId) -> Result<u64, WriteError> {
        if let Some(&n) = buf.placement_2d_ids.get(&id) {
            return Ok(n);
        }
        let placement = buf.model.geometry.placements_2d[id];
        let loc = CartesianPoint2dHandler::write(buf, placement.location)?;
        let ref_attr = match placement.ref_direction {
            Some(dir) => Attribute::EntityRef(Direction2dHandler::write(buf, dir)?),
            None => Attribute::Unset,
        };
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "AXIS2_PLACEMENT_2D".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(loc),
                    ref_attr,
                ],
            },
        });
        buf.placement_2d_ids.insert(id, n);
        Ok(n)
    }
}
