//! `DERIVED_UNIT_ELEMENT(unit, exponent)` handler (units-1).
//!
//! STEP positional order is `(unit_ref, exponent_real)` — opposite of the
//! ir.toml blueprint's alphabetical (exponent, unit) field declaration.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_real};
use crate::ir::error::ConvertError;
use crate::ir::units::DerivedUnitElement;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct DerivedUnitElementHandler;

#[step_entity(name = "DERIVED_UNIT_ELEMENT")]
impl SimpleEntityHandler for DerivedUnitElementHandler {
    /// `(unit_step_id, exponent)` — STEP positional `(unit, exponent)`.
    type WriteInput = (u64, f64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "DERIVED_UNIT_ELEMENT")?;
        let unit_step = read_entity_ref(attrs, 0, entity_id, "unit")?;
        let exponent = read_real(attrs, 1, entity_id, "exponent")?;
        let Some(unit_id) = ctx.id_cache.get::<crate::ir::id::NamedUnitId>(unit_step) else {
            return Ok(());
        };
        let id = ctx.due_arena.push(DerivedUnitElement {
            unit: unit_id,
            exponent,
        });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, (unit_step, exponent): (u64, f64)) -> Result<u64, WriteError> {
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "DERIVED_UNIT_ELEMENT".into(),
                attrs: vec![Attribute::EntityRef(unit_step), Attribute::Real(exponent)],
            },
        });
        Ok(n)
    }
}
