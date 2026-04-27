//! `VECTOR` handler — Pass 4a-2 (2D, intermediate map).
//!
//! Sister handler of [`crate::entities::geometry::vector::VectorHandler`].
//! Like its 3D counterpart, the reader stores the resolved
//! `(Direction2dId, magnitude)` pair in `vector_2d_map` keyed by entity
//! id rather than allocating an arena entry.

use crate::entities::geometry::direction_2d::Direction2dHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::Direction2dId;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string};
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct Vector2dHandler;

impl SimpleEntityHandler for Vector2dHandler {
    const NAME: &'static str = "VECTOR";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4aVector;
    type WriteInput = (Direction2dId, f64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "VECTOR")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let dir_ref = read_entity_ref(attrs, 1, entity_id, "orientation")?;
        let magnitude = read_real(attrs, 2, entity_id, "magnitude")?;
        let dir = *ctx
            .direction_2d_map
            .get(&dir_ref)
            .ok_or(ConvertError::MissingReference {
                from: entity_id,
                to: dir_ref,
                field_name: "orientation",
            })?;
        ctx.vector_2d_map.insert(entity_id, (dir, magnitude));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (direction, magnitude): (Direction2dId, f64),
    ) -> Result<u64, WriteError> {
        let dir_n = Direction2dHandler::write(buf, direction)?;
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
static VECTOR_2D_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: Vector2dHandler::NAME,
    pass_level: Vector2dHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: Vector2dHandler::read,
    },
};
