//! `DERIVED_UNIT` handler — units (2-layer path). The named subtypes
//! (`AREA_UNIT` / `VOLUME_UNIT`) are also 2-layer (single `derived_unit`
//! inheritance, one-attr `elements` body) in their own handlers, sharing the
//! `derived_unit_arena` via [`crate::ir::units::DerivedUnitKind`].

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DerivedUnitHandler;

#[step_entity(name = "DERIVED_UNIT")]
impl SimpleEntityHandler for DerivedUnitHandler {
    /// Element STEP entity ids, in source order. Serialize wraps them in a
    /// single `Attribute::List(EntityRef…)`.
    type WriteInput = Vec<u64>;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_derived_unit(entity_id, attrs)?;
        lower::lower_derived_unit(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, refs: Vec<u64>) -> Result<u64, WriteError> {
        let early = lift::lift_derived_unit(refs);
        Ok(serialize::serialize_derived_unit(buf, &early))
    }
}
