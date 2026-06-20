//! `FILL_AREA_STYLE` handler. Aggregates one or more
//! `FILL_AREA_STYLE_COLOUR` entries into a named fill-area style. Pushes
//! into the shared `founded_item` arena as the `FillAreaStyle` variant.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::FillAreaStyle;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use step_io_macros::step_entity;

pub(crate) struct FillAreaStyleHandler;

#[step_entity(name = "FILL_AREA_STYLE")]
impl SimpleEntityHandler for FillAreaStyleHandler {
    type WriteInput = FillAreaStyle;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        // 2-layer path: bind → L1, then lower → L2. `lower` resolves the
        // fill_styles via viz_fasc_map and registers the typed
        // `EarlyFillAreaStyleId` key (replaces viz_fas_id_map).
        let early = bind::bind_fill_area_style(entity_id, attrs)?;
        lower::lower_fill_area_style(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, fas: FillAreaStyle) -> Result<u64, WriteError> {
        // 2-layer write path: lift L2 → L1 (emitting the inlined colours), then
        // serialize L1 → Part21 text.
        let early = lift::lift_fill_area_style(buf, &fas)?;
        Ok(serialize::serialize_fill_area_style(buf, &early))
    }
}
