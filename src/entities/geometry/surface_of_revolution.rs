//! `SURFACE_OF_REVOLUTION` handler — Pass 4-4A.
//!
//! Mirrors `ReaderContext::convert_surface_of_revolution` and
//! `WriteBuffer::emit_surface_of_revolution`.

use crate::entities::geometry::axis1_placement::Axis1PlacementHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Surface, SurfaceOfRevolution};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct SurfaceOfRevolutionHandler;

impl SimpleEntityHandler for SurfaceOfRevolutionHandler {
    const NAME: &'static str = "SURFACE_OF_REVOLUTION";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4_4Swept;
    type WriteInput = SurfaceOfRevolution;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SURFACE_OF_REVOLUTION")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static SURFACE_OF_REVOLUTION_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: SurfaceOfRevolutionHandler::NAME,
    pass_level: SurfaceOfRevolutionHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: SurfaceOfRevolutionHandler::read,
    },
};
