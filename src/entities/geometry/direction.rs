//! DIRECTION handler — Step 1 pilot.
//!
//! Mirrors the legacy `ReaderContext::convert_direction` (`reader/geometry.rs`)
//! and `WriteBuffer::emit_direction` (`writer/buffer/geometry.rs`) one-to-one.
//! The legacy methods stay in place under `#[allow(dead_code)]` until Plan 2
//! introduces a registry that fully supersedes the `run_pass!` macro.

use crate::entities::EntityHandler;
use crate::ir::DirectionId;
use crate::ir::attr::{check_count, read_real_list, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Direction3;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::buffer::geometry::direction_at;
use crate::writer::entity::{WriterBody, WriterEntity};

#[allow(dead_code)] // Constructed by the Plan 2 registry; only static methods are called now.
pub(crate) struct DirectionHandler;

impl EntityHandler for DirectionHandler {
    const NAME: &'static str = "DIRECTION";
    const PASS_LEVEL: u8 = 1;
    type WriteInput = DirectionId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "DIRECTION")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let ratios = read_real_list(attrs, 1, entity_id, "direction_ratios")?;
        if ratios.len() != 3 {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "direction_ratios",
                expected: 3,
                actual: ratios.len(),
            });
        }
        let dir = Direction3 {
            x: ratios[0],
            y: ratios[1],
            z: ratios[2],
        };
        let id = ctx.geometry.directions.push(dir);
        ctx.direction_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: DirectionId) -> Result<u64, WriteError> {
        if let Some(&n) = buf.direction_ids.get(&id) {
            return Ok(n);
        }
        let d = direction_at(buf.model, id)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "DIRECTION".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::List(vec![
                        Attribute::Real(d.x),
                        Attribute::Real(d.y),
                        Attribute::Real(d.z),
                    ]),
                ],
            },
        });
        buf.direction_ids.insert(id, n);
        Ok(n)
    }
}
