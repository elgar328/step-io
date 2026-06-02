//! `CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM` (CDOSI) handler.
//!
//! Extends [`super::over_riding_styled_item::OverRidingStyledItemHandler`]
//! with the additional `style_context` SELECT list. Each context resolves
//! through [`resolve_representation_item_ref`] into a `RepresentationItemRef`
//! (geometry, topology, geometry representation, or 3D placement); `shape`
//! and `shape_aspect` SELECT members are silently dropped with a warning.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    ContextDependentOverRidingStyledItem, StyleContextRef, StyledItem, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::styled_item::resolve_representation_item_ref;
use step_io_macros::step_entity;

pub(crate) struct ContextDependentOverRidingStyledItemHandler;

#[step_entity(name = "CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM")]
impl SimpleEntityHandler for ContextDependentOverRidingStyledItemHandler {
    type WriteInput = ContextDependentOverRidingStyledItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            5,
            entity_id,
            "CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM",
        )?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;
        let over_ridden_ref = read_entity_ref(attrs, 3, entity_id, "over_ridden_style")?;
        let context_refs = read_entity_ref_list(attrs, 4, entity_id, "style_context")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(&psa_id) = ctx.viz_psa_id_map.get(&r) {
                styles.push(psa_id);
            }
        }
        let Some(item) = resolve_representation_item_ref(ctx, item_ref) else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM target #{item_ref} did not resolve \
                     to a modelled representation_item kind (likely cascade from a dropped \
                     dependency)"
                ),
            });
            return Ok(());
        };
        let Some(&over_ridden_style) = ctx.viz_styled_item_id_map.get(&over_ridden_ref) else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM over_ridden_style \
                     #{over_ridden_ref} did not resolve to a previously-loaded STYLED_ITEM"
                ),
            });
            return Ok(());
        };

        let mut style_context = Vec::with_capacity(context_refs.len());
        for r in &context_refs {
            if let Some(target) = resolve_representation_item_ref(ctx, *r) {
                style_context.push(StyleContextRef::RepresentationItem(target));
            } else {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM.style_context #{r} target type unsupported"
                    ),
                });
            }
        }

        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool.styled_items.push(StyledItem::ContextDependent(
            ContextDependentOverRidingStyledItem {
                name,
                styles,
                item,
                over_ridden_style,
                style_context,
            },
        ));
        ctx.viz_styled_item_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        cd: ContextDependentOverRidingStyledItem,
    ) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(cd.item)?;
        let mut style_refs = Vec::with_capacity(cd.styles.len());
        for psa_id in cd.styles {
            style_refs.push(Attribute::EntityRef(buf.psa_step_ids[psa_id.0 as usize]));
        }
        let over_ridden_step_id = buf.styled_item_step_ids[cd.over_ridden_style.0 as usize];
        let mut context_refs = Vec::with_capacity(cd.style_context.len());
        for ctx_ref in cd.style_context {
            match ctx_ref {
                StyleContextRef::RepresentationItem(target) => {
                    let step_id = buf.emit_representation_item_ref(target)?;
                    context_refs.push(Attribute::EntityRef(step_id));
                }
            }
        }
        Ok(buf.push_simple(
            "CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM",
            vec![
                Attribute::String(cd.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(item_id),
                Attribute::EntityRef(over_ridden_step_id),
                Attribute::List(context_refs),
            ],
        ))
    }
}
