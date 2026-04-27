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

use crate::entities::geometry::direction::DirectionHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::DirectionId;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string};
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct VectorHandler;

impl SimpleEntityHandler for VectorHandler {
    const NAME: &'static str = "VECTOR";
    const PASS_LEVEL: PassLevel = PassLevel::Pass2;
    type WriteInput = (DirectionId, f64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "VECTOR")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let dir_ref = read_entity_ref(attrs, 1, entity_id, "orientation")?;
        let magnitude = read_real(attrs, 2, entity_id, "magnitude")?;

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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static VECTOR_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: VectorHandler::NAME,
    pass_level: VectorHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: VectorHandler::read,
    },
};
