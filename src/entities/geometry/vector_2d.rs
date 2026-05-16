//! `VECTOR` handler — Pass 4a-2 (2D, intermediate map).
//!
//! Sister handler of [`crate::entities::geometry::vector::VectorHandler`].
//! Like its 3D counterpart, the reader stores the resolved
//! `(Direction2dId, magnitude)` pair in `vector_2d_map` keyed by entity
//! id rather than allocating an arena entry.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::direction_2d::Direction2dHandler;
use crate::ir::Direction2dId;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct Vector2dHandler;

#[step_entity(name = "VECTOR", pass = Pass4aVector)]
impl SimpleEntityHandler for Vector2dHandler {
    type WriteInput = (Direction2dId, f64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "VECTOR")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let dir_ref = read_entity_ref(attrs, 1, entity_id, "orientation")?;
        let magnitude = read_real(attrs, 2, entity_id, "magnitude")?;
        // First cross-ref is the 2D-vs-3D discriminator: if the
        // referenced DIRECTION is not in the 2D direction map, this
        // VECTOR is the 3D variant — handled by the sister handler.
        let Some(&dir) = ctx.direction_2d_map.get(&dir_ref) else {
            return Ok(());
        };
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
