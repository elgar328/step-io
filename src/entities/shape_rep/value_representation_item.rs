//! `VALUE_REPRESENTATION_ITEM` handler (2-layer path).
//!
//! `value_component` is the `measure_value` SELECT, codegen via the hint-less
//! synth path → 43-typed `EarlyMeasureValue` (type determines real/text; NUMBER
//! members are real). `lower`/`lift` bridge to the generic L2 `MeasureValue`
//! (`type_name` preserved → round-trips reproduce `COUNT_MEASURE(21.)` etc.).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::representation_item::ValueRepresentationItem;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ValueRepresentationItemHandler;

#[step_entity(name = "VALUE_REPRESENTATION_ITEM")]
impl SimpleEntityHandler for ValueRepresentationItemHandler {
    type WriteInput = ValueRepresentationItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let Some(early) = bind::bind_value_representation_item(entity_id, attrs)? else {
            return Ok(()); // value_component did not bind (non-standard measure) — drop
        };
        lower::lower_value_representation_item(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, vri: ValueRepresentationItem) -> Result<u64, WriteError> {
        let early = lift::lift_value_representation_item(&vri);
        Ok(serialize::serialize_value_representation_item(buf, &early))
    }
}
