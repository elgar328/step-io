//! `DIMENSIONAL_CHARACTERISTIC_REPRESENTATION` handler — property
//! (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::pmi::DimensionalCharacteristic;
use crate::ir::property::DimensionalCharacteristicRepresentation;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DimensionalCharacteristicRepresentationHandler;

#[step_entity(name = "DIMENSIONAL_CHARACTERISTIC_REPRESENTATION")]
impl SimpleEntityHandler for DimensionalCharacteristicRepresentationHandler {
    type WriteInput = DimensionalCharacteristicRepresentation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_dimensional_characteristic_representation(entity_id, attrs)?;
        lower::lower_dimensional_characteristic_representation(ctx, entity_id, &early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        dcr: DimensionalCharacteristicRepresentation,
    ) -> Result<u64, WriteError> {
        let dim_step = match dcr.dimension {
            DimensionalCharacteristic::Size(id) => buf.step_id(id),
            DimensionalCharacteristic::Location(id) => buf.step_id(id),
            DimensionalCharacteristic::SizeWithDatumFeature(id) => buf.step_id(id),
        };
        let repr_step = buf.step_id(dcr.representation);
        let early = lift::lift_dimensional_characteristic_representation(dim_step, repr_step);
        Ok(serialize::serialize_dimensional_characteristic_representation(buf, &early))
    }
}
