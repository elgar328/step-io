//! `DERIVED_UNIT` handler — Pass 0-4 (units-1b).
//!
//! AP214: `DERIVED_UNIT(elements: SET[1:?] OF derived_unit_element)`.
//! STEP positional: single `(#a,#b,...)` list. The reader resolves each
//! element ref through [`crate::reader::ReaderContext::due_id_map`]
//! (populated by `Pass0MwuDue`); refs that don't resolve are dropped with
//! a warning. The schema's `[1:?]` cardinality is enforced post-resolve —
//! a `DERIVED_UNIT` whose `elements` resolve to an empty `Vec` is
//! dropped rather than admitted to the arena.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::units::DerivedUnit;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct DerivedUnitHandler;

#[step_entity(name = "DERIVED_UNIT", pass = Pass0Du)]
impl SimpleEntityHandler for DerivedUnitHandler {
    /// Element STEP entity ids, in source order. Writer wraps them in a
    /// single `Attribute::List(EntityRef…)`.
    type WriteInput = Vec<u64>;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "DERIVED_UNIT")?;
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
                detail: "DERIVED_UNIT has no resolvable elements (schema WHERE: SET[1:?])".into(),
            });
            return Ok(());
        }
        let id = ctx.derived_unit_arena.push(DerivedUnit { elements });
        ctx.derived_unit_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, refs: Vec<u64>) -> Result<u64, WriteError> {
        let list = refs.into_iter().map(Attribute::EntityRef).collect();
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "DERIVED_UNIT".into(),
                attrs: vec![Attribute::List(list)],
            },
        });
        Ok(n)
    }
}
