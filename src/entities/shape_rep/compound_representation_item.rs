//! `COMPOUND_REPRESENTATION_ITEM` handler — phase cri.
//!
//! `representation_item` SUBTYPE with one typed `item_element` SELECT
//! (`SET_REPRESENTATION_ITEM(...)` / `LIST_REPRESENTATION_ITEM(...)`).
//! Each child ref is either an inline `DescriptiveRepresentationItem`
//! (step-io has no DRI arena — kept inline) or any
//! [`crate::ir::representation_item::RepresentationItemRef`] member
//! resolvable via `resolve_representation_item_ref`. Carrier drops when
//! every child ref is unresolved (symmetric on re-read).

use crate::entities::SimpleEntityHandler;
use crate::entities::shape_rep::descriptive_representation_item::DescriptiveRepresentationItemHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{
    CompoundItem, CompoundItemElement, CompoundItemKind, CompoundRepresentationItem,
};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "COMPOUND_REPRESENTATION_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let Attribute::Typed { type_name, value } = &attrs[1] else {
            return Ok(());
        };
        let kind = match type_name.as_str() {
            "SET_REPRESENTATION_ITEM" => CompoundItemKind::Set,
            "LIST_REPRESENTATION_ITEM" => CompoundItemKind::List,
            _ => return Ok(()),
        };
        let Attribute::List(child_attrs) = value.as_ref() else {
            return Ok(());
        };
        let mut items = Vec::with_capacity(child_attrs.len());
        for child in child_attrs {
            let Attribute::EntityRef(r) = child else {
                continue;
            };
            if let Some(d) = ctx.descriptive_item_map.get(r).cloned() {
                items.push(CompoundItem::Descriptive(d));
                continue;
            }
            if let Some(item_ref) = resolve_representation_item_ref(ctx, *r) {
                items.push(CompoundItem::Item(item_ref));
            }
        }
        if items.is_empty() {
            return Ok(());
        }
        ctx.compound_representation_items
            .push(CompoundRepresentationItem {
                name,
                item_element: CompoundItemElement { kind, items },
            });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, cri: CompoundRepresentationItem) -> Result<u64, WriteError> {
        let mut refs = Vec::with_capacity(cri.item_element.items.len());
        for item in cri.item_element.items {
            let step = match item {
                CompoundItem::Descriptive(d) => {
                    DescriptiveRepresentationItemHandler::write(buf, d)?
                }
                CompoundItem::Item(r) => buf.emit_representation_item_ref(r)?,
            };
            refs.push(Attribute::EntityRef(step));
        }
        let type_name = match cri.item_element.kind {
            CompoundItemKind::Set => "SET_REPRESENTATION_ITEM",
            CompoundItemKind::List => "LIST_REPRESENTATION_ITEM",
        };
        Ok(buf.push_simple(
            "COMPOUND_REPRESENTATION_ITEM",
            vec![
                Attribute::String(cri.name),
                Attribute::Typed {
                    type_name: type_name.into(),
                    value: Box::new(Attribute::List(refs)),
                },
            ],
        ))
    }
}
