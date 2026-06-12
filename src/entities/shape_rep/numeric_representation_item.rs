//! `INTEGER_REPRESENTATION_ITEM` / `REAL_REPRESENTATION_ITEM` handlers
//! (phase numeric-representation-item).
//!
//! Both are `representation_item` value-items — STEP P21 `(name, the_value)`.
//! The schema types the integer variant's `the_value` as `INTEGER`, but
//! fixtures encode it as a real literal (`8.`); the reader accepts both
//! forms via `read_real` and stores a standard `i64`. Read into the shared
//! `numeric_representation_items` arena and emitted orphan — no modelled
//! consumer references them yet.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{
    IntegerRepresentationItem, NumericRepresentationItem, RealRepresentationItem,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct IntegerRepresentationItemHandler;

#[step_entity(name = "INTEGER_REPRESENTATION_ITEM")]
impl SimpleEntityHandler for IntegerRepresentationItemHandler {
    type WriteInput = IntegerRepresentationItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "INTEGER_REPRESENTATION_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        // Schema types `the_value` as INTEGER, but fixtures write a real
        // literal (`8.`); `read_real` accepts both — store the standard i64.
        #[allow(clippy::cast_possible_truncation)]
        let the_value = read_real(attrs, 1, entity_id, "the_value")? as i64;
        ctx.numeric_representation_items
            .push(NumericRepresentationItem::Integer(
                IntegerRepresentationItem { name, the_value },
            ));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, item: IntegerRepresentationItem) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "INTEGER_REPRESENTATION_ITEM",
            vec![
                Attribute::String(item.name),
                Attribute::Integer(item.the_value),
            ],
        ))
    }
}

pub(crate) struct RealRepresentationItemHandler;

#[step_entity(name = "REAL_REPRESENTATION_ITEM")]
impl SimpleEntityHandler for RealRepresentationItemHandler {
    type WriteInput = RealRepresentationItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_real_representation_item(entity_id, attrs)?;
        crate::early::lower::lower_real_representation_item(ctx, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, item: RealRepresentationItem) -> Result<u64, WriteError> {
        let early = crate::early::lift::lift_real_representation_item(item.name, item.the_value);
        Ok(crate::early::serialize::serialize_real_representation_item(
            buf, &early,
        ))
    }
}
