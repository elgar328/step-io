//! `DATUM_SYSTEM` handler — shape-rep domain (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::DatumSystem;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DatumSystemHandler;

#[step_entity(name = "DATUM_SYSTEM")]
impl SimpleEntityHandler for DatumSystemHandler {
    type WriteInput = DatumSystem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_datum_system(entity_id, attrs)?;
        lower::lower_datum_system(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ds: DatumSystem) -> Result<u64, WriteError> {
        let pds_step_id = buf
            .product_def_shape_ids
            .get(&ds.target)
            .copied()
            .unwrap_or(0);
        let constituents: Vec<u64> = ds.constituents.iter().map(|g| buf.step_id(g)).collect();
        let early = lift::lift_datum_system(
            ds.name,
            ds.description,
            pds_step_id,
            ds.product_definitional,
            constituents,
        );
        Ok(serialize::serialize_datum_system(buf, &early))
    }
}
