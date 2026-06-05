//! `GEOMETRIC_ITEM_SPECIFIC_USAGE` handler — phase gisu.
//!
//! Sibling of `DRAUGHTING_MODEL_ITEM_ASSOCIATION` — both subtype
//! `item_identified_representation_usage`, but GISU narrows
//! `definition` to a shape-aspect-family ref and `identified_item` to a
//! `representation_item` ref. Round-trip drops the carrier when any of
//! the three refs fails to resolve (symmetric on re-read).

use crate::entities::SimpleEntityHandler;
use crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::GeometricItemSpecificUsage;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::{DeferredGisu, ReaderContext};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct GeometricItemSpecificUsageHandler;

#[step_entity(name = "GEOMETRIC_ITEM_SPECIFIC_USAGE")]
impl SimpleEntityHandler for GeometricItemSpecificUsageHandler {
    type WriteInput = GeometricItemSpecificUsage;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 5, entity_id, "GEOMETRIC_ITEM_SPECIFIC_USAGE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = match &attrs[1] {
            Attribute::Unset => None,
            Attribute::String(s) => Some(s.clone()),
            _ => return Ok(()),
        };
        let def_ref = read_entity_ref(attrs, 2, entity_id, "definition")?;
        let Some(definition) = resolve_shape_aspect_ref(ctx, def_ref) else {
            return Ok(());
        };
        let item_ref = read_entity_ref(attrs, 4, entity_id, "identified_item")?;
        let Some(identified_item) = resolve_representation_item_ref(ctx, item_ref) else {
            return Ok(());
        };
        // `used_representation` is a required `representation`, but CATIA emits
        // `$` for "Solid" GISUs. Defer the `$` case: its standard value (the
        // WHERE-rule container of `identified_item`) is not referenced by this
        // GISU, so dispatch order gives no guarantee the container was read
        // first — `resolve_deferred_gisu_used_representation` derives it.
        if matches!(attrs[3], Attribute::Unset) {
            ctx.deferred_gisu_used_repr.push(DeferredGisu {
                entity_id,
                name,
                description,
                definition,
                identified_item,
            });
            return Ok(());
        }
        let used_ref = read_entity_ref(attrs, 3, entity_id, "used_representation")?;
        let Some(&used_representation) = ctx.repr_id_map.get(&used_ref) else {
            return Ok(());
        };
        let id = ctx
            .geometric_item_specific_usages
            .push(GeometricItemSpecificUsage {
                name,
                description,
                definition,
                used_representation,
                identified_item,
            });
        ctx.gisu_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gisu: GeometricItemSpecificUsage) -> Result<u64, WriteError> {
        let def_step = buf.emit_shape_aspect_ref(gisu.definition);
        let used_step = buf.representation_step_ids[gisu.used_representation.0 as usize];
        let item_step = buf.emit_representation_item_ref(gisu.identified_item)?;
        let description_attr = match gisu.description {
            Some(s) => Attribute::String(s),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "GEOMETRIC_ITEM_SPECIFIC_USAGE",
            vec![
                Attribute::String(gisu.name),
                description_attr,
                Attribute::EntityRef(def_step),
                Attribute::EntityRef(used_step),
                Attribute::EntityRef(item_step),
            ],
        ))
    }
}
