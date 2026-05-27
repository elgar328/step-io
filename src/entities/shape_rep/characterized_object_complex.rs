//! `(CHARACTERIZED_OBJECT CHARACTERIZED_REPRESENTATION ...)` complex MI
//! handler — phase characterized-min.
//!
//! Detects the corpus 100%-complex-MI form where `CHARACTERIZED_OBJECT`
//! and `CHARACTERIZED_REPRESENTATION` parts always co-occur (with
//! `DRAUGHTING_MODEL` / `REPRESENTATION` / `SHAPE_REPRESENTATION` /
//! `TESSELLATED_SHAPE_REPRESENTATION` companions). The `CHARACTERIZED_OBJECT`
//! part itself carries `(*, *)` (both attrs DERIVE); the `name` value
//! lives in the `REPRESENTATION` part — extracted here. Other parts'
//! data is discarded (minimal scope).

use crate::entities::ComplexEntityHandler;
use crate::ir::attr::read_string_or_unset;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{CharacterizedObject, CharacterizedObjectData};
use crate::parser::entity::{EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct CharacterizedObjectComplexHandler;

#[step_entity_complex(
    name = "CHARACTERIZED_OBJECT",
    pass = Pass8CharacterizedComplex,
    required = ["CHARACTERIZED_OBJECT", "CHARACTERIZED_REPRESENTATION"]
)]
impl ComplexEntityHandler for CharacterizedObjectComplexHandler {
    type WriteInput = CharacterizedObjectData;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // REPRESENTATION part is always co-instantiated per ir.toml
        // (verified: 48/48 inst). Extract `name` from its first attr.
        let repr_attrs = require_part_attrs(parts, "REPRESENTATION", entity_id)?;
        let name = read_string_or_unset(repr_attrs, 0, entity_id, "name")?.to_owned();
        ctx.characterized_objects
            .push(CharacterizedObject::Itself(CharacterizedObjectData {
                name,
                description: None,
            }));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: CharacterizedObjectData) -> Result<u64, WriteError> {
        use crate::parser::entity::Attribute;
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
