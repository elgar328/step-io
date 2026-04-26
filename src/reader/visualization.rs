//! Visualization entity converters (Pass 7).
//!
//! Sub-passes process the `STYLED_ITEM` chain in dependency order:
//! `COLOUR_RGB` → `FILL_AREA_STYLE_COLOUR` → `FILL_AREA_STYLE` →
//! (`SURFACE_STYLE_FILL_AREA` ‖ `SURFACE_STYLE_TRANSPARENT` →
//! `SURFACE_STYLE_RENDERING_WITH_PROPERTIES`) → `SURFACE_SIDE_STYLE` →
//! `SURFACE_STYLE_USAGE` → `PRESENTATION_STYLE_ASSIGNMENT` →
//! `STYLED_ITEM` → `MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION`.
//!
//! Each pass populates a temp map (`viz_*_map`) keyed by STEP entity id.
//! Down-chain converts clone the cached struct so the final IR is a
//! tree-inline representation — color sharing in the source file is lost,
//! preserved as a transitional design (see `ir::visualization` doc).

use super::ReaderContext;
use crate::ir::attr::{
    check_count, read_entity_ref, read_entity_ref_list, read_enum, read_real, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    ColorRgb, FillAreaStyle, FillAreaStyleColour, Mdgpr, PresentationStyleAssignment,
    RenderingProperty, ShadingMethod, StyledItem, StyledItemTarget, SurfaceSide, SurfaceSideStyle,
    SurfaceSideStyleEntry, SurfaceStyleFillArea, SurfaceStyleRendering, SurfaceStyleUsage,
    VisualizationPool,
};
use crate::parser::entity::Attribute;

impl ReaderContext {
    // ------------------------------------------------------------------
    // Pass 7-1: COLOUR_RGB
    // ------------------------------------------------------------------

    pub(super) fn convert_colour_rgb(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "COLOUR_RGB")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let red = read_real(attrs, 1, entity_id, "red")?;
        let green = read_real(attrs, 2, entity_id, "green")?;
        let blue = read_real(attrs, 3, entity_id, "blue")?;
        self.viz_colour_rgb_map.insert(
            entity_id,
            ColorRgb {
                name,
                red,
                green,
                blue,
            },
        );
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 7-2: FILL_AREA_STYLE_COLOUR
    // ------------------------------------------------------------------

    pub(super) fn convert_fill_area_style_colour(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "FILL_AREA_STYLE_COLOUR")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let colour_ref = read_entity_ref(attrs, 1, entity_id, "fill_colour")?;
        let Some(colour) = self.viz_colour_rgb_map.get(&colour_ref).cloned() else {
            return Ok(()); // symmetric ignorance — unknown ref skipped
        };
        self.viz_fasc_map
            .insert(entity_id, FillAreaStyleColour { name, colour });
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 7-3: FILL_AREA_STYLE
    // ------------------------------------------------------------------

    pub(super) fn convert_fill_area_style(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "FILL_AREA_STYLE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "fill_styles")?;
        let mut fill_styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(fasc) = self.viz_fasc_map.get(&r).cloned() {
                fill_styles.push(fasc);
            }
        }
        self.viz_fas_map
            .insert(entity_id, FillAreaStyle { name, fill_styles });
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 7-4a: SURFACE_STYLE_FILL_AREA
    // ------------------------------------------------------------------

    pub(super) fn convert_surface_style_fill_area(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "SURFACE_STYLE_FILL_AREA")?;
        let fas_ref = read_entity_ref(attrs, 0, entity_id, "fill_area")?;
        let Some(fill_area) = self.viz_fas_map.get(&fas_ref).cloned() else {
            return Ok(());
        };
        self.viz_sss_entry_map.insert(
            entity_id,
            SurfaceSideStyleEntry::FillArea(SurfaceStyleFillArea { fill_area }),
        );
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 7-4b: SURFACE_STYLE_TRANSPARENT — leaf, populates a temp map
    // consumed by `convert_surface_style_rendering_with_properties`.
    // ------------------------------------------------------------------

    pub(super) fn convert_surface_style_transparent(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "SURFACE_STYLE_TRANSPARENT")?;
        let transparency = read_real(attrs, 0, entity_id, "transparency")?;
        self.viz_transparent_map.insert(entity_id, transparency);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 7-4c: SURFACE_STYLE_RENDERING_WITH_PROPERTIES
    // ------------------------------------------------------------------

