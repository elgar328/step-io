//! `PLACED_DATUM_TARGET_FEATURE` handler — shape-rep domain (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::PlacedDatumTargetFeature;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PlacedDatumTargetFeatureHandler;

#[step_entity(name = "PLACED_DATUM_TARGET_FEATURE")]
impl SimpleEntityHandler for PlacedDatumTargetFeatureHandler {
    type WriteInput = PlacedDatumTargetFeature;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_placed_datum_target_feature(entity_id, attrs)?;
        lower::lower_placed_datum_target_feature(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, p: PlacedDatumTargetFeature) -> Result<u64, WriteError> {
        let pds_step_id = buf
            .product_def_shape_ids
            .get(&p.target)
            .copied()
            .unwrap_or(0);
        let early = lift::lift_placed_datum_target_feature(p, pds_step_id);
        Ok(serialize::serialize_placed_datum_target_feature(
            buf, &early,
        ))
    }
}
