//! `SURFACE_STYLE_RENDERING_WITH_PROPERTIES` handler — Pass 7-6.
//!
//! Combines a `COLOUR_RGB` reference with optional rendering-method enum
//! and `SURFACE_STYLE_TRANSPARENT` property refs. The schema declares
//! `rendering_method` as non-optional, but Fusion 360 emits `$` — accept
//! Unset as `None` so the writer round-trips whichever form the source
//! used. Other property entities (`REFLECTANCE_AMBIENT` etc.) are silently
//! dropped to preserve round-trip equality on supported subset.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_enum};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    RenderingProperty, ShadingMethod, SurfaceSideStyleEntry, SurfaceStyleRendering,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::colour_rgb::ColourRgbHandler;
use super::surface_style_transparent::SurfaceStyleTransparentHandler;
use step_io_macros::step_entity;

pub(crate) struct SurfaceStyleRenderingHandler;

#[step_entity(name = "SURFACE_STYLE_RENDERING_WITH_PROPERTIES", pass = Pass7Rendering)]
impl SimpleEntityHandler for SurfaceStyleRenderingHandler {
    type WriteInput = SurfaceStyleRendering;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            3,
            entity_id,
            "SURFACE_STYLE_RENDERING_WITH_PROPERTIES",
        )?;
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
        let Some(surface_colour) = ctx.viz_colour_rgb_map.get(&colour_ref).cloned() else {
            return Ok(());
        };
        let prop_refs = read_entity_ref_list(attrs, 2, entity_id, "properties")?;
        let mut properties = Vec::with_capacity(prop_refs.len());
        for r in prop_refs {
            if let Some(&t) = ctx.viz_transparent_map.get(&r) {
                properties.push(RenderingProperty::Transparent(t));
            }
        }
        ctx.viz_sss_entry_map.insert(
            entity_id,
            SurfaceSideStyleEntry::Rendering(SurfaceStyleRendering {
                rendering_method,
                surface_colour,
                properties,
            }),
        );
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ssr: SurfaceStyleRendering) -> Result<u64, WriteError> {
        let colour_id = ColourRgbHandler::write(buf, ssr.surface_colour)?;
        let mut prop_refs = Vec::with_capacity(ssr.properties.len());
        for prop in ssr.properties {
            let prop_id = match prop {
                RenderingProperty::Transparent(t) => SurfaceStyleTransparentHandler::write(buf, t)?,
            };
            prop_refs.push(Attribute::EntityRef(prop_id));
        }
        let method_attr = match ssr.rendering_method {
            None => Attribute::Unset,
            Some(ShadingMethod::Constant) => Attribute::Enum("CONSTANT_SHADING".into()),
            Some(ShadingMethod::Colour) => Attribute::Enum("COLOUR_SHADING".into()),
            Some(ShadingMethod::Dot) => Attribute::Enum("DOT_SHADING".into()),
            Some(ShadingMethod::Normal) => Attribute::Enum("NORMAL_SHADING".into()),
        };
        Ok(buf.push_simple(
            "SURFACE_STYLE_RENDERING_WITH_PROPERTIES",
            vec![
                method_attr,
                Attribute::EntityRef(colour_id),
                Attribute::List(prop_refs),
            ],
        ))
    }
}
