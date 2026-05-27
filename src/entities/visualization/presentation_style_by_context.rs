//! `PRESENTATION_STYLE_BY_CONTEXT` handler — phase psbc.
//!
//! `presentation_style_assignment` SUBTYPE that adds `style_context`
//! (a `style_context_select`). The `styles` field reuses the parent
//! PSA's `PsaStyle` enum (`SURFACE_STYLE_USAGE` + `CURVE_STYLE`).
//! `style_context` is narrowed to the two corpus-common SELECT
//! members (`representation` / `representation_item`); other members
//! drop the carrier on read (symmetric).

use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    PresentationStyleAssignment, PresentationStyleByContext, StyleContext, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PresentationStyleByContextHandler;

#[step_entity(name = "PRESENTATION_STYLE_BY_CONTEXT", pass = Pass7Assignment)]
impl SimpleEntityHandler for PresentationStyleByContextHandler {
    type WriteInput = PresentationStyleByContext;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PRESENTATION_STYLE_BY_CONTEXT")?;
        let ctx_ref = read_entity_ref(attrs, 1, entity_id, "style_context")?;
        let style_context = if let Some(&rid) = ctx.repr_id_map.get(&ctx_ref) {
            StyleContext::Representation(rid)
        } else if let Some(item) = resolve_representation_item_ref(ctx, ctx_ref) {
            StyleContext::Item(item)
        } else {
            return Ok(());
        };
        let styles =
            crate::entities::visualization::presentation_style_assignment::parse_psa_styles(
                ctx, entity_id, &attrs[0],
            );
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool.presentation_style_assignments.push(
            PresentationStyleAssignment::PresentationStyleByContext(PresentationStyleByContext {
                styles,
                style_context,
            }),
        );
        ctx.viz_psa_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, psbc: PresentationStyleByContext) -> Result<u64, WriteError> {
        let style_attrs =
            crate::entities::visualization::presentation_style_assignment::emit_psa_styles(
                buf,
                psbc.styles,
            );
        let ctx_step = match psbc.style_context {
            StyleContext::Representation(rid) => buf.representation_step_ids[rid.0 as usize],
            StyleContext::Item(item) => buf.emit_representation_item_ref(item)?,
        };
        Ok(buf.push_simple(
            "PRESENTATION_STYLE_BY_CONTEXT",
            vec![Attribute::List(style_attrs), Attribute::EntityRef(ctx_step)],
        ))
    }
}
