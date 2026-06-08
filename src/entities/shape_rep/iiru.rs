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

use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{
    CompoundItemKind, IiruDefinition, IiruIdentifiedItem, ItemIdentifiedRepresentationUsage,
};
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
        check_count(attrs, 5, entity_id, "ITEM_IDENTIFIED_REPRESENTATION_USAGE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = match &attrs[1] {
            Attribute::String(s) => Some(s.clone()),
            _ => None,
        };
        let def_ref = read_entity_ref(attrs, 2, entity_id, "definition")?;
        let Some(definition) = resolve_iiru_definition(ctx, def_ref) else {
            return Ok(());
        };
        let used_ref = read_entity_ref(attrs, 3, entity_id, "used_representation")?;
        let Some(&used_representation) = ctx.repr_id_map.get(&used_ref) else {
            return Ok(());
        };
        let Some(identified_item) = parse_iiru_identified_item(ctx, &attrs[4]) else {
            return Ok(());
        };
        ctx.item_identified_representation_usages
            .push(ItemIdentifiedRepresentationUsage {
                name,
                description,
                definition,
                used_representation,
                identified_item,
            });
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        iiru: ItemIdentifiedRepresentationUsage,
    ) -> Result<u64, WriteError> {
        let def_step = match iiru.definition {
            IiruDefinition::ShapeAspect(id) => buf.step_id(id),
            IiruDefinition::Datum(id) => buf.step_id(id),
            IiruDefinition::DatumFeature(id) => buf.step_id(id),
            IiruDefinition::DimensionalSize(id) => buf.step_id(id),
            IiruDefinition::GeometricTolerance(id) => buf.step_id(id),
        };
        let used_step = buf.step_id(iiru.used_representation);
        let identified_attr = emit_iiru_identified_item(buf, iiru.identified_item)?;
        let desc_attr = match iiru.description {
            Some(d) => Attribute::String(d),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "ITEM_IDENTIFIED_REPRESENTATION_USAGE",
            vec![
                Attribute::String(iiru.name),
                desc_attr,
                Attribute::EntityRef(def_step),
                Attribute::EntityRef(used_step),
                identified_attr,
            ],
        ))
    }
}

fn resolve_iiru_definition(ctx: &ReaderContext, ref_id: u64) -> Option<IiruDefinition> {
    if let Some(&id) = ctx.shape_aspect_id_map.get(&ref_id) {
        return Some(IiruDefinition::ShapeAspect(id));
    }
    if let Some(&id) = ctx.datum_feature_id_map.get(&ref_id) {
        return Some(IiruDefinition::DatumFeature(id));
    }
    if let Some(&id) = ctx.datum_id_map.get(&ref_id) {
        return Some(IiruDefinition::Datum(id));
    }
    if let Some(&id) = ctx.dimensional_size_id_map.get(&ref_id) {
        return Some(IiruDefinition::DimensionalSize(id));
    }
    if let Some(&id) = ctx.geometric_tolerance_id_map.get(&ref_id) {
        return Some(IiruDefinition::GeometricTolerance(id));
    }
    None
}

fn parse_iiru_identified_item(ctx: &ReaderContext, attr: &Attribute) -> Option<IiruIdentifiedItem> {
    match attr {
        Attribute::EntityRef(r) => {
            resolve_representation_item_ref(ctx, *r).map(IiruIdentifiedItem::Item)
        }
        Attribute::Typed { type_name, value } => {
            let kind = match type_name.as_str() {
                "SET_REPRESENTATION_ITEM" => CompoundItemKind::Set,
                "LIST_REPRESENTATION_ITEM" => CompoundItemKind::List,
                _ => return None,
            };
            let Attribute::List(child_attrs) = value.as_ref() else {
                return None;
            };
            let mut items = Vec::with_capacity(child_attrs.len());
            for c in child_attrs {
                if let Attribute::EntityRef(r) = c {
                    if let Some(item) = resolve_representation_item_ref(ctx, *r) {
                        items.push(item);
                    }
                }
            }
            if items.is_empty() {
                return None;
            }
            Some(IiruIdentifiedItem::Compound { kind, items })
        }
        _ => None,
    }
}

fn emit_iiru_identified_item(
    buf: &mut WriteBuffer,
    item: IiruIdentifiedItem,
) -> Result<Attribute, WriteError> {
    match item {
        IiruIdentifiedItem::Item(r) => {
            let step = buf.emit_representation_item_ref(r)?;
            Ok(Attribute::EntityRef(step))
        }
        IiruIdentifiedItem::Compound { kind, items } => {
            let mut refs = Vec::with_capacity(items.len());
            for r in items {
                let step = buf.emit_representation_item_ref(r)?;
                refs.push(Attribute::EntityRef(step));
            }
            let type_name = match kind {
                CompoundItemKind::Set => "SET_REPRESENTATION_ITEM",
                CompoundItemKind::List => "LIST_REPRESENTATION_ITEM",
            };
            Ok(Attribute::Typed {
                type_name: type_name.into(),
                value: Box::new(Attribute::List(refs)),
            })
        }
    }
}
