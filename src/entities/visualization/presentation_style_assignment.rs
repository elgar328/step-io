//! `PRESENTATION_STYLE_ASSIGNMENT` handler. Aggregates one or
//! more styling entries: `SURFACE_STYLE_USAGE` and `POINT_STYLE` (each a
//! `FoundedItemId` arena reference) and `CURVE_STYLE` (a `CurveStyleId`).
//!
//! Remaining style flavours (`FILL_AREA_STYLE` direct, `SYMBOL_STYLE`, etc.)
//! are silently dropped at read time so the writer's symmetric drop
//! preserves round-trip equality on the supported subset.

use crate::early::model::{EarlyPointStyleId, EarlySurfaceStyleUsageId};
use crate::entities::SimpleEntityHandler;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    PresentationStyleAssignment, PresentationStyleAssignmentData, PsaStyle, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "PRESENTATION_STYLE_ASSIGNMENT")?;
        let styles = parse_psa_styles(ctx, entity_id, &attrs[0]);
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .presentation_style_assignments
            .push(PresentationStyleAssignment::Itself(
                PresentationStyleAssignmentData { styles },
            ));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        data: PresentationStyleAssignmentData,
    ) -> Result<u64, WriteError> {
        let style_attrs = emit_psa_styles(buf, data.styles);
        Ok(buf.push_simple(
            "PRESENTATION_STYLE_ASSIGNMENT",
            vec![Attribute::List(style_attrs)],
        ))
    }
}

/// Parse the `styles` SET attribute of `PRESENTATION_STYLE_ASSIGNMENT` /
/// `PRESENTATION_STYLE_BY_CONTEXT`. Each member is either an
/// `EntityRef` (resolved via SSU / `CurveStyle` id maps) or a typed
/// `NULL_STYLE(.NULL.)` placeholder. Unknown variants emit a warning
/// and are skipped (visibility for future corpus variants).
pub(crate) fn parse_psa_styles(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attr: &Attribute,
) -> Vec<PsaStyle> {
    // NsCase::PsaStylesUnset `styles` is a mandatory `SET[1:?]`; some exporters
    // emit it as `$` (Unset). Accept as empty rather than a defect, matching the
    // sibling NsCase::PsaBareNullStyle policy. See reader::nonstandard.
    if matches!(attr, Attribute::Unset | Attribute::Derived) {
        ctx.ns_record(
            crate::reader::NsCase::PsaStylesUnset,
            "PRESENTATION_STYLE_ASSIGNMENT.styles (Unset)".into(),
            "()",
        );
        return Vec::new();
    }
    let Attribute::List(items) = attr else {
        ctx.warnings
            .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("PSA styles: expected List, got {attr:?}"),
            });
        return Vec::new();
    };
    let mut styles = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Attribute::EntityRef(r) => {
                if let Some(ssu_id) = ctx
                    .id_cache
                    .get::<EarlySurfaceStyleUsageId>(*r)
                    .map(|e| ctx.early.lookup_lowered(e))
                {
                    styles.push(PsaStyle::Surface(ssu_id));
                } else if let Some(ps_id) = ctx
                    .id_cache
                    .get::<EarlyPointStyleId>(*r)
                    .map(|e| ctx.early.lookup_lowered(e))
                {
                    styles.push(PsaStyle::Point(ps_id));
                } else if let Some(cs_id) = ctx.id_cache.get::<crate::ir::id::CurveStyleId>(*r) {
                    styles.push(PsaStyle::Curve(cs_id));
                }
                // Remaining style flavours (FILL_AREA_STYLE direct, SYMBOL_STYLE,
                // etc.) silently skipped. Add explicit handling here if a
                // flavour becomes corpus-relevant.
            }
            Attribute::Typed { type_name, value }
                if type_name == "NULL_STYLE"
                    && matches!(value.as_ref(), Attribute::Enum(t) if t == "NULL") =>
            {
                styles.push(PsaStyle::Null);
            }
            // NsCase::PsaBareNullStyle some exporters write a bare `.NULL.`
            // enum instead of the typed NULL_STYLE(.NULL.) placeholder → accept
            // and re-emit the standard typed form. See reader::nonstandard.
            Attribute::Enum(t) if t == "NULL" => {
                ctx.ns_record(
                    crate::reader::NsCase::PsaBareNullStyle,
                    "PRESENTATION_STYLE_ASSIGNMENT.styles (bare .NULL.)".into(),
                    "NULL_STYLE(.NULL.)",
                );
                styles.push(PsaStyle::Null);
            }
            other => {
                ctx.warnings
                    .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                        entity_id,
                        detail: format!("PSA styles member: unknown variant {other:?}"),
                    });
            }
        }
    }
    styles
}

/// Emit the `styles` SET — inverse of [`parse_psa_styles`]. `PsaStyle::Null`
/// produces the typed `NULL_STYLE(.NULL.)` placeholder.
pub(crate) fn emit_psa_styles(buf: &WriteBuffer, styles: Vec<PsaStyle>) -> Vec<Attribute> {
    styles
        .into_iter()
        .map(|style| match style {
            PsaStyle::Surface(ssu_id) => Attribute::EntityRef(buf.step_id(ssu_id)),
            PsaStyle::Point(ps_id) => Attribute::EntityRef(buf.step_id(ps_id)),
            PsaStyle::Curve(cs_id) => Attribute::EntityRef(buf.step_id(cs_id)),
            PsaStyle::Null => Attribute::Typed {
                type_name: "NULL_STYLE".into(),
                value: Box::new(Attribute::Enum("NULL".into())),
            },
        })
        .collect()
}
