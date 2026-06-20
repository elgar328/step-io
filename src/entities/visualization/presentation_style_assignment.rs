//! `PRESENTATION_STYLE_ASSIGNMENT` handler. Aggregates one or
//! more styling entries: `SURFACE_STYLE_USAGE` and `POINT_STYLE` (each a
//! `FoundedItemId` arena reference) and `CURVE_STYLE` (a `CurveStyleId`).
//!
//! Remaining style flavours (`FILL_AREA_STYLE` direct, `SYMBOL_STYLE`, etc.)
//! are silently dropped at read time so the writer's symmetric drop
//! preserves round-trip equality on the supported subset.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::visualization::PresentationStyleAssignmentData;
use crate::parser::entity::Attribute;
use crate::reader::{NsCase, ReaderContext};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use step_io_macros::step_entity;

pub(crate) struct PresentationStyleAssignmentHandler;

#[step_entity(name = "PRESENTATION_STYLE_ASSIGNMENT")]
impl SimpleEntityHandler for PresentationStyleAssignmentHandler {
    type WriteInput = PresentationStyleAssignmentData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "PRESENTATION_STYLE_ASSIGNMENT")?;
        // Pre-bind normalization of the `styles` SET (shared with PSBC), keeping
        // the generated bind schema-strict; `lower` resolves the members.
        let mut norm = attrs.to_vec();
        norm[0] = normalize_psa_styles_attr(ctx, &attrs[0]);
        let early = bind::bind_presentation_style_assignment(entity_id, &norm)?;
        lower::lower_presentation_style_assignment(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        data: PresentationStyleAssignmentData,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_presentation_style_assignment(buf, &data);
        Ok(serialize::serialize_presentation_style_assignment(
            buf, &early,
        ))
    }
}

/// Normalize the `styles` SET attribute of `PRESENTATION_STYLE_ASSIGNMENT` /
/// `PRESENTATION_STYLE_BY_CONTEXT` to its schema-standard form *before* the
/// (strict) generated bind, recording the `NsCase` so NORM is preserved (bind
/// has no `ctx`):
///  - `$`/`*` (mandatory `SET[1:?]` emitted unset) → empty SET.
///  - bare `.NULL.` member → the typed `NULL_STYLE(.NULL.)` placeholder.
///
/// Any other attribute is passed through unchanged (the generated bind surfaces
/// it). The actual member resolution / drop of unmodelled flavours happens in
/// `lower` (`lower_psa_styles`).
pub(crate) fn normalize_psa_styles_attr(ctx: &mut ReaderContext, attr: &Attribute) -> Attribute {
    match attr {
        Attribute::Unset | Attribute::Derived => {
            ctx.ns_record(
                NsCase::PsaStylesUnset,
                "PRESENTATION_STYLE_ASSIGNMENT.styles (Unset)".into(),
                "()",
            );
            Attribute::List(Vec::new())
        }
        Attribute::List(items) => {
            let mut new_items = Vec::with_capacity(items.len());
            for it in items {
                match it {
                    Attribute::Enum(t) if t == "NULL" => {
                        ctx.ns_record(
                            NsCase::PsaBareNullStyle,
                            "PRESENTATION_STYLE_ASSIGNMENT.styles (bare .NULL.)".into(),
                            "NULL_STYLE(.NULL.)",
                        );
                        new_items.push(Attribute::Typed {
                            type_name: "NULL_STYLE".into(),
                            value: Box::new(Attribute::Enum("NULL".into())),
                        });
                    }
                    _ => new_items.push(it.clone()),
                }
            }
            Attribute::List(new_items)
        }
        other => other.clone(),
    }
}
