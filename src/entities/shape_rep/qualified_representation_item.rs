//! `QUALIFIED_REPRESENTATION_ITEM` handler (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::representation_item::QualifiedRepresentationItem;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct QualifiedRepresentationItemHandler;

#[step_entity(name = "QUALIFIED_REPRESENTATION_ITEM")]
impl SimpleEntityHandler for QualifiedRepresentationItemHandler {
    type WriteInput = QualifiedRepresentationItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_qualified_representation_item(entity_id, attrs)?;
        lower::lower_qualified_representation_item(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, qri: QualifiedRepresentationItem) -> Result<u64, WriteError> {
        let qualifiers: Vec<u64> = qri
            .qualifiers
            .into_iter()
            .map(|q| q.emit_select(buf))
            .collect();
        let early = lift::lift_qualified_representation_item(qri.name, qualifiers);
        Ok(serialize::serialize_qualified_representation_item(
            buf, &early,
        ))
    }
}
