//! `AREA_UNIT` handler — units (2-layer path).
//!
//! EXPRESS (AP242 ed2/ed3, AP214E3, AP203e2): `AREA_UNIT SUBTYPE OF
//! (derived_unit)` — single inheritance, with a `WHERE` clause fixing the
//! derived `dimensional_exponents` to `(2.0, 0, …)`. `dimensions` is a DERIVE
//! function, not a stored attribute, so the standard (and corpus-universal)
//! encoding is `AREA_UNIT((#e1, #e2))` — one `elements` attribute, identical
//! body to `DERIVED_UNIT`, only the entity name differs.
//!
//! Shares the `derived_unit_arena` with `DERIVED_UNIT` / `VOLUME_UNIT`; the
//! subtype is recorded via [`crate::ir::units::DerivedUnitKind::AreaUnit`] so
//! the writer re-emits the correct entity name.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AreaUnitHandler;

#[step_entity(name = "AREA_UNIT")]
impl SimpleEntityHandler for AreaUnitHandler {
    /// Element STEP entity ids, in source order (the writer's
    /// `emit_derived_unit_by_kind` supplies them for the `AreaUnit` kind).
    type WriteInput = Vec<u64>;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_area_unit(entity_id, attrs)?;
        lower::lower_area_unit(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, refs: Vec<u64>) -> Result<u64, WriteError> {
        let early = lift::lift_area_unit(refs);
        Ok(serialize::serialize_area_unit(buf, &early))
    }
}
