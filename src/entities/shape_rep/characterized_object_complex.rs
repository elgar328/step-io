//! Single owner of the `DRAUGHTING_MODEL`-family complex MI — phase
//! exact-case-merge, now 2-layer. Three exact part-sets co-occur in the corpus:
//!   A `(CHARACTERIZED_OBJECT CHARACTERIZED_REPRESENTATION DRAUGHTING_MODEL
//!      REPRESENTATION)`                                     → `Characterized`
//!   C `(DRAUGHTING_MODEL REPRESENTATION SHAPE_REPRESENTATION
//!      TESSELLATED_SHAPE_REPRESENTATION)`                   → `ShapeTessellated`
//!   B their union (6 parts)              → `CharacterizedShapeTessellated`
//! All three carry data only on the `REPRESENTATION` part (name / items /
//! context); the `CHARACTERIZED_OBJECT` part is `(*, *)` (both DERIVE). One
//! handler claims all three so no two handlers fight over the 6-part form.
//!
//! `read_complex` = generated `bind` (exact part-set dispatch) + hand
//! `lower_characterized_object_complex` (2-arena: the `DRAUGHTING_MODEL` facet +
//! the `CHARACTERIZED_OBJECT::Itself` carrier, with a dual `id_cache` insert).
//! The DM-driven complex `write` lives in `draughting_model.rs`; this handler's
//! `write` only emits the standalone simple `CHARACTERIZED_OBJECT(name, $)`.

use crate::early::{bind, lower};
use crate::entities::ComplexEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::CharacterizedObjectData;
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct CharacterizedObjectComplexHandler;

#[step_entity_complex(
    name = "CHARACTERIZED_OBJECT",
    cases = [
        ["CHARACTERIZED_OBJECT", "CHARACTERIZED_REPRESENTATION", "DRAUGHTING_MODEL", "REPRESENTATION"],
        ["CHARACTERIZED_OBJECT", "CHARACTERIZED_REPRESENTATION", "DRAUGHTING_MODEL", "REPRESENTATION", "SHAPE_REPRESENTATION", "TESSELLATED_SHAPE_REPRESENTATION"],
        ["DRAUGHTING_MODEL", "REPRESENTATION", "SHAPE_REPRESENTATION", "TESSELLATED_SHAPE_REPRESENTATION"],
    ]
)]
impl ComplexEntityHandler for CharacterizedObjectComplexHandler {
    type WriteInput = CharacterizedObjectData;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_characterized_object_complex(entity_id, parts)?;
        lower::lower_characterized_object_complex(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: CharacterizedObjectData) -> Result<u64, WriteError> {
        let desc = match data.description {
            Some(d) => Attribute::String(d),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "CHARACTERIZED_OBJECT",
            vec![Attribute::String(data.name), desc],
        ))
    }
}
