//! `SURFACE_STYLE_PARAMETER_LINE` handler — 2-layer path. `founded_item` subtype
//! with a `curve_or_render` SELECT and a `SET OF direction_count_select`
//! (U / V typed integers). Reuses the generated bind/serialize (hint-less synth
//! select + vec-select).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::SurfaceStyleParameterLine;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SurfaceStyleParameterLineHandler;

#[step_entity(name = "SURFACE_STYLE_PARAMETER_LINE")]
impl SimpleEntityHandler for SurfaceStyleParameterLineHandler {
    type WriteInput = SurfaceStyleParameterLine;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_surface_style_parameter_line(entity_id, attrs)?;
        lower::lower_surface_style_parameter_line(ctx, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, sspl: SurfaceStyleParameterLine) -> Result<u64, WriteError> {
        let style_step = sspl.style_of_parameter_lines.emit_select(buf);
        let early = lift::lift_surface_style_parameter_line(style_step, sspl.direction_counts);
        Ok(serialize::serialize_surface_style_parameter_line(
            buf, &early,
        ))
    }
}
