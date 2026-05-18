//! Visualization emission entry point. Plan 7 stage C2~C4 lifted every
//! emit body into `entities/visualization/<name>.rs` (the per-entity
//! handler chain). This file remains as a single dispatcher so
//! `emit_all` keeps a stable entry — analogous to the `emit_unit_context`
//! / `emit_face` wrappers in units / topology.

use super::WriteBuffer;
use crate::ir::visualization::{Colour, CurveFont, StyledItem};
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_visualization_if_set(
        &mut self,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::mdgpr::MdgprHandler;
        use crate::entities::visualization::colour_rgb::ColourRgbHandler;
        use crate::entities::visualization::curve_style::CurveStyleHandler;
        use crate::entities::visualization::draughting_pre_defined_colour::DraughtingPreDefinedColourHandler;
        use crate::entities::visualization::draughting_pre_defined_curve_font::DraughtingPreDefinedCurveFontHandler;
        use crate::entities::visualization::over_riding_styled_item::OverRidingStyledItemHandler;
        use crate::entities::visualization::styled_item::StyledItemHandler;
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
        for mdgpr in viz.mdgprs {
            MdgprHandler::write(self, mdgpr)?;
        }
        Ok(())
    }
}
