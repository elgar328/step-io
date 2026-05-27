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
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    PresentationStyleAssignment, PresentationStyleByContext, PsaStyle, StyleContext,
    VisualizationPool,
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
        let style_refs = read_entity_ref_list(attrs, 0, entity_id, "styles")?;
        let ctx_ref = read_entity_ref(attrs, 1, entity_id, "style_context")?;
        let style_context = if let Some(&rid) = ctx.repr_id_map.get(&ctx_ref) {
            StyleContext::Representation(rid)
        } else if let Some(item) = resolve_representation_item_ref(ctx, ctx_ref) {
            StyleContext::Item(item)
        } else {
            return Ok(());
        };
        let mut styles = Vec::new();
        for r in style_refs {
            if let Some(&ssu_id) = ctx.viz_ssu_id_map.get(&r) {
                styles.push(PsaStyle::Surface(ssu_id));
            } else if let Some(&cs_id) = ctx.viz_curve_style_id_map.get(&r) {
                styles.push(PsaStyle::Curve(cs_id));
            }
        }
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
        let mut style_refs = Vec::with_capacity(psbc.styles.len());
        for style in psbc.styles {
            let ref_id = match style {
                PsaStyle::Surface(ssu_id) => buf.founded_item_step_ids[ssu_id.0 as usize],
                PsaStyle::Curve(cs_id) => buf.curve_style_step_ids[cs_id.0 as usize],
            };
            style_refs.push(Attribute::EntityRef(ref_id));
        }
        let ctx_step = match psbc.style_context {
            StyleContext::Representation(rid) => buf.representation_step_ids[rid.0 as usize],
            StyleContext::Item(item) => buf.emit_representation_item_ref(item)?,
        };
        Ok(buf.push_simple(
            "PRESENTATION_STYLE_BY_CONTEXT",
            vec![Attribute::List(style_refs), Attribute::EntityRef(ctx_step)],
        ))
    }
}
