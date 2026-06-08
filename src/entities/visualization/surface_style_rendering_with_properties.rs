//! `SURFACE_STYLE_RENDERING_WITH_PROPERTIES` handler.
//!
//! Combines a `COLOUR_RGB` reference with a rendering-method enum and
//! `SURFACE_STYLE_TRANSPARENT` property refs. The schema declares both
//! `rendering_method` and `surface_colour` as required, but some CAD writers
//! emit `$`; the reader normalizes those to standard defaults
//! (`NORMAL_SHADING`, matching OCCT / neutral grey) so the IR stays
//! schema-valid. Other property entities (`REFLECTANCE_AMBIENT` etc.) are
//! silently dropped to preserve round-trip equality on the supported subset.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    RenderingProperty, SurfaceStyleRendering, SurfaceStyleRenderingWithProperties,
    VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::surface_style_rendering::{
    read_rendering_method, read_surface_colour, shading_method_attr,
};
use super::surface_style_transparent::SurfaceStyleTransparentHandler;
use step_io_macros::step_entity;

pub(crate) struct SurfaceStyleRenderingWithPropertiesHandler;

#[step_entity(name = "SURFACE_STYLE_RENDERING_WITH_PROPERTIES")]
impl SimpleEntityHandler for SurfaceStyleRenderingWithPropertiesHandler {
    type WriteInput = SurfaceStyleRenderingWithProperties;

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
        let rendering_method = read_rendering_method(
            ctx,
            attrs,
            entity_id,
            "SURFACE_STYLE_RENDERING_WITH_PROPERTIES",
        )?;
        let Some(surface_colour) = read_surface_colour(
            ctx,
            attrs,
            entity_id,
            "SURFACE_STYLE_RENDERING_WITH_PROPERTIES",
        )?
        else {
            return Ok(());
        };
        let prop_refs = read_entity_ref_list(attrs, 2, entity_id, "properties")?;
        let mut properties = Vec::with_capacity(prop_refs.len());
        for r in prop_refs {
            if let Some(&t) = ctx.viz_transparent_map.get(&r) {
                properties.push(RenderingProperty::Transparent(t));
            }
        }
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool.surface_style_renderings.push(
            SurfaceStyleRendering::SurfaceStyleRenderingWithProperties(
                SurfaceStyleRenderingWithProperties {
                    rendering_method,
                    surface_colour,
                    properties,
                },
            ),
        );
        ctx.viz_ssr_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ssr: SurfaceStyleRenderingWithProperties,
    ) -> Result<u64, WriteError> {
        let colour_step_id = buf.step_id(ssr.surface_colour);
        let mut prop_refs = Vec::with_capacity(ssr.properties.len());
        for prop in ssr.properties {
            let prop_id = match prop {
                RenderingProperty::Transparent(t) => SurfaceStyleTransparentHandler::write(buf, t)?,
            };
            prop_refs.push(Attribute::EntityRef(prop_id));
        }
        Ok(buf.push_simple(
            "SURFACE_STYLE_RENDERING_WITH_PROPERTIES",
            vec![
                shading_method_attr(ssr.rendering_method),
                Attribute::EntityRef(colour_step_id),
                Attribute::List(prop_refs),
            ],
        ))
    }
}
