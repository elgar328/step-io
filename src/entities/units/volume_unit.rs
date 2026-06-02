//! `VOLUME_UNIT` handler — Pass 0-4 (units-3a).
//!
//! Sister of [`crate::entities::units::area_unit::AreaUnitHandler`].
//! EXPRESS: `VOLUME_UNIT SUBTYPE OF (derived_unit)` with `WHERE`
//! clause fixing `dimensional_exponents` to `(3.0, 0, ...)`.

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

pub(crate) struct VolumeUnitHandler;

#[step_entity(name = "VOLUME_UNIT")]
impl SimpleEntityHandler for VolumeUnitHandler {
    type WriteInput = Vec<u64>;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "VOLUME_UNIT")?;
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
                detail: "VOLUME_UNIT has no resolvable elements (schema WHERE: SET[1:?])".into(),
            });
            return Ok(());
        }
        let id = ctx.derived_unit_arena.push(DerivedUnit {
            elements,
            kind: DerivedUnitKind::VolumeUnit,
        });
        ctx.derived_unit_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, refs: Vec<u64>) -> Result<u64, WriteError> {
        Ok(emit_derived_unit_named(buf, "VOLUME_UNIT", refs))
    }
}
