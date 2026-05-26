//! Visualization emission entry point. Plan 7 stage C2~C4 lifted every
//! emit body into `entities/visualization/<name>.rs` (the per-entity
//! handler chain). This file remains as a single dispatcher so
//! `emit_all` keeps a stable entry — analogous to the `emit_unit_context`
//! / `emit_face` wrappers in units / topology.

use super::WriteBuffer;
use crate::ir::representation_item::RepresentationItemRef;
use crate::ir::visualization::{
    Colour, FoundedItem, PreDefinedCurveFont, PreDefinedSymbol, PresentationStyleAssignment,
    StyledItem, SurfaceStyleRendering, TextStyle,
};
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    /// Emit the entity behind a [`RepresentationItemRef`] and return its STEP
    /// id. Geometry / topology / placement variants delegate to the existing
    /// idempotent emitters; `Representation` resolves through the
    /// `representation_step_ids` cache, which `emit_representations_pre_pass`
    /// fills for every geometry representation before the visualization pass
    /// runs. The resolver's MDGPR guard guarantees `id` is never an MDGPR
    /// (whose cache slot is appended only later, during this pass).
    pub(crate) fn emit_representation_item_ref(
        &mut self,
        item: RepresentationItemRef,
    ) -> Result<u64, WriteError> {
        match item {
            RepresentationItemRef::Solid(id) => self.emit_solid(id),
            RepresentationItemRef::Face(id) => self.emit_face(id),
            RepresentationItemRef::Edge(id) => self.emit_edge(id),
            RepresentationItemRef::Curve(id) => self.emit_curve(id),
            RepresentationItemRef::Point(id) => self.emit_point(id),
            RepresentationItemRef::Surface(id) => self.emit_surface(id),
            RepresentationItemRef::Vertex(id) => self.emit_vertex(id),
            RepresentationItemRef::Shell(id) => self.emit_shell(id),
            RepresentationItemRef::Placement3d(id) => self.emit_axis2_placement_3d(id),
            RepresentationItemRef::Representation(id) => {
                Ok(self.representation_step_ids[id.0 as usize])
            }
            RepresentationItemRef::RepresentationItem(id) => {
                Ok(self.representation_item_step_ids[id.0 as usize])
            }
        }
    }

    /// Emit the `characterized_object` arena (phase
    /// characterized-object-ciwr). Only `CharacterizedItemWithinRepresentation`
    /// variants are emitted in this phase; `Itself` (complex MI) is
    /// skipped — handled by a future sub-phase.
    pub(in crate::writer::buffer) fn emit_characterized_objects(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::characterized_item_within_representation::CharacterizedItemWithinRepresentationHandler;
        use crate::ir::shape_rep::CharacterizedObject;
        let items: Vec<_> = self.model.characterized_objects.iter().cloned().collect();
        for obj in items {
            match obj {
                CharacterizedObject::CharacterizedItemWithinRepresentation(ciwr) => {
                    let _ = CharacterizedItemWithinRepresentationHandler::write(self, ciwr);
                }
                CharacterizedObject::Itself(_) => {
                    // complex-MI Itself variant — future sub-phase
                }
            }
        }
    }

    /// Emit the `representation_item` arena (phase repr-item-arena-1).
    /// Fills `representation_item_step_ids` so other entities referencing
    /// QRI / VRI can resolve through `emit_representation_item_ref`.
    pub(in crate::writer::buffer) fn emit_representation_items(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::qualified_representation_item::QualifiedRepresentationItemHandler;
        use crate::entities::shape_rep::value_representation_item::ValueRepresentationItemHandler;
        use crate::ir::representation_item::RepresentationItem;
        let items: Vec<_> = self.model.representation_items.iter().cloned().collect();
        for item in items {
            let step = match item {
                RepresentationItem::QualifiedRepresentationItem(qri) => {
                    QualifiedRepresentationItemHandler::write(self, qri)
                }
                RepresentationItem::ValueRepresentationItem(vri) => {
                    ValueRepresentationItemHandler::write(self, vri)
                }
            };
            self.representation_item_step_ids.push(step.unwrap_or(0));
        }
    }

    fn emit_pre_defined_curve_fonts(
        &mut self,
        viz: &crate::ir::visualization::VisualizationPool,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::visualization::draughting_pre_defined_curve_font::DraughtingPreDefinedCurveFontHandler;
        use crate::entities::visualization::pre_defined_curve_font::PreDefinedCurveFontHandler;
        self.pre_defined_curve_font_step_ids =
            Vec::with_capacity(viz.pre_defined_curve_fonts.len());
        for font in viz.pre_defined_curve_fonts.iter() {
            let id = match font {
                PreDefinedCurveFont::Plain(f) => {
                    PreDefinedCurveFontHandler::write(self, f.clone())?
                }
                PreDefinedCurveFont::Draughting(f) => {
                    DraughtingPreDefinedCurveFontHandler::write(self, f.clone())?
                }
            };
            self.pre_defined_curve_font_step_ids.push(id);
        }
        Ok(())
    }

    fn emit_pre_defined_markers(
        &mut self,
        viz: &crate::ir::visualization::VisualizationPool,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::visualization::pre_defined_marker::PreDefinedMarkerHandler;
        self.pre_defined_marker_step_ids = Vec::with_capacity(viz.pre_defined_markers.len());
        for m in viz.pre_defined_markers.iter() {
            let id = PreDefinedMarkerHandler::write(self, m.clone())?;
            self.pre_defined_marker_step_ids.push(id);
        }
        Ok(())
    }

    fn emit_pre_defined_symbols(
        &mut self,
        viz: &crate::ir::visualization::VisualizationPool,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::visualization::pre_defined_symbol::PreDefinedSymbolHandler;
        use crate::entities::visualization::pre_defined_terminator_symbol::PreDefinedTerminatorSymbolHandler;
        self.pre_defined_symbol_step_ids = Vec::with_capacity(viz.pre_defined_symbols.len());
        for sym in viz.pre_defined_symbols.iter() {
            let id = match sym {
                PreDefinedSymbol::Plain(s) => PreDefinedSymbolHandler::write(self, s.clone())?,
                PreDefinedSymbol::Terminator(s) => {
                    PreDefinedTerminatorSymbolHandler::write(self, s.clone())?
                }
            };
            self.pre_defined_symbol_step_ids.push(id);
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub(in crate::writer::buffer) fn emit_visualization_if_set(
        &mut self,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::mdgpr::MdgprHandler;
        use crate::entities::visualization::camera_model_d3::CameraModelD3Handler;
        use crate::entities::visualization::colour_rgb::ColourRgbHandler;
        use crate::entities::visualization::context_dependent_over_riding_styled_item::ContextDependentOverRidingStyledItemHandler;
        use crate::entities::visualization::curve_style::CurveStyleHandler;
        use crate::entities::visualization::draughting_pre_defined_colour::DraughtingPreDefinedColourHandler;
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
        // SYMBOL_COLOUR — after colour cache, before symbol_style (future
        // phase) emits.
        self.symbol_colour_step_ids = Vec::with_capacity(viz.symbol_colours.len());
        for sc in viz.symbol_colours.iter() {
            use crate::entities::visualization::symbol_colour::SymbolColourHandler;
            let id = SymbolColourHandler::write(self, sc.clone())?;
            self.symbol_colour_step_ids.push(id);
        }
        // TEXT_STYLE_FOR_DEFINED_FONT — same timing.
        self.text_style_for_defined_font_step_ids =
            Vec::with_capacity(viz.text_styles_for_defined_font.len());
        for t in viz.text_styles_for_defined_font.iter() {
            use crate::entities::visualization::text_style_for_defined_font::TextStyleForDefinedFontHandler;
            let id = TextStyleForDefinedFontHandler::write(self, t.clone())?;
            self.text_style_for_defined_font_step_ids.push(id);
        }
        // TEXT_STYLE / TEXT_STYLE_WITH_BOX_CHARACTERISTICS — depends on
        // text_style_for_defined_font_step_ids for `character_appearance`.
        self.text_style_step_ids = Vec::with_capacity(viz.text_styles.len());
        for ts in viz.text_styles.iter() {
            use crate::entities::visualization::text_style_with_box_characteristics::TextStyleWithBoxCharacteristicsHandler;
            let step = match ts {
                TextStyle::WithBoxCharacteristics(t) => {
                    TextStyleWithBoxCharacteristicsHandler::write(self, t.clone())?
                }
                // `Itself` variant is corpus 0 (abstract supertype) — never
                // produced by the reader. Unreachable in practice.
                TextStyle::Itself(_) => 0,
            };
            self.text_style_step_ids.push(step);
        }
        self.emit_pre_defined_curve_fonts(&viz)?;
        self.emit_pre_defined_symbols(&viz)?;
        self.emit_pre_defined_markers(&viz)?;
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
        // CAMERA_MODEL_D3 — after emit_founded_item_arena so
        // `perspective_of_volume` resolves through `founded_item_step_ids`.
        for cm in viz.camera_models.iter() {
            match cm {
                crate::ir::visualization::CameraModel::CameraModelD3(d3) => {
                    CameraModelD3Handler::write(self, d3.clone())?;
                }
            }
        }
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
        use crate::entities::visualization::view_volume::ViewVolumeHandler;
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
        for (idx, item) in founded_items.iter().enumerate() {
            if let FoundedItem::ViewVolume(vv) = item {
                self.founded_item_step_ids[idx] = ViewVolumeHandler::write(self, vv.clone())?;
            }
        }
        for (idx, item) in founded_items.iter().enumerate() {
            if let FoundedItem::SymbolStyle(ss) = item {
                use crate::entities::visualization::symbol_style::SymbolStyleHandler;
                self.founded_item_step_ids[idx] = SymbolStyleHandler::write(self, ss.clone())?;
            }
        }
        for (idx, item) in founded_items.iter().enumerate() {
            if let FoundedItem::PointStyle(ps) = item {
                use crate::entities::visualization::point_style::PointStyleHandler;
                self.founded_item_step_ids[idx] = PointStyleHandler::write(self, ps.clone())?;
            }
        }
        Ok(())
    }
}
