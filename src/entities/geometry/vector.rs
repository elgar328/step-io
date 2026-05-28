//! VECTOR handler — Step 1 pilot (dependency + intermediate-map case).
//!
//! Mirrors the legacy `ReaderContext::convert_vector` (`reader/geometry.rs`)
//! and the previously module-local `WriteBuffer::emit_vector`
//! (`writer/buffer/geometry.rs`). VECTOR is the second pilot because it
//! demonstrates two patterns absent from DIRECTION:
//!   - **Dependency**: read resolves an upstream DIRECTION reference.
//!   - **Intermediate map**: VECTOR is not stored in an arena. The reader
//!     keeps `(DirectionId, f64)` in `vector_map` keyed by entity id, and
//!     the writer reconstructs the entity inline at each call site.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::direction::DirectionHandler;
use crate::ir::DirectionId;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct VectorHandler;

#[step_entity(name = "VECTOR", pass = Pass2)]
impl SimpleEntityHandler for VectorHandler {
    type WriteInput = (DirectionId, f64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "VECTOR")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let dir_ref = read_entity_ref(attrs, 1, entity_id, "orientation")?;
        let magnitude = read_real(attrs, 2, entity_id, "magnitude")?;

        // If the referenced DIRECTION is a known 2D direction, this
        // VECTOR is the 2D sister variant — silently skip.
        if ctx.direction_2d_map.contains_key(&dir_ref) {
            return Ok(());
        }
        let dir_id = ctx.resolve_direction(entity_id, dir_ref, "orientation")?;

        ctx.vector_map.insert(entity_id, (dir_id, magnitude));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (direction, magnitude): (DirectionId, f64),
    ) -> Result<u64, WriteError> {
        let dir_n = DirectionHandler::write(buf, direction)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "VECTOR".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(dir_n),
                    Attribute::Real(magnitude),
                ],
            },
        });
        Ok(n)
    }
}
