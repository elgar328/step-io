//! `SURFACE_STYLE_RENDERING` handler (Pass 7-6) — the `Itself` variant of
//! the `surface_style_rendering` arena. Same shape as the
//! `_WITH_PROPERTIES` subtype minus the `properties` list. Not currently
//! observed in the reference-check corpus; the handler exists to align
//! with the ir.toml blueprint so a future kernel adapter emitting plain
//! `SURFACE_STYLE_RENDERING` round-trips losslessly.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_enum};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    ShadingMethod, SurfaceStyleRendering, SurfaceStyleRenderingData, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SurfaceStyleRenderingHandler;

#[step_entity(name = "SURFACE_STYLE_RENDERING", pass = Pass7Rendering)]
impl SimpleEntityHandler for SurfaceStyleRenderingHandler {
    type WriteInput = SurfaceStyleRenderingData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SURFACE_STYLE_RENDERING")?;
        let rendering_method = if matches!(attrs.first(), Some(Attribute::Enum(_))) {
            match read_enum(attrs, 0, entity_id, "rendering_method")? {
                "CONSTANT_SHADING" => Some(ShadingMethod::Constant),
                "COLOUR_SHADING" => Some(ShadingMethod::Colour),
                "DOT_SHADING" => Some(ShadingMethod::Dot),
                "NORMAL_SHADING" => Some(ShadingMethod::Normal),
                _ => None,
            }
        } else {
            None
        };
        let colour_ref = read_entity_ref(attrs, 1, entity_id, "surface_colour")?;
        let Some(&surface_colour) = ctx.viz_colour_id_map.get(&colour_ref) else {
            return Ok(());
        };
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .surface_style_renderings
            .push(SurfaceStyleRendering::Itself(SurfaceStyleRenderingData {
                rendering_method,
                surface_colour,
            }));
        ctx.viz_ssr_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: SurfaceStyleRenderingData) -> Result<u64, WriteError> {
        let colour_step_id = buf.colour_step_ids[data.surface_colour.0 as usize];
        Ok(buf.push_simple(
            "SURFACE_STYLE_RENDERING",
            vec![
                shading_method_attr(data.rendering_method),
                Attribute::EntityRef(colour_step_id),
            ],
        ))
    }
}

/// Serialize the `rendering_method` field shared between `SURFACE_STYLE_RENDERING`
/// and `SURFACE_STYLE_RENDERING_WITH_PROPERTIES`. `None` round-trips as `$`
/// to preserve Fusion 360's unset form.
pub(crate) fn shading_method_attr(method: Option<ShadingMethod>) -> Attribute {
    match method {
        None => Attribute::Unset,
        Some(ShadingMethod::Constant) => Attribute::Enum("CONSTANT_SHADING".into()),
        Some(ShadingMethod::Colour) => Attribute::Enum("COLOUR_SHADING".into()),
        Some(ShadingMethod::Dot) => Attribute::Enum("DOT_SHADING".into()),
        Some(ShadingMethod::Normal) => Attribute::Enum("NORMAL_SHADING".into()),
    }
}
