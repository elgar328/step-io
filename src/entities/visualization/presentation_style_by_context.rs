//! `PRESENTATION_STYLE_BY_CONTEXT` handler — phase psbc.
//!
//! `presentation_style_assignment` SUBTYPE that adds `style_context`
//! (a `style_context_select`). The `styles` field reuses the parent
//! PSA's `PsaStyle` enum (`SURFACE_STYLE_USAGE` + `CURVE_STYLE`).
//! `style_context` is narrowed to the two corpus-common SELECT
//! members (`representation` / `representation_item`); other members
//! drop the carrier on read (symmetric).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::presentation_style_assignment::normalize_psa_styles_attr;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::visualization::PresentationStyleByContext;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PresentationStyleByContextHandler;

#[step_entity(name = "PRESENTATION_STYLE_BY_CONTEXT")]
impl SimpleEntityHandler for PresentationStyleByContextHandler {
    type WriteInput = PresentationStyleByContext;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PRESENTATION_STYLE_BY_CONTEXT")?;
        // Pre-bind normalization of the `styles` SET (shared with PSA), keeping
        // the generated bind schema-strict; `lower` resolves both fields.
        let mut norm = attrs.to_vec();
        norm[0] = normalize_psa_styles_attr(ctx, &attrs[0]);
        let early = bind::bind_presentation_style_by_context(entity_id, &norm)?;
        lower::lower_presentation_style_by_context(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, psbc: PresentationStyleByContext) -> Result<u64, WriteError> {
        let early = lift::lift_presentation_style_by_context(buf, &psbc)?;
        Ok(serialize::serialize_presentation_style_by_context(
            buf, &early,
        ))
    }
}
