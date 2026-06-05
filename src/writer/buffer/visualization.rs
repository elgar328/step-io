//! Visualization emission entry point. Every emit body lives
//! in `entities/visualization/<name>.rs` (the per-entity
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
            RepresentationItemRef::Placement2d(id) => self.emit_axis2_placement_2d(id),
            RepresentationItemRef::PlanarExtent(id) => self.emit_planar_extent(id),
            RepresentationItemRef::Representation(id) => {
                Ok(self.representation_step_ids[id.0 as usize])
            }
            RepresentationItemRef::RepresentationItem(id) => {
                Ok(self.representation_item_step_ids[id.0 as usize])
            }
            RepresentationItemRef::GeometricRepresentationItem(id) => {
                Ok(self.geometric_representation_item_step_ids[id.0 as usize])
            }
            RepresentationItemRef::TessellatedItem(id) => {
                Ok(self.tessellated_item_step_ids[id.0 as usize])
            }
            RepresentationItemRef::TessellatedFace(id) => {
                Ok(self.tessellated_face_step_ids[id.0 as usize])
            }
            RepresentationItemRef::MappedItem(id) => Ok(self.mapped_item_step_ids[id.0 as usize]),
            RepresentationItemRef::AnnotationOccurrence(id) => Ok(self.ao_step_ids[id.0 as usize]),
            RepresentationItemRef::AnnotationCurveOccurrence(id) => {
                Ok(self.acoc_step_ids[id.0 as usize])
            }
            RepresentationItemRef::DraughtingCallout(id) => {
                Ok(self.draughting_callout_step_ids[id.0 as usize])
            }
            RepresentationItemRef::CameraModel(id) => {
                Ok(self.viz_camera_model_step_ids[id.0 as usize])
            }
            RepresentationItemRef::TextLiteral(id) => Ok(self.text_literal_step_ids[id.0 as usize]),
            RepresentationItemRef::CompositeText(id) => {
                Ok(self.composite_text_step_ids[id.0 as usize])
            }
            RepresentationItemRef::StyledItem(id) => Ok(self.styled_item_step_ids[id.0 as usize]),
        }
    }

    /// Reserve a STEP id for every standalone
    /// `CHARACTERIZED_ITEM_WITHIN_REPRESENTATION` (filling
    /// `characterized_object_step_ids`) before the PD-definition pass, so a
    /// `PROPERTY_DEFINITION` targeting a CIWR can emit the forward ref. The CO
    /// bodies emit later in `emit_characterized_objects` under these reserved
    /// ids. Inline-DM and non-CIWR COs keep slot 0 (a PD can only target a
    /// standalone CIWR — the reader's subtype gate).
    pub(in crate::writer::buffer) fn emit_characterized_objects_prepass(&mut self) {
        use crate::ir::shape_rep::CharacterizedObject;
        let inline = self.inline_characterized_object_ids();
        let n = self.model.characterized_objects.len();
        self.characterized_object_step_ids = vec![0; n];
        let ciwr_ids: Vec<_> = self
            .model
            .characterized_objects
            .iter_with_ids()
            .filter(|(id, obj)| {
                !inline.contains(id)
                    && matches!(
                        obj,
                        CharacterizedObject::CharacterizedItemWithinRepresentation(_)
                    )
            })
            .map(|(id, _)| id)
            .collect();
        for id in ciwr_ids {
            let step = self.fresh();
            self.characterized_object_step_ids[id.0 as usize] = step;
        }
        // Reserve a step id for each inline-CO DM (Characterized /
        // CharacterizedShapeTessellated) so a PROPERTY_DEFINITION targeting the
        // plain CHARACTERIZED_OBJECT can forward-ref the (shared) DM complex id;
        // `emit_draughting_models` emits the body later under this same id.
        // Iterate in arena order (NOT `inline` set order) so the reserved #N —
        // and therefore the round-tripped arena indices — are deterministic.
        let inline_ids: Vec<_> = self
            .model
            .characterized_objects
            .iter_with_ids()
            .filter(|(id, _)| inline.contains(id))
            .map(|(id, _)| id)
            .collect();
        for id in inline_ids {
            if self.characterized_object_step_ids[id.0 as usize] == 0 {
                let step = self.fresh();
                self.characterized_object_step_ids[id.0 as usize] = step;
            }
        }
    }

    /// `CharacterizedObject` ids carried inline by a `DraughtingModel`'s
    /// complex MI form (`(CO + CR + DM + REPR)`) — emitted inside the DM, not standalone.
    fn inline_characterized_object_ids(
        &self,
    ) -> std::collections::HashSet<crate::ir::CharacterizedObjectId> {
        use crate::ir::shape_rep::{DraughtingModelForm, Representation};
        self.model
            .representations
            .iter()
            .filter_map(|r| match r {
                Representation::DraughtingModel(dm) => match dm.form {
                    DraughtingModelForm::Characterized(id)
                    | DraughtingModelForm::CharacterizedShapeTessellated(id) => Some(id),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    pub(in crate::writer::buffer) fn emit_characterized_objects(&mut self) {
        use crate::ir::shape_rep::CharacterizedObject;
        use crate::parser::entity::Attribute;
        let inline_co_ids = self.inline_characterized_object_ids();
        let entries: Vec<_> = self
            .model
            .characterized_objects
            .iter_with_ids()
            .map(|(id, obj)| (id, obj.clone()))
            .collect();
        for (id, obj) in entries {
            if inline_co_ids.contains(&id) {
                continue;
            }
            match obj {
                CharacterizedObject::CharacterizedItemWithinRepresentation(ciwr) => {
                    // Emit under the reserved id (forward-ref by any PD that
                    // targeted this CIWR). item/rep step caches are now filled.
                    let reserved = self.characterized_object_step_ids[id.0 as usize];
                    let Ok(item_step) = self.emit_representation_item_ref(ciwr.item) else {
                        continue;
                    };
                    let rep_step = self.representation_step_ids[ciwr.rep.0 as usize];
                    let desc_attr = match ciwr.inherited.description {
                        Some(d) => Attribute::String(d),
                        None => Attribute::Unset,
                    };
                    self.push_simple_with_id(
                        reserved,
                        "CHARACTERIZED_ITEM_WITHIN_REPRESENTATION",
                        vec![
                            Attribute::String(ciwr.inherited.name),
                            desc_attr,
                            Attribute::EntityRef(item_step),
                            Attribute::EntityRef(rep_step),
                        ],
                    );
                }
                CharacterizedObject::Itself(data) => {
                    // Phase characterized-min: simple form
                    // `CHARACTERIZED_OBJECT(name, $)`. Original corpus
                    // complex MI parts (DM/TSR/SR/REPRESENTATION) are
                    // discarded (minimal scope).
                    use crate::entities::ComplexEntityHandler;
                    use crate::entities::shape_rep::characterized_object_complex::CharacterizedObjectComplexHandler;
                    let _ = CharacterizedObjectComplexHandler::write(self, data);
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
                RepresentationItem::MeasureRepresentationItem(mri) => {
                    Ok(self.emit_measure_repr_item(mri))
                }
            };
            self.representation_item_step_ids.push(step.unwrap_or(0));
        }
    }

    /// Emit a complex-MI `MEASURE_REPRESENTATION_ITEM` (phase measure-arena-1):
    /// `(<X>_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM()
    /// MEASURE_WITH_UNIT(<typed value>, #unit)
    /// [QUALIFIED_REPRESENTATION_ITEM((..))] REPRESENTATION_ITEM(name))`,
    /// reproducing the captured typed supertype, value, unit, and qualifiers.
    /// Qualifier / unit step caches are filled by `emit_pmi_if_set` / the unit
    /// emit, both of which run before `emit_representation_items`.
    fn emit_measure_repr_item(
        &mut self,
        mri: crate::ir::representation_item::MeasureRepresentationItem,
    ) -> u64 {
        use crate::ir::representation_item::{MeasureValue, QualifierRef};
        use crate::parser::entity::Attribute;
        use crate::writer::entity::{WriterBody, WriterEntity};
        let typed = match mri.value {
            MeasureValue::Real { type_name, value } => Attribute::Typed {
                type_name,
                value: Box::new(Attribute::Real(value)),
            },
            MeasureValue::Integer { type_name, value } => Attribute::Typed {
                type_name,
                value: Box::new(Attribute::Integer(value)),
            },
            MeasureValue::Text { type_name, value } => Attribute::Typed {
                type_name,
                value: Box::new(Attribute::String(value)),
            },
        };
        let unit_step = self.resolve_explicit_unit_ref(mri.unit_ref).unwrap_or(0);
        // Simple form: the bare 3-attr MEASURE_REPRESENTATION_ITEM line
        // (phase measure-arena-4).
        if matches!(
            mri.form,
            crate::ir::representation_item::MeasureForm::Simple
        ) {
            return self.push_simple(
                "MEASURE_REPRESENTATION_ITEM",
                vec![
                    Attribute::String(mri.name),
                    typed,
                    Attribute::EntityRef(unit_step),
                ],
            );
        }
        let mut parts: Vec<(String, Vec<Attribute>)> = Vec::with_capacity(5);
        if let Some(supertype) = mri.measure_supertype {
            parts.push((supertype, vec![]));
        }
        parts.push(("MEASURE_REPRESENTATION_ITEM".into(), vec![]));
        parts.push((
            "MEASURE_WITH_UNIT".into(),
            vec![typed, Attribute::EntityRef(unit_step)],
        ));
        if !mri.qualifiers.is_empty() {
            let q_refs: Vec<Attribute> = mri
                .qualifiers
                .iter()
                .map(|q| {
                    let step = match q {
                        QualifierRef::TypeQualifier(id) => {
                            self.type_qualifier_step_ids[id.0 as usize]
                        }
                        QualifierRef::ValueFormatTypeQualifier(id) => {
                            self.value_format_type_qualifier_step_ids[id.0 as usize]
                        }
                    };
                    Attribute::EntityRef(step)
                })
                .collect();
            parts.push((
                "QUALIFIED_REPRESENTATION_ITEM".into(),
                vec![Attribute::List(q_refs)],
            ));
        }
        parts.push((
            "REPRESENTATION_ITEM".into(),
            vec![Attribute::String(mri.name)],
        ));
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex { parts },
        });
        n
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
        use crate::entities::visualization::pre_defined_marker::{
            PreDefinedMarkerHandler, PreDefinedPointMarkerSymbolHandler,
        };
        use crate::ir::visualization::PreDefinedMarker;
        self.pre_defined_marker_step_ids = Vec::with_capacity(viz.pre_defined_markers.len());
        for m in viz.pre_defined_markers.iter() {
            let id = match m {
                PreDefinedMarker::Plain(d) => PreDefinedMarkerHandler::write(self, d.clone())?,
                PreDefinedMarker::PointMarkerSymbol(p) => {
                    PreDefinedPointMarkerSymbolHandler::write(self, p.clone())?
                }
            };
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
        use crate::entities::visualization::colour::ColourHandler;
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
                Colour::Itself => ColourHandler::write(self, ())?,
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
        // TEXT_LITERAL — depends on placement caches (emit-on-demand) and
        // dptf_step_ids (populated by `emit_pmi_if_set` which runs before
        // this method via `WriteBuffer::emit_pools`).
        self.text_literal_step_ids = Vec::with_capacity(viz.text_literals.len());
        for tl in viz.text_literals.iter() {
            use crate::entities::visualization::text_literal::TextLiteralHandler;
            let step = TextLiteralHandler::write(self, tl.clone())?;
            self.text_literal_step_ids.push(step);
        }
        // COMPOSITE_TEXT — depends on text_literal_step_ids just filled.
        self.composite_text_step_ids = Vec::with_capacity(viz.composite_texts.len());
        for ct in viz.composite_texts.iter() {
            use crate::entities::visualization::composite_text::CompositeTextHandler;
            let step = CompositeTextHandler::write(self, ct.clone())?;
            self.composite_text_step_ids.push(step);
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
        // Populates `viz_camera_model_step_ids` so `CAMERA_USAGE` can
        // resolve `mapping_origin` through one index lookup later.
        self.viz_camera_model_step_ids = Vec::with_capacity(viz.camera_models.len());
        for cm in viz.camera_models.iter() {
            use crate::ir::visualization::CameraModel as CM;
            let step = match cm {
                CM::CameraModelD3(d3) => CameraModelD3Handler::write(self, d3.clone())?,
                CM::CameraModelD3WithHlhsr(c) => {
                    use crate::entities::visualization::camera_model_variants::CameraModelD3WithHlhsrHandler;
                    CameraModelD3WithHlhsrHandler::write(self, c.clone())?
                }
                CM::CameraModelD3MultiClipping(c) => {
                    use crate::entities::visualization::camera_model_variants::CameraModelD3MultiClippingHandler;
                    CameraModelD3MultiClippingHandler::write(self, c.clone())?
                }
            };
            self.viz_camera_model_step_ids.push(step);
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
                PresentationStyleAssignment::PresentationStyleByContext(psbc) => {
                    use crate::entities::visualization::presentation_style_by_context::PresentationStyleByContextHandler;
                    PresentationStyleByContextHandler::write(self, psbc.clone())?
                }
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
        // MDGPR — iterate the unified `representations` arena so each
        // step id lands in its `RepresentationId` slot. `viz.mdgprs` is a
        // dual-write of the same data and is intentionally ignored here.
        let reprs = self.model.representations.clone();
        for (id, repr) in reprs.iter_with_ids() {
            if let crate::ir::shape_rep::Representation::Mdgpr(mdgpr) = repr {
                let step_id = MdgprHandler::write(self, mdgpr.clone())?;
                self.representation_step_ids[id.0 as usize] = step_id;
            }
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
        for (idx, item) in founded_items.iter().enumerate() {
            if let FoundedItem::SurfaceStyleBoundary(ssb) = item {
                use crate::entities::visualization::surface_style_boundary::SurfaceStyleBoundaryHandler;
                self.founded_item_step_ids[idx] =
                    SurfaceStyleBoundaryHandler::write(self, ssb.clone())?;
            }
        }
        for (idx, item) in founded_items.iter().enumerate() {
            if let FoundedItem::SurfaceStyleParameterLine(sspl) = item {
                use crate::entities::visualization::surface_style_parameter_line::SurfaceStyleParameterLineHandler;
                self.founded_item_step_ids[idx] =
                    SurfaceStyleParameterLineHandler::write(self, sspl.clone())?;
            }
        }
        Ok(())
    }

    /// Emit the `CompoundRepresentationItem` arena (phase cri). Orphan —
    /// no inbound refs. Each child is either an inline
    /// `DescriptiveRepresentationItem` (re-emitted in place) or any
    /// resolvable `RepresentationItemRef`.
    pub(in crate::writer::buffer) fn emit_compound_representation_items(
        &mut self,
    ) -> Result<(), WriteError> {
        let items: Vec<_> = self
            .model
            .compound_representation_items
            .iter()
            .cloned()
            .collect();
        for cri in items {
            use crate::entities::SimpleEntityHandler;
            use crate::entities::shape_rep::compound_representation_item::CompoundRepresentationItemHandler;
            CompoundRepresentationItemHandler::write(self, cri)?;
        }
        Ok(())
    }

    /// Emit the `GeometricRepresentationItem` arena (phase ds-st) in
    /// two passes — `SymbolTarget` first so `DefinedSymbol.target` resolves
    /// through the populated `geometric_representation_item_step_ids` cache.
    pub(in crate::writer::buffer) fn emit_geometric_representation_items(
        &mut self,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::visualization::symbol::{DefinedSymbolHandler, SymbolTargetHandler};
        use crate::ir::visualization::GeometricRepresentationItem as GRI;
        let items: Vec<_> = self
            .model
            .geometric_representation_items
            .iter()
            .cloned()
            .collect();
        // SBSM entries are emitted earlier (phase sbsm-cluster-b) via
        // `emit_sbsm_in_gri_arena` so `emit_representation` can resolve
        // them through the GRI cache. The cache was sized + the SBSM
        // slots filled there; the SymbolTarget / DefinedSymbol loops
        // below only fill their own slots and leave SBSM slots intact.
        for (idx, item) in items.iter().enumerate() {
            if let GRI::SymbolTarget(t) = item {
                self.geometric_representation_item_step_ids[idx] =
                    SymbolTargetHandler::write(self, t.clone())?;
            }
        }
        for (idx, item) in items.iter().enumerate() {
            if let GRI::DefinedSymbol(d) = item {
                self.geometric_representation_item_step_ids[idx] =
                    DefinedSymbolHandler::write(self, d.clone())?;
            }
        }
        Ok(())
    }

    /// Emit the `ShellBasedSurfaceModel` slots of the
    /// `geometric_representation_item` arena early, before the product chain
    /// resolves `MANIFOLD_SURFACE_SHAPE_REPRESENTATION` children through the
    /// GRI cache (phase sbsm-cluster). Sizes the cache so the later
    /// `emit_geometric_representation_items` symbol passes can index into
    /// it without touching SBSM slots.
    pub(in crate::writer::buffer) fn emit_sbsm_in_gri_arena(&mut self) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::geometry::geometric_curve_set::{
            CurveSetWriteInput, GeometricCurveSetHandler,
        };
        use crate::entities::geometry::geometric_set::GeometricSetHandler;
        use crate::entities::geometry::shell_based_surface_model::ShellBasedSurfaceModelHandler;
        use crate::ir::visualization::GeometricRepresentationItem as GRI;
        let items: Vec<_> = self
            .model
            .geometric_representation_items
            .iter()
            .cloned()
            .collect();
        self.geometric_representation_item_step_ids = vec![0; items.len()];
        for (idx, item) in items.iter().enumerate() {
            match item {
                GRI::ShellBasedSurfaceModel(sbsm) => {
                    self.geometric_representation_item_step_ids[idx] =
                        ShellBasedSurfaceModelHandler::write(self, sbsm.shells.clone())?;
                }
                GRI::GeometricCurveSet(gcs) => {
                    self.geometric_representation_item_step_ids[idx] =
                        GeometricCurveSetHandler::write(
                            self,
                            CurveSetWriteInput {
                                curves: gcs.curves.clone(),
                                points: gcs.points.clone(),
                            },
                        )?;
                }
                GRI::GeometricSet(gs) => {
                    self.geometric_representation_item_step_ids[idx] = GeometricSetHandler::write(
                        self,
                        CurveSetWriteInput {
                            curves: gs.curves.clone(),
                            points: gs.points.clone(),
                        },
                    )?;
                }
                GRI::DefinedSymbol(_) | GRI::SymbolTarget(_) => {
                    // Symbol-domain entries are emitted after the
                    // visualization pass by
                    // `emit_geometric_representation_items` because they
                    // depend on caches that pass fills.
                }
            }
        }
        Ok(())
    }
}
