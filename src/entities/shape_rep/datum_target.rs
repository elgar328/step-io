//! `DATUM_TARGET` handler — shape-rep domain (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::DatumTarget;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DatumTargetHandler;

#[step_entity(name = "DATUM_TARGET")]
impl SimpleEntityHandler for DatumTargetHandler {
    type WriteInput = DatumTarget;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_datum_target(entity_id, attrs)?;
        lower::lower_datum_target(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dt: DatumTarget) -> Result<u64, WriteError> {
        // `target` -> PRODUCT_DEFINITION_SHAPE step id; a miss is the
        // kernel-built IR defensive case, in practice unreachable.
        let pds_step_id = buf
            .product_def_shape_ids
            .get(&dt.target)
            .copied()
            .unwrap_or(0);
        let early = lift::lift_datum_target(dt, pds_step_id);
        Ok(serialize::serialize_datum_target(buf, &early))
    }
}
