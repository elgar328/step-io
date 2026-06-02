//! `DIMENSIONAL_CHARACTERISTIC_REPRESENTATION` handler — phase sdr-dcr.
//!
//! Pairs a `dimensional_characteristic` SELECT (resolved through the
//! `dimensional_size_id_map` / `dimensional_location_id_map`) with a
//! `RepresentationId` (resolved through `repr_id_map`). Either ref
//! unresolved drops the occurrence, symmetric on re-read.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::pmi::DimensionalCharacteristic;
use crate::ir::property::{DimensionalCharacteristicRepresentation, PropertyPool};
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
        check_count(
            attrs,
            2,
            entity_id,
            "DIMENSIONAL_CHARACTERISTIC_REPRESENTATION",
        )?;
        let dim_ref = read_entity_ref(attrs, 0, entity_id, "dimension")?;
        let repr_ref = read_entity_ref(attrs, 1, entity_id, "representation")?;
        let dimension = if let Some(&id) = ctx.dimensional_size_id_map.get(&dim_ref) {
            DimensionalCharacteristic::Size(id)
        } else if let Some(&id) = ctx.dimensional_location_id_map.get(&dim_ref) {
            DimensionalCharacteristic::Location(id)
        } else {
            return Ok(());
        };
        let Some(&representation) = ctx.repr_id_map.get(&repr_ref) else {
            return Ok(());
        };
        let property = ctx.properties.get_or_insert_with(PropertyPool::default);
        let id = property.dimensional_characteristic_representations.push(
            DimensionalCharacteristicRepresentation {
                dimension,
                representation,
            },
        );
        ctx.dimensional_characteristic_representation_id_map
            .insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        dcr: DimensionalCharacteristicRepresentation,
    ) -> Result<u64, WriteError> {
        let dim_step = match dcr.dimension {
            DimensionalCharacteristic::Size(id) => buf.dimensional_size_step_ids[id.0 as usize],
            DimensionalCharacteristic::Location(id) => {
                buf.dimensional_location_step_ids[id.0 as usize]
            }
        };
        let repr_step = buf.representation_step_ids[dcr.representation.0 as usize];
        Ok(buf.push_simple(
            "DIMENSIONAL_CHARACTERISTIC_REPRESENTATION",
            vec![
                Attribute::EntityRef(dim_step),
                Attribute::EntityRef(repr_step),
            ],
        ))
    }
}