    pub(super) fn convert_surface_style_rendering_with_properties(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            3,
            entity_id,
            "SURFACE_STYLE_RENDERING_WITH_PROPERTIES",
        )?;
        // The schema declares `rendering_method` as a non-optional enum, but
        // Fusion 360 routinely emits `$`. Treat Unset (or any other shape) as
        // `None` and a real enum value as `Some(...)` so the writer reproduces
        // whichever form the source file used.
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
        let Some(surface_colour) = self.viz_colour_rgb_map.get(&colour_ref).cloned() else {
            return Ok(());
        };
        let prop_refs = read_entity_ref_list(attrs, 2, entity_id, "properties")?;
        let mut properties = Vec::with_capacity(prop_refs.len());
        for r in prop_refs {
            // Only SURFACE_STYLE_TRANSPARENT is in scope — other property
            // entities (SURFACE_STYLE_REFLECTANCE_AMBIENT etc.) are silently
            // dropped (symmetric ignorance preserves round-trip equality).
            if let Some(&t) = self.viz_transparent_map.get(&r) {
                properties.push(RenderingProperty::Transparent(t));
            }
        }
        self.viz_sss_entry_map.insert(
            entity_id,
            SurfaceSideStyleEntry::Rendering(SurfaceStyleRendering {
                rendering_method,
                surface_colour,
                properties,
            }),
        );
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 7-5: SURFACE_SIDE_STYLE
    // ------------------------------------------------------------------

    pub(super) fn convert_surface_side_style(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SURFACE_SIDE_STYLE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(entry) = self.viz_sss_entry_map.get(&r).cloned() {
                styles.push(entry);
            }
        }
        self.viz_sss_map
            .insert(entity_id, SurfaceSideStyle { name, styles });
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 7-6: SURFACE_STYLE_USAGE
    // ------------------------------------------------------------------

    pub(super) fn convert_surface_style_usage(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SURFACE_STYLE_USAGE")?;
        let side_str = read_enum(attrs, 0, entity_id, "side")?;
        let side = match side_str {
            "POSITIVE" => SurfaceSide::Front,
            "NEGATIVE" => SurfaceSide::Back,
            _ => SurfaceSide::Both, // BOTH or unknown
        };
        let style_ref = read_entity_ref(attrs, 1, entity_id, "style")?;
        let Some(style) = self.viz_sss_map.get(&style_ref).cloned() else {
            return Ok(());
        };
        self.viz_ssu_map
            .insert(entity_id, SurfaceStyleUsage { side, style });
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 7-7: PRESENTATION_STYLE_ASSIGNMENT
    // ------------------------------------------------------------------

    pub(super) fn convert_presentation_style_assignment(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "PRESENTATION_STYLE_ASSIGNMENT")?;
        let style_refs = read_entity_ref_list(attrs, 0, entity_id, "styles")?;
        let mut styles = Vec::new();
        for r in style_refs {
            // Only SurfaceStyleUsage is in scope — CURVE_STYLE / POINT_STYLE /
            // SURFACE_STYLE_RENDERING_WITH_PROPERTIES are silently dropped
            // (symmetric ignorance preserves round-trip equality).
            if let Some(ssu) = self.viz_ssu_map.get(&r).cloned() {
                styles.push(ssu);
            }
        }
        self.viz_psa_map
            .insert(entity_id, PresentationStyleAssignment { styles });
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 7-8: STYLED_ITEM
    // ------------------------------------------------------------------

    pub(super) fn convert_styled_item(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "STYLED_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(psa) = self.viz_psa_map.get(&r).cloned() {
                styles.push(psa);
            }
        }

        // Resolve item ref to a geometry IR object id. Fusion 360 styles
        // individual ADVANCED_FACE entities; CATIA + FreeCAD typically
        // style solids / wires / loose points. Unresolved targets (Surface,
        // Edge, etc. — not currently in StyledItemTarget) are dropped at
        // read time so the writer's symmetric drop produces identical IR
        // on re-read (round-trip equality).
        let item = if let Some(&sid) = self.solid_map.get(&item_ref) {
            StyledItemTarget::Solid(sid)
        } else if let Some(&fid) = self.face_map.get(&item_ref) {
            StyledItemTarget::Face(fid)
        } else if let Some(&cid) = self.curve_map.get(&item_ref) {
            StyledItemTarget::Curve(cid)
        } else if let Some(&pid) = self.point_map.get(&item_ref) {
            StyledItemTarget::Point(pid)
        } else {
            return Ok(());
        };

        self.viz_styled_item_map
            .insert(entity_id, StyledItem { name, styles, item });
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 7-9: MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION
    // ------------------------------------------------------------------

    pub(super) fn convert_mdgpr(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            3,
            entity_id,
            "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION",
        )?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        // attrs[2] = context_of_items — Commit 2 will resolve this to a
        // `UnitContextId` via `context_id_map`. For Commit 1 we leave
        // `context` as `None`; the writer falls back to the first arena entry.

        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(si) = self.viz_styled_item_map.get(&r).cloned() {
                items.push(si);
            }
        }

        self.visualization
            .get_or_insert_with(VisualizationPool::default)
            .mdgprs
            .push(Mdgpr {
                name,
                items,
                context: None,
            });
        Ok(())
    }
}
