//! `SURFACE_OF_LINEAR_EXTRUSION` handler — Pass 4-4A.
//!
//! Mirrors `ReaderContext::convert_surface_of_linear_extrusion` and
//! `WriteBuffer::emit_surface_of_linear_extrusion`.

use crate::entities::geometry::vector::VectorHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Surface, SurfaceOfLinearExtrusion};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct SurfaceOfLinearExtrusionHandler;

impl SimpleEntityHandler for SurfaceOfLinearExtrusionHandler {
    const NAME: &'static str = "SURFACE_OF_LINEAR_EXTRUSION";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4_4Swept;
    type WriteInput = SurfaceOfLinearExtrusion;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SURFACE_OF_LINEAR_EXTRUSION")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static SURFACE_OF_LINEAR_EXTRUSION_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: SurfaceOfLinearExtrusionHandler::NAME,
    pass_level: SurfaceOfLinearExtrusionHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: SurfaceOfLinearExtrusionHandler::read,
    },
};
