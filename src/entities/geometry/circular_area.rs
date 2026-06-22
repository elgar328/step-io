//! `CIRCULAR_AREA` handler — phase ca.
//!
//! `primitive_2d` SUBTYPE — orphan in step-io (no inbound refs).
//! 3 attr (name + centre + radius). `centre` resolves through `point_map`
//! (a local `cartesian_point`) or, for the P21 edition 3 conformance
//! fixture, through `external_ref_id_map` (a `REFERENCE`-section external
//! reference); unresolved drops the carrier.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::{CircularArea, CircularAreaCentre};
use crate::parser::entity::Attribute;
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
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_circular_area(entity_id, attrs)?;
        lower::lower_circular_area(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ca: CircularArea) -> Result<u64, WriteError> {
        let centre_step = match ca.centre {
            CircularAreaCentre::Point(point) => buf.emit_point(point)?,
            CircularAreaCentre::External(ext) => buf.step_id(ext),
        };
        let early = lift::lift_circular_area(ca.name, centre_step, ca.radius);
        Ok(serialize::serialize_circular_area(buf, &early))
    }
}
