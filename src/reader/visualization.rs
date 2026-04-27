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
    check_count, read_entity_ref, read_entity_ref_list, read_enum, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    Mdgpr, PresentationStyleAssignment, StyledItem, StyledItemTarget, SurfaceSide,
    SurfaceSideStyle, SurfaceStyleUsage, VisualizationPool,
};
use crate::parser::entity::Attribute;

impl ReaderContext {
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
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        let context = self.context_id_map.get(&ctx_ref).copied();

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
                context,
            });
        Ok(())
    }
}
