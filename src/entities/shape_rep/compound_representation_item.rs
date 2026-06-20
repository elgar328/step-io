//! `COMPOUND_REPRESENTATION_ITEM` handler — phase cri.
//!
//! `representation_item` SUBTYPE with one typed `item_element` SELECT
//! (`SET_REPRESENTATION_ITEM(...)` / `LIST_REPRESENTATION_ITEM(...)`).
//! Each child ref is either an inline `DescriptiveRepresentationItem`
//! (step-io has no DRI arena — kept inline) or any
//! [`crate::ir::representation_item::RepresentationItemRef`] member
//! resolvable via `resolve_representation_item_ref`. Carrier drops when
//! every child ref is unresolved (symmetric on re-read).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::CompoundRepresentationItem;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CompoundRepresentationItemHandler;

#[step_entity(name = "COMPOUND_REPRESENTATION_ITEM")]
impl SimpleEntityHandler for CompoundRepresentationItemHandler {
    type WriteInput = CompoundRepresentationItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        if let Some(early) = bind::bind_compound_representation_item(entity_id, attrs)? {
            lower::lower_compound_representation_item(ctx, entity_id, early);
        }
        // else: unrecognized `item_element` wrapper — silent drop (legacy behavior).
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, cri: CompoundRepresentationItem) -> Result<u64, WriteError> {
        let early = lift::lift_compound_representation_item(buf, cri)?;
        Ok(serialize::serialize_compound_representation_item(
            buf, &early,
        ))
    }
}
