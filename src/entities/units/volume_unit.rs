//! `VOLUME_UNIT` handler — units (2-layer path).
//!
//! Sister of [`crate::entities::units::area_unit::AreaUnitHandler`].
//! EXPRESS: `VOLUME_UNIT SUBTYPE OF (derived_unit)` — single inheritance, with
//! a `WHERE` clause fixing the derived `dimensional_exponents` to `(3.0, 0, …)`.
//! Standard encoding: `VOLUME_UNIT((#e1, #e2))` — one `elements` attribute.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct VolumeUnitHandler;

#[step_entity(name = "VOLUME_UNIT")]
impl SimpleEntityHandler for VolumeUnitHandler {
    /// Element STEP entity ids, in source order (the writer's
    /// `emit_derived_unit_by_kind` supplies them for the `VolumeUnit` kind).
    type WriteInput = Vec<u64>;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_volume_unit(entity_id, attrs)?;
        lower::lower_volume_unit(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, refs: Vec<u64>) -> Result<u64, WriteError> {
        let early = lift::lift_volume_unit(refs);
        Ok(serialize::serialize_volume_unit(buf, &early))
    }
}
