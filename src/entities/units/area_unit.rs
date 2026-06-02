//! `AREA_UNIT` handler — Pass 0-4 (units-3a).
//!
//! EXPRESS (`AP203e2` / `AP214e3`): `AREA_UNIT SUBTYPE OF (derived_unit)`
//! with a `WHERE` clause fixing `dimensional_exponents` to `(2.0, 0, ...)`.
//! Carries no extra attributes beyond the inherited `elements` set, so the
//! corpus form is `AREA_UNIT((#e1, #e2))` — identical body to
//! `DERIVED_UNIT`, only the entity name differs.
//!
//! Shares the `derived_unit_arena` with `DERIVED_UNIT` / `VOLUME_UNIT`;
//! the subtype is recorded via [`DerivedUnitKind::AreaUnit`] so the writer
//! can re-emit the correct entity name.

use crate::entities::SimpleEntityHandler;
use crate::entities::units::derived_unit::emit_derived_unit_named;
use crate::ir::attr::{check_count, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::units::{DerivedUnit, DerivedUnitKind};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AreaUnitHandler;

#[step_entity(name = "AREA_UNIT")]
impl SimpleEntityHandler for AreaUnitHandler {
    type WriteInput = Vec<u64>;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "AREA_UNIT")?;
        let refs = read_entity_ref_list(attrs, 0, entity_id, "elements")?;
        let mut elements = Vec::with_capacity(refs.len());
        for r in refs {
            if let Some(&due_id) = ctx.due_id_map.get(&r) {
                elements.push(due_id);
            }
        }
        if elements.is_empty() {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: "AREA_UNIT has no resolvable elements (schema WHERE: SET[1:?])".into(),
            });
            return Ok(());
        }
        let id = ctx.derived_unit_arena.push(DerivedUnit {
            elements,
            kind: DerivedUnitKind::AreaUnit,
        });
        ctx.derived_unit_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, refs: Vec<u64>) -> Result<u64, WriteError> {
        Ok(emit_derived_unit_named(buf, "AREA_UNIT", refs))
    }
}
