//! `INTEGER_REPRESENTATION_ITEM` / `REAL_REPRESENTATION_ITEM` handlers — 2-layer.
//!
//! Both are `representation_item` value-items — STEP P21 `(name, the_value)`.
//! Read into the shared `numeric_representation_items` arena and emitted orphan
//! (no modelled consumer references them yet).
//!
//! INTEGER `read` = generated `bind` + hand `lower_integer_representation_item`.
//! The schema (with the `int_literal` redeclare now reflected) types `the_value`
//! as INTEGER, so the strict `bind` uses `read_integer`. But fixtures commonly
//! encode it as a real literal (`8.`) — that non-standard form is normalized in
//! the handler before bind (read as real, store the standard i64), keeping L1
//! schema-strict.

use crate::early::model::EarlyIntegerRepresentationItem;
use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{IntegerRepresentationItem, RealRepresentationItem};
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
        let early = if matches!(attrs.get(1), Some(Attribute::Real(_))) {
            // Non-standard: fixtures encode the integer as a real literal (`8.`).
            // Normalize before the strict `read_integer` bind would reject it.
            let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
            #[allow(clippy::cast_possible_truncation)]
            let the_value = read_real(attrs, 1, entity_id, "the_value")? as i64;
            EarlyIntegerRepresentationItem { name, the_value }
        } else {
            bind::bind_integer_representation_item(entity_id, attrs)?
        };
        lower::lower_integer_representation_item(ctx, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, item: IntegerRepresentationItem) -> Result<u64, WriteError> {
        let early = lift::lift_integer_representation_item(item.name, item.the_value);
        Ok(serialize::serialize_integer_representation_item(
            buf, &early,
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
