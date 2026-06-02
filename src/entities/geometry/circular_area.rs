//! `CIRCULAR_AREA` handler — phase ca.
//!
//! `primitive_2d` SUBTYPE — orphan in step-io (no inbound refs).
//! 3 attr (name + centre + radius). centre ref resolves through
//! `point_map`; unresolved drops the carrier.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::CircularArea;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CircularAreaHandler;

#[step_entity(name = "CIRCULAR_AREA")]
impl SimpleEntityHandler for CircularAreaHandler {
    type WriteInput = CircularArea;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "CIRCULAR_AREA")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let centre_ref = read_entity_ref(attrs, 1, entity_id, "centre")?;
        let radius = read_real(attrs, 2, entity_id, "radius")?;
        let Some(&centre) = ctx.point_map.get(&centre_ref) else {
            return Ok(());
        };
        ctx.geometry.circular_areas.push(CircularArea {
            name,
            centre,
            radius,
        });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ca: CircularArea) -> Result<u64, WriteError> {
        let centre_step = buf.emit_point(ca.centre)?;
        Ok(buf.push_simple(
            "CIRCULAR_AREA",
            vec![
                Attribute::String(ca.name),
                Attribute::EntityRef(centre_step),
                Attribute::Real(ca.radius),
            ],
        ))
    }
}
