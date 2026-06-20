//! `CURVE_STYLE` handler — combines a curve-font reference, a width measure
//! (`size_select`), and a colour reference, pushed into
//! `VisualizationPool::curve_styles`. 2-layer path: bind → L1, lower → L2.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::CurveStyle;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CurveStyleHandler;

#[step_entity(name = "CURVE_STYLE")]
impl SimpleEntityHandler for CurveStyleHandler {
    type WriteInput = CurveStyle;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        // 2-layer path: bind → L1, then lower → L2. `lower` resolves the
        // curve_font / curve_width / colour refs via the id_cache and registers
        // the `CurveStyleId` key the PSA / styled-item consumers probe.
        if let Some(early) = bind::bind_curve_style(entity_id, attrs)? {
            lower::lower_curve_style(ctx, entity_id, early);
        } else {
            // bind returned None because a present `curve_width` (`size_select`)
            // carried a non-standard member. Rejecting it is correct → drop +
            // NORM, not a silent loss.
            ctx.ns_push(
                crate::reader::NsCase::NonStandardEnumValue,
                "CURVE_STYLE".into(),
                2,
                "dropped (non-standard curve_width value)".into(),
            );
            ctx.nonstandard_dropped_refs.insert(entity_id);
        }
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, cs: CurveStyle) -> Result<u64, WriteError> {
        // 2-layer write path: lift L2 → L1, then serialize L1 → Part21 text.
        let early = lift::lift_curve_style(buf, &cs);
        Ok(serialize::serialize_curve_style(buf, &early))
    }
}
