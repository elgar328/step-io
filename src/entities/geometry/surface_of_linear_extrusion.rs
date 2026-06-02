//! `SURFACE_OF_LINEAR_EXTRUSION` handler — Pass 4-4A.
//!
//! Mirrors `ReaderContext::convert_surface_of_linear_extrusion` and
//! `WriteBuffer::emit_surface_of_linear_extrusion`.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::vector::VectorHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Surface, SurfaceOfLinearExtrusion};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct SurfaceOfLinearExtrusionHandler;

#[step_entity(name = "SURFACE_OF_LINEAR_EXTRUSION")]
impl SimpleEntityHandler for SurfaceOfLinearExtrusionHandler {
    type WriteInput = SurfaceOfLinearExtrusion;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SURFACE_OF_LINEAR_EXTRUSION")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let curve_ref = read_entity_ref(attrs, 1, entity_id, "swept_curve")?;
        let vector_ref = read_entity_ref(attrs, 2, entity_id, "extrusion_axis")?;

        let swept_curve = ctx.resolve_curve(entity_id, curve_ref, "swept_curve")?;
        let (extrusion_direction, depth) =
            ctx.resolve_vector(entity_id, vector_ref, "extrusion_axis")?;

        let surface = SurfaceOfLinearExtrusion {
            swept_curve,
            extrusion_direction,
            depth,
        };
        let id = ctx.geometry.surfaces.push(Surface::Extrusion(surface));
        ctx.surface_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, e: SurfaceOfLinearExtrusion) -> Result<u64, WriteError> {
        let swept = buf.emit_curve(e.swept_curve)?;
        let vector = VectorHandler::write(buf, (e.extrusion_direction, e.depth))?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "SURFACE_OF_LINEAR_EXTRUSION".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(swept),
                    Attribute::EntityRef(vector),
                ],
            },
        });
        Ok(n)
    }
}
