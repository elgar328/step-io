//! `TOLERANCE_ZONE` handler — shape-rep domain (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::pmi::GeometricToleranceRef;
use crate::ir::shape_rep::ToleranceZone;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ToleranceZoneHandler;

#[step_entity(name = "TOLERANCE_ZONE")]
impl SimpleEntityHandler for ToleranceZoneHandler {
    type WriteInput = ToleranceZone;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_tolerance_zone(entity_id, attrs)?;
        lower::lower_tolerance_zone(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, tz: ToleranceZone) -> Result<u64, WriteError> {
        let pds_step_id = buf
            .product_def_shape_ids
            .get(&tz.target)
            .copied()
            .unwrap_or(0);
        let defining_tolerance: Vec<u64> = tz
            .defining_tolerance
            .iter()
            .map(|gtr| match gtr {
                GeometricToleranceRef::Plain(id) => buf.step_id(id),
                GeometricToleranceRef::WithDatumReference(id) => buf.step_id(id),
            })
            .collect();
        let form = buf.step_id(tz.form);
        let early = lift::lift_tolerance_zone(
            tz.name,
            tz.description,
            pds_step_id,
            tz.product_definitional,
            defining_tolerance,
            form,
        );
        Ok(serialize::serialize_tolerance_zone(buf, &early))
    }
}
