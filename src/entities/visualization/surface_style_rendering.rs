//! `SURFACE_STYLE_RENDERING` handler (Pass 7-6) — the `Itself` variant of
//! the `surface_style_rendering` arena. Same shape as the
//! `_WITH_PROPERTIES` subtype minus the `properties` list. Not currently
//! observed in the reference-check corpus; the handler exists to align
//! with the ir.toml blueprint so a future kernel adapter emitting plain
//! `SURFACE_STYLE_RENDERING` round-trips losslessly.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_enum};
use crate::ir::error::ConvertError;
use crate::ir::id::ColourId;
use crate::ir::visualization::{
    Colour, ColourRgb, ShadingMethod, SurfaceStyleRendering, SurfaceStyleRenderingData,
    VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SurfaceStyleRenderingHandler;

#[step_entity(name = "SURFACE_STYLE_RENDERING")]
impl SimpleEntityHandler for SurfaceStyleRenderingHandler {
    type WriteInput = SurfaceStyleRenderingData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SURFACE_STYLE_RENDERING")?;
        let rendering_method =
            read_rendering_method(ctx, attrs, entity_id, "SURFACE_STYLE_RENDERING")?;
        let Some(surface_colour) =
            read_surface_colour(ctx, attrs, entity_id, "SURFACE_STYLE_RENDERING")?
        else {
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

/// Read the required `rendering_method` field, shared between
/// `SURFACE_STYLE_RENDERING` and `SURFACE_STYLE_RENDERING_WITH_PROPERTIES`.
/// Non-standard `$` (Unset) and unrecognized enum values are normalized to
/// `NORMAL_SHADING`; each is recorded for the per-file summary warning.
pub(crate) fn read_rendering_method(
    ctx: &mut ReaderContext,
    attrs: &[Attribute],
    entity_id: u64,
    type_name: &'static str,
) -> Result<ShadingMethod, ConvertError> {
    if matches!(attrs.first(), Some(Attribute::Enum(_))) {
        Ok(match read_enum(attrs, 0, entity_id, "rendering_method")? {
            "CONSTANT_SHADING" => ShadingMethod::Constant,
            "COLOUR_SHADING" => ShadingMethod::Colour,
            "DOT_SHADING" => ShadingMethod::Dot,
            "NORMAL_SHADING" => ShadingMethod::Normal,
            _ => {
                ctx.record_nonstandard(
                    format!("{type_name}.rendering_method (unrecognized value)"),
                    "NORMAL_SHADING",
                );
                ShadingMethod::Normal
            }
        })
    } else {
        ctx.record_nonstandard(
            format!("{type_name}.rendering_method (Unset)"),
            "NORMAL_SHADING",
        );
        Ok(ShadingMethod::Normal)
    }
}

/// Read the required `surface_colour` field, shared between the two
/// rendering entities. A non-standard `$` (Unset) is normalized to a neutral
/// grey `COLOUR_RGB` pushed into the colour arena. `Ok(None)` means the ref
/// was present but unresolved — the caller drops the entity, as before.
pub(crate) fn read_surface_colour(
    ctx: &mut ReaderContext,
    attrs: &[Attribute],
    entity_id: u64,
    type_name: &'static str,
) -> Result<Option<ColourId>, ConvertError> {
    if attrs[1] == Attribute::Unset {
        ctx.record_nonstandard(
            format!("{type_name}.surface_colour"),
            "grey COLOUR_RGB(0.7,0.7,0.7)",
        );
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        Ok(Some(pool.colours.push(Colour::Rgb(ColourRgb {
            name: String::new(),
            red: 0.7,
            green: 0.7,
            blue: 0.7,
        }))))
    } else {
        let colour_ref = read_entity_ref(attrs, 1, entity_id, "surface_colour")?;
        Ok(ctx.viz_colour_id_map.get(&colour_ref).copied())
    }
}

/// Serialize the required `rendering_method` field, shared between
/// `SURFACE_STYLE_RENDERING` and `SURFACE_STYLE_RENDERING_WITH_PROPERTIES`.
pub(crate) fn shading_method_attr(method: ShadingMethod) -> Attribute {
    match method {
        ShadingMethod::Constant => Attribute::Enum("CONSTANT_SHADING".into()),
        ShadingMethod::Colour => Attribute::Enum("COLOUR_SHADING".into()),
        ShadingMethod::Dot => Attribute::Enum("DOT_SHADING".into()),
        ShadingMethod::Normal => Attribute::Enum("NORMAL_SHADING".into()),
    }
}
