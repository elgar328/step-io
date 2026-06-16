//! `ITEM_IDENTIFIED_REPRESENTATION_USAGE` handler — phase iiru.
//!
//! Concrete `item_identified_representation_usage` instances (54 corpus,
//! separate from `GISU` / `DMIA` subtypes which have their own arenas).
//! 5 attrs: `name` + `description` (opt) + `definition` (SELECT) +
//! `used_representation` (`ref_repr`) + `identified_item` (SELECT typed
//! wrapper). `definition` SELECT is partial — covers the corpus-common
//! PMI / shape-aspect members; others drop the carrier on read.
//! `identified_item` is either a single ref or a typed
//! `SET_REPRESENTATION_ITEM` / `LIST_REPRESENTATION_ITEM` wrapper (CRI
//! precedent).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::ItemIdentifiedRepresentationUsage;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ItemIdentifiedRepresentationUsageHandler;

#[step_entity(name = "ITEM_IDENTIFIED_REPRESENTATION_USAGE")]
impl SimpleEntityHandler for ItemIdentifiedRepresentationUsageHandler {
    type WriteInput = ItemIdentifiedRepresentationUsage;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        if let Some(early) = bind::bind_item_identified_representation_usage(entity_id, attrs)? {
            lower::lower_item_identified_representation_usage(ctx, entity_id, early);
        }
        // else: unrecognized `identified_item` member — silent drop (legacy behavior).
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        iiru: ItemIdentifiedRepresentationUsage,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_item_identified_representation_usage(buf, iiru)?;
        Ok(serialize::serialize_item_identified_representation_usage(
            buf, &early,
        ))
    }
}
