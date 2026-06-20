//! `GEOMETRIC_ITEM_SPECIFIC_USAGE` handler — 2-layer path.
//!
//! Subtype of `item_identified_representation_usage` (same parent attrs as IIRU,
//! reused as the bind/lower/lift template), narrowing `definition` to a
//! shape-aspect-family ref and `identified_item` to a single `representation_item`.
//!
//! `read` = generated `bind` + hand `lower_geometric_item_specific_usage`.
//! (`identified_item` is the schema-narrowed all-entity `geometric_model_item`
//! → single ref `u64` in L1; `lower` resolves it via `resolve_representation_item_ref`.)
//! `write` resolves the three refs (`emit_shape_aspect_ref` / `step_id` /
//! `emit_representation_item_ref`) then lift + generated serialize.
//!
//! Non-standard `used_representation=$` (CATIA `GisuUnsetUsedRep`): the strict
//! generated bind rejects `$`, and the standard value (the container of
//! `identified_item`) can only be derived in a post-pass. So the `$` form is
//! intercepted before bind and deferred to `resolve_deferred_gisu_used_representation`
//! (L1 stays strict; the normalization lives in the hand layer + post-pass).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::GeometricItemSpecificUsage;
use crate::parser::entity::Attribute;
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
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 5, entity_id, "GEOMETRIC_ITEM_SPECIFIC_USAGE")?;
        // Non-standard used_representation=$ — intercept before strict bind and
        // defer (the post-pass derives the container of identified_item).
        if matches!(attrs[3], Attribute::Unset) {
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
            ctx.deferred_gisu_used_repr.push(DeferredGisu {
                entity_id,
                name,
                description,
                definition,
                identified_item,
            });
            return Ok(());
        }
        let early = bind::bind_geometric_item_specific_usage(entity_id, attrs)?;
        lower::lower_geometric_item_specific_usage(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gisu: GeometricItemSpecificUsage) -> Result<u64, WriteError> {
        let def_step = buf.emit_shape_aspect_ref(gisu.definition);
        let used_step = buf.step_id(gisu.used_representation);
        let item_step = buf.emit_representation_item_ref(gisu.identified_item)?;
        let early = lift::lift_geometric_item_specific_usage(
            gisu.name,
            gisu.description,
            def_step,
            used_step,
            item_step,
        );
        Ok(serialize::serialize_geometric_item_specific_usage(
            buf, &early,
        ))
    }
}
