//! `DERIVED_UNIT` handler — units (2-layer path). The named subtypes
//! (`AREA_UNIT` / `VOLUME_UNIT`) keep their own handlers (their early.toml
//! shape is the two-attr multiple-inheritance form, which the corpus's
//! one-attr files do not match — legacy parse retained there).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
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

/// Emit a named derived-unit line (`AREA_UNIT` / `VOLUME_UNIT`) — the
/// non-migrated named subtypes share this single-list shape.
pub(crate) fn emit_derived_unit_named(
    buf: &mut WriteBuffer,
    name: &'static str,
    refs: Vec<u64>,
) -> u64 {
    let list = refs.into_iter().map(Attribute::EntityRef).collect();
    buf.push_simple(name, vec![Attribute::List(list)])
}
