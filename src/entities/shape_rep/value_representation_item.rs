//! `VALUE_REPRESENTATION_ITEM` handler — phase repr-item-arena-1.
//!
//! `value_component` is a `measure_value` SELECT; STEP parser exposes
//! the typed literal as [`Attribute::Typed { type_name, value }`]. The
//! `type_name` is preserved verbatim so round-trips reproduce
//! `POSITIVE_LENGTH_MEASURE(0.05)` etc.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::representation_item::{MeasureValue, RepresentationItem, ValueRepresentationItem};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "VALUE_REPRESENTATION_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        // value_component is a `measure_value` SELECT — parser exposes it as
        // Attribute::Typed { type_name, value: Real/Integer/String }.
        let value_component = match &attrs[1] {
            Attribute::Typed { type_name, value } => match value.as_ref() {
                Attribute::Real(v) => MeasureValue::Real {
                    type_name: type_name.clone(),
                    value: *v,
                },
                Attribute::Integer(v) => MeasureValue::Integer {
                    type_name: type_name.clone(),
                    value: *v,
                },
                Attribute::String(s) => MeasureValue::Text {
                    type_name: type_name.clone(),
                    value: s.clone(),
                },
                _ => return Ok(()), // unsupported nested payload — drop
            },
            _ => return Ok(()), // expected typed literal — drop
        };
        let id = ctx
            .representation_items
            .push(RepresentationItem::ValueRepresentationItem(
                ValueRepresentationItem {
                    name,
                    value_component,
                },
            ));
        ctx.repr_item_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, vri: ValueRepresentationItem) -> Result<u64, WriteError> {
        let typed = match vri.value_component {
            MeasureValue::Real { type_name, value } => Attribute::Typed {
                type_name,
                value: Box::new(Attribute::Real(value)),
            },
            MeasureValue::Integer { type_name, value } => Attribute::Typed {
                type_name,
                value: Box::new(Attribute::Integer(value)),
            },
            MeasureValue::Text { type_name, value } => Attribute::Typed {
                type_name,
                value: Box::new(Attribute::String(value)),
            },
        };
        Ok(buf.push_simple(
            "VALUE_REPRESENTATION_ITEM",
            vec![Attribute::String(vri.name), typed],
        ))
    }
}
