//! `SURFACE_OF_REVOLUTION` handler — Pass 4-4A.
//!
//! Mirrors `ReaderContext::convert_surface_of_revolution` and
//! `WriteBuffer::emit_surface_of_revolution`.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis1_placement::Axis1PlacementHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Surface, SurfaceOfRevolution};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct SurfaceOfRevolutionHandler;

#[step_entity(name = "SURFACE_OF_REVOLUTION", pass = Pass4_4Swept)]
impl SimpleEntityHandler for SurfaceOfRevolutionHandler {
    type WriteInput = SurfaceOfRevolution;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SURFACE_OF_REVOLUTION")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let curve_ref = read_entity_ref(attrs, 1, entity_id, "swept_curve")?;
        let axis_ref = read_entity_ref(attrs, 2, entity_id, "axis_position")?;

        let swept_curve = ctx.resolve_curve(entity_id, curve_ref, "swept_curve")?;
        let axis_placement = ctx.resolve_axis1(entity_id, axis_ref, "axis_position")?;

        let surface = SurfaceOfRevolution {
            swept_curve,
            axis_placement,
        };
        let id = ctx.geometry.surfaces.push(Surface::Revolution(surface));
        ctx.surface_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, r: SurfaceOfRevolution) -> Result<u64, WriteError> {
        let swept = buf.emit_curve(r.swept_curve)?;
        let axis = Axis1PlacementHandler::write(buf, r.axis_placement)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "SURFACE_OF_REVOLUTION".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(swept),
                    Attribute::EntityRef(axis),
                ],
            },
        });
        Ok(n)
    }
}
