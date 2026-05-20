//! Visualization emission entry point. Plan 7 stage C2~C4 lifted every
//! emit body into `entities/visualization/<name>.rs` (the per-entity
//! handler chain). This file remains as a single dispatcher so
//! `emit_all` keeps a stable entry — analogous to the `emit_unit_context`
//! / `emit_face` wrappers in units / topology.

use super::WriteBuffer;
use crate::ir::visualization::{
    Colour, CurveFont, FoundedItem, PresentationStyleAssignment, StyledItem, SurfaceStyleRendering,
};
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_visualization_if_set(
        &mut self,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::mdgpr::MdgprHandler;
        use crate::entities::visualization::colour_rgb::ColourRgbHandler;
        use crate::entities::visualization::context_dependent_over_riding_styled_item::ContextDependentOverRidingStyledItemHandler;
        use crate::entities::visualization::curve_style::CurveStyleHandler;
        use crate::entities::visualization::draughting_pre_defined_colour::DraughtingPreDefinedColourHandler;
        use crate::entities::visualization::draughting_pre_defined_curve_font::DraughtingPreDefinedCurveFontHandler;
        use crate::entities::visualization::over_riding_styled_item::OverRidingStyledItemHandler;
        use crate::entities::visualization::presentation_layer_assignment::PresentationLayerAssignmentHandler;
        use crate::entities::visualization::presentation_style_assignment::PresentationStyleAssignmentHandler;
        use crate::entities::visualization::styled_item::StyledItemHandler;
        use crate::entities::visualization::surface_style_rendering::SurfaceStyleRenderingHandler;
        use crate::entities::visualization::surface_style_rendering_with_properties::SurfaceStyleRenderingWithPropertiesHandler;
        let Some(viz) = self.model.visualization.clone() else {
            return Ok(());
        };
        // Emit order: colours -> curve_fonts -> curve_styles -> mdgprs.
        // Front-loading the leaf arenas populates the *_step_ids caches so
        // every downstream consumer (FILL_AREA_STYLE_COLOUR, SSRWP,
        // CURVE_STYLE, PSA) can resolve an arena reference to a cached
        // STEP id with one index lookup.
        self.colour_step_ids = Vec::with_capacity(viz.colours.len());
        for colour in viz.colours.iter() {
            let id = match colour {
                Colour::Rgb(c) => ColourRgbHandler::write(self, c.clone())?,
                Colour::PreDefined(c) => DraughtingPreDefinedColourHandler::write(self, c.clone())?,
            };
            self.colour_step_ids.push(id);
        }
        self.curve_font_step_ids = Vec::with_capacity(viz.curve_fonts.len());
        for font in viz.curve_fonts.iter() {
            let id = match font {
                CurveFont::PreDefined(f) => {
                    DraughtingPreDefinedCurveFontHandler::write(self, f.clone())?
                }
            };
            self.curve_font_step_ids.push(id);
        }
        self.curve_style_step_ids = Vec::with_capacity(viz.curve_styles.len());
        for cs in viz.curve_styles.iter() {
            let id = CurveStyleHandler::write(self, cs.clone())?;
            self.curve_style_step_ids.push(id);
        }
        // SURFACE_STYLE_RENDERING arena — emit every entry up-front so the
        // downstream SURFACE_SIDE_STYLE writer (invoked transitively from
        // each PSA's SSU body) resolves SurfaceSideStyleEntry::Rendering
        // through ssr_step_ids[id.0]. Pre-emit runs before the PSA cache
        // population so the SSU/SSS chain inside each PSA can hit the
        // cache.
        self.ssr_step_ids = Vec::with_capacity(viz.surface_style_renderings.len());
        for ssr in viz.surface_style_renderings.iter() {
            let id = match ssr {
                SurfaceStyleRendering::Itself(data) => {
                    SurfaceStyleRenderingHandler::write(self, data.clone())?
                }
                SurfaceStyleRendering::SurfaceStyleRenderingWithProperties(data) => {
                    SurfaceStyleRenderingWithPropertiesHandler::write(self, data.clone())?
                }
            };
            self.ssr_step_ids.push(id);
        }
        self.emit_founded_item_arena(&viz.founded_items)?;
        // PRESENTATION_STYLE_ASSIGNMENT arena — emit every PSA up-front so
        // STYLED_ITEM / OVER_RIDING_STYLED_ITEM writers can resolve their
        // styles refs through psa_step_ids[id.0]. `ByContext` variant is
        // never produced by the current reader (handler unregistered
        // pending Representation IR phase); placeholder 0 keeps the
        // indexing aligned should one ever appear from a kernel adapter.
        self.psa_step_ids = Vec::with_capacity(viz.presentation_style_assignments.len());
        for psa in viz.presentation_style_assignments.iter() {
            let id = match psa {
                PresentationStyleAssignment::Itself(data) => {
                    PresentationStyleAssignmentHandler::write(self, data.clone())?
                }
                PresentationStyleAssignment::PresentationStyleByContext(_) => 0,
            };
            self.psa_step_ids.push(id);
        }
        // STYLED_ITEM arena — emit Plain entries first so their STEP ids
        // are cached when OverRiding entries reference them through
        // `over_ridden_style`. The vec is pre-sized to viz.styled_items.len()
        // and each pass writes into its variant's slot; downstream
        // consumers (MDGPR, PSA) read `styled_item_step_ids[id.0]`.
        self.styled_item_step_ids = vec![0; viz.styled_items.len()];
        for (idx, si) in viz.styled_items.iter().enumerate() {
            if let StyledItem::Plain(p) = si {
                let id = StyledItemHandler::write(self, p.clone())?;
                self.styled_item_step_ids[idx] = id;
            }
        }
        for (idx, si) in viz.styled_items.iter().enumerate() {
            if let StyledItem::OverRiding(o) = si {
                let id = OverRidingStyledItemHandler::write(self, o.clone())?;
                self.styled_item_step_ids[idx] = id;
            }
        }
        for (idx, si) in viz.styled_items.iter().enumerate() {
            if let StyledItem::ContextDependent(cd) = si {
                let id = ContextDependentOverRidingStyledItemHandler::write(self, cd.clone())?;
                self.styled_item_step_ids[idx] = id;
            }
        }
        // MDGPR is the trailing segment of the `representations` arena
        // (Pass 7, after every geometry representation). `viz.mdgprs` is in
        // the same order, so appending each step id keeps
        // `representation_step_ids` aligned with `RepresentationId`.
        for mdgpr in viz.mdgprs {
            let step_id = MdgprHandler::write(self, mdgpr)?;
            self.representation_step_ids.push(step_id);
        }
        for pla in viz.presentation_layer_assignments.iter() {
            PresentationLayerAssignmentHandler::write(self, pla.clone())?;
        }
        Ok(())
    }

    /// Pre-emit the `founded_item` arena variant-by-variant so each pass
    /// can resolve its predecessors through `founded_item_step_ids`.
    /// Order: `FillAreaStyle` -> `SurfaceStyleFillArea` -> `SurfaceSideStyle`
    /// -> `SurfaceStyleUsage`. arena iteration order already matches reader
    /// pass order, but the defensive variant split keeps this safe if a
    /// kernel adapter ever reorders pushes.
    fn emit_founded_item_arena(
        &mut self,
        founded_items: &crate::ir::Arena<FoundedItem>,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::visualization::fill_area_style::FillAreaStyleHandler;
        use crate::entities::visualization::surface_side_style::SurfaceSideStyleHandler;
        use crate::entities::visualization::surface_style_fill_area::SurfaceStyleFillAreaHandler;
        use crate::entities::visualization::surface_style_usage::SurfaceStyleUsageHandler;
        self.founded_item_step_ids = vec![0; founded_items.len()];
        for (idx, item) in founded_items.iter().enumerate() {
            if let FoundedItem::FillAreaStyle(fas) = item {
                self.founded_item_step_ids[idx] = FillAreaStyleHandler::write(self, fas.clone())?;
            }
        }
        for (idx, item) in founded_items.iter().enumerate() {
            if let FoundedItem::SurfaceStyleFillArea(ssfa) = item {
                self.founded_item_step_ids[idx] =
                    SurfaceStyleFillAreaHandler::write(self, ssfa.clone())?;
            }
        }
        for (idx, item) in founded_items.iter().enumerate() {
            if let FoundedItem::SurfaceSideStyle(sss) = item {
                self.founded_item_step_ids[idx] =
                    SurfaceSideStyleHandler::write(self, sss.clone())?;
            }
        }
        for (idx, item) in founded_items.iter().enumerate() {
            if let FoundedItem::SurfaceStyleUsage(ssu) = item {
                self.founded_item_step_ids[idx] =
                    SurfaceStyleUsageHandler::write(self, ssu.clone())?;
            }
        }
        Ok(())
    }
}
