//! `TRIMMED_CURVE` handler (2-layer path).
//!
//! `trim_1`/`trim_2` are `SET OF trimming_select` (`cartesian_point` ref or
//! `parameter_value` real). Non-standard TAG-less bare reals (`( 0.0 )`) are
//! accepted by the generated `bind_trimming_select` and normalized to
//! `PARAMETER_VALUE(0.0)` on write.

use crate::early::model::EarlyTrimSelect;
use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::{TrimSelect, TrimmedCurve};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct TrimmedCurveHandler;

#[step_entity(name = "TRIMMED_CURVE")]
impl SimpleEntityHandler for TrimmedCurveHandler {
    type WriteInput = TrimmedCurve;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // NsCase::TaglessParameterValue some exporters write trim parameters as
        // bare reals `( 0.0 )` instead of `PARAMETER_VALUE(0.0)`. The strict
        // generated bind only accepts the tagged form, so normalize the input
        // before binding and surface the recovery.
        let (attrs, normalized) =
            crate::ir::attr::normalize_tagless_select(attrs, &[2, 3], "PARAMETER_VALUE");
        if normalized > 0 {
            ctx.ns_push(
                crate::reader::NsCase::TaglessParameterValue,
                "TRIMMED_CURVE.trim".into(),
                normalized,
                "PARAMETER_VALUE(real)".into(),
            );
        }
        let early = bind::bind_trimmed_curve(entity_id, &attrs)?;
        lower::lower_trimmed_curve(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, trimmed: TrimmedCurve) -> Result<u64, WriteError> {
        let basis = buf.emit_curve(trimmed.basis)?;
        let trim_1 = emit_trim_select(buf, &trimmed.trim_1)?;
        let trim_2 = emit_trim_select(buf, &trimmed.trim_2)?;
        let early = lift::lift_trimmed_curve(
            basis,
            trim_1,
            trim_2,
            trimmed.sense_agreement,
            trimmed.master,
        );
        Ok(serialize::serialize_trimmed_curve(buf, &early))
    }
}

/// Resolve a trim slot's L2 `TrimSelect`s to `EarlyTrimSelect`s, emitting each
/// `Point`'s `CARTESIAN_POINT` on demand (the `Param` reals pass through).
fn emit_trim_select(
    buf: &mut WriteBuffer,
    items: &[TrimSelect],
) -> Result<Vec<EarlyTrimSelect>, WriteError> {
    let mut out = Vec::with_capacity(items.len());
    for sel in items {
        out.push(match *sel {
            TrimSelect::Point(p) => EarlyTrimSelect::Point(CartesianPointHandler::write(buf, p)?),
            TrimSelect::Param(v) => EarlyTrimSelect::Param(v),
        });
    }
    Ok(out)
}
