//! `SURFACE_STYLE_RENDERING` handler (2-layer path) + the shared non-standard
//! pre-bind normalization for the rendering cluster.
//!
//! Both `rendering_method` and `surface_colour` are EXPRESS-required but some
//! CAD writers emit `$`. The generated bind is strict, so the handler normalizes
//! the input *before* binding ([`normalize_ssr_attrs`]): an Unset/unrecognized
//! `rendering_method` enum is rewritten to `NORMAL_SHADING`, and an Unset
//! `surface_colour` is backed by a reader-injected bare `COLOUR()` placeholder
//! referenced through a collision-free synthetic id — so L1 stays schema-faithful
//! (a real, resolvable ref; no Option, no sentinel value). Both deviations are
//! NORM-recorded. `SURFACE_STYLE_RENDERING` (this `Itself` form) is not observed
//! in the corpus; the handler exists for blueprint round-trip.

use std::borrow::Cow;

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{Colour, SurfaceStyleRenderingData, VisualizationPool};
use crate::parser::entity::Attribute;
use crate::reader::{NsCase, ReaderContext};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// The four EXPRESS `shading_surface_method` tokens.
fn is_known_shading(token: &str) -> bool {
    matches!(
        token,
        "CONSTANT_SHADING" | "COLOUR_SHADING" | "DOT_SHADING" | "NORMAL_SHADING"
    )
}

/// Pre-bind normalization shared by `SURFACE_STYLE_RENDERING` and its
/// `_WITH_PROPERTIES` subtype. Rewrites non-standard required slots to a
/// schema-valid form before the strict generated bind, recording each as a NORM:
/// - `rendering_method` (slot 0): Unset / unrecognized enum → `NORMAL_SHADING`.
/// - `surface_colour` (slot 1): Unset → a reader-injected bare `COLOUR()`
///   placeholder (`Colour::Itself`) addressed by a collision-free synthetic id,
///   so the bound L1 ref always resolves.
pub(crate) fn normalize_ssr_attrs<'a>(
    ctx: &mut ReaderContext,
    attrs: &'a [Attribute],
    type_name: &'static str,
) -> Cow<'a, [Attribute]> {
    let mut out = Cow::Borrowed(attrs);

    let method_ok = matches!(attrs.first(), Some(Attribute::Enum(s)) if is_known_shading(s));
    if !method_ok {
        let detail = if matches!(attrs.first(), Some(Attribute::Enum(_))) {
            format!("{type_name}.rendering_method (unrecognized value)")
        } else {
            format!("{type_name}.rendering_method (Unset)")
        };
        ctx.ns_record(
            NsCase::SurfaceStyleRenderingMethod,
            detail,
            "NORMAL_SHADING",
        );
        out.to_mut()[0] = Attribute::Enum("NORMAL_SHADING".into());
    }

    if attrs.get(1) == Some(&Attribute::Unset) {
        ctx.ns_record(
            NsCase::SurfaceStyleSurfaceColour,
            format!("{type_name}.surface_colour"),
            "COLOUR() (unspecified colour)",
        );
        let cid = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default)
            .colours
            .push(Colour::Itself);
        let syn = ctx.alloc_synthetic_entity_id();
        ctx.id_cache.insert(syn, cid);
        out.to_mut()[1] = Attribute::EntityRef(syn);
    }

    out
}

pub(crate) struct SurfaceStyleRenderingHandler;

#[step_entity(name = "SURFACE_STYLE_RENDERING")]
impl SimpleEntityHandler for SurfaceStyleRenderingHandler {
    type WriteInput = SurfaceStyleRenderingData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let attrs = normalize_ssr_attrs(ctx, attrs, "SURFACE_STYLE_RENDERING");
        let early = bind::bind_surface_style_rendering(entity_id, &attrs)?;
        lower::lower_surface_style_rendering(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: SurfaceStyleRenderingData) -> Result<u64, WriteError> {
        let colour_step = buf.step_id(data.surface_colour);
        let early = lift::lift_surface_style_rendering(data.rendering_method, colour_step);
        Ok(serialize::serialize_surface_style_rendering(buf, &early))
    }
}
