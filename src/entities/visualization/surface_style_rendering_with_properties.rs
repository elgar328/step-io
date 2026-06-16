//! `SURFACE_STYLE_RENDERING_WITH_PROPERTIES` handler (2-layer path).
//!
//! Combines a colour reference with a rendering-method enum and
//! `SURFACE_STYLE_TRANSPARENT` property refs. Non-standard `rendering_method` /
//! `surface_colour` are normalized before binding by the shared
//! [`normalize_ssr_attrs`](super::surface_style_rendering::normalize_ssr_attrs)
//! pre-bind step. Other property entities (`REFLECTANCE_AMBIENT` etc.) are not
//! modelled and silently dropped to preserve round-trip on the supported subset.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{RenderingProperty, SurfaceStyleRenderingWithProperties};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::surface_style_rendering::normalize_ssr_attrs;
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
        let attrs = normalize_ssr_attrs(ctx, attrs, "SURFACE_STYLE_RENDERING_WITH_PROPERTIES");
        let early = bind::bind_surface_style_rendering_with_properties(entity_id, &attrs)?;
        lower::lower_surface_style_rendering_with_properties(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ssr: SurfaceStyleRenderingWithProperties,
    ) -> Result<u64, WriteError> {
        let colour_step = buf.step_id(ssr.surface_colour);
        let mut prop_refs = Vec::with_capacity(ssr.properties.len());
        for prop in ssr.properties {
            let r = match prop {
                RenderingProperty::Transparent(t) => SurfaceStyleTransparentHandler::write(buf, t)?,
            };
            prop_refs.push(r);
        }
        let early = lift::lift_surface_style_rendering_with_properties(
            ssr.rendering_method,
            colour_step,
            prop_refs,
        );
        Ok(serialize::serialize_surface_style_rendering_with_properties(buf, &early))
    }
}
