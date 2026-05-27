//! PMI emission — `SHAPE_ASPECT` entries that anchor future Tolerance /
//! Datum / GD&T work. Each `ShapeAspect` re-emits via the
//! [`ShapeAspectHandler::write`] handler (single source of truth) using the
//! cached `PRODUCT_DEFINITION_SHAPE` step id populated by the assembly emitter.

use super::WriteBuffer;
use crate::entities::SimpleEntityHandler;
use crate::entities::shape_rep::shape_aspect::{ShapeAspectHandler, ShapeAspectWriteInput};
use crate::entities::shape_rep::shape_aspect_subtypes::{
    AllAroundShapeAspectHandler, CentreOfSymmetryHandler, CompositeGroupShapeAspectHandler,
    ShapeAspectSubtypeWriteInput,
};
use crate::ir::shape_aspect_ref::ShapeAspectRef;
use crate::ir::shape_rep::ShapeAspect;
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_pmi_if_set(&mut self) {
        // Snapshot to release the &model borrow before per-SA emission
        // (handler.write mutates self via push_simple).
        let entries: Vec<ShapeAspect> = self.model.shape_aspects.iter().cloned().collect();
        self.shape_aspect_step_ids.resize(entries.len(), 0);
        for (index, sa) in entries.iter().enumerate() {
            // Defensive: silent skip when product chain wasn't emitted (e.g.
            // kernel-built IR with PMI only, or model.units empty so the
            // assembly pass bailed out). Reader symmetry — reader silent
            // skips SAs whose target ref doesn't resolve.
            let Some(&pds_step_id) = self.product_def_shape_ids.get(&sa.target) else {
                continue;
            };
            if let Ok(step_id) = ShapeAspectHandler::write(
                self,
                ShapeAspectWriteInput {
                    name: sa.name.clone(),
                    description: sa.description.clone(),
                    pds_step_id,
                    product_definitional: sa.product_definitional,
                },
            ) {
                self.shape_aspect_step_ids[index] = step_id;
            }
        }
        self.emit_shape_aspect_subtypes();
        self.emit_datums();
        self.emit_datum_features();
        self.emit_pmi_pool();
    }

    /// Emit the `pmi` pool's `DATUM` arena. Each datum resolves its
    /// `target` (`ProductId`) to a `PRODUCT_DEFINITION_SHAPE` step id via
    /// `product_def_shape_ids` — the same chain as `SHAPE_ASPECT`. Silent
    /// skip when the product chain was not emitted (reader symmetry). The
    /// emitted step id is index-cached in `datum_step_ids` (by `DatumId.0`)
    /// so `emit_shape_aspect_ref` can resolve a `ShapeAspectRef::Datum`.
    fn emit_datums(&mut self) {
        use crate::entities::pmi::{DatumHandler, DatumWriteInput};
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        self.datum_step_ids.resize(pmi.datums.len(), 0);
        for (index, datum) in pmi.datums.iter().enumerate() {
            let Some(&pds_step_id) = self.product_def_shape_ids.get(&datum.target) else {
                continue;
            };
            if let Ok(step_id) = DatumHandler::write(
                self,
                DatumWriteInput {
                    name: datum.name.clone(),
                    description: datum.description.clone(),
                    pds_step_id,
                    product_definitional: datum.product_definitional,
                    identification: datum.identification.clone(),
                },
            ) {
                self.datum_step_ids[index] = step_id;
            }
        }
    }

    /// Emit the `pmi` pool's `DATUM_FEATURE` arena. Same `target` →
    /// `PRODUCT_DEFINITION_SHAPE` resolution as `DATUM`; the emitted step id
    /// is index-cached in `datum_feature_step_ids` (by `DatumFeatureId.0`)
    /// for `emit_shape_aspect_ref`.
    fn emit_datum_features(&mut self) {
        use crate::entities::pmi::{DatumFeatureHandler, DatumFeatureWriteInput};
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        self.datum_feature_step_ids
            .resize(pmi.datum_features.len(), 0);
        for (index, df) in pmi.datum_features.iter().enumerate() {
            let Some(&pds_step_id) = self.product_def_shape_ids.get(&df.target) else {
                continue;
            };
            // Shared writer dispatches the STEP entity name on `df.kind`
            // (`DATUM_FEATURE` vs `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE`).
            if let Ok(step_id) = DatumFeatureHandler::write(
                self,
                DatumFeatureWriteInput {
                    name: df.name.clone(),
                    description: df.description.clone(),
                    pds_step_id,
                    product_definitional: df.product_definitional,
                    kind: df.kind,
                },
            ) {
                self.datum_feature_step_ids[index] = step_id;
            }
        }
    }

    /// Emit the `pmi` pool's leaf primitives (`TOLERANCE_ZONE_FORM` /
    /// `TYPE_QUALIFIER` / `VALUE_FORMAT_TYPE_QUALIFIER` /
    /// `DRAUGHTING_PRE_DEFINED_TEXT_FONT`). No cross-refs — each is a
    /// standalone 1-attr entity.
    fn emit_pmi_pool(&mut self) {
        use crate::entities::pmi::{
            DraughtingPreDefinedTextFontHandler, ToleranceZoneFormHandler, TypeQualifierHandler,
            ValueFormatTypeQualifierHandler,
        };
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        self.tolerance_zone_form_step_ids
            .resize(pmi.tolerance_zone_forms.len(), 0);
        for (index, tzf) in pmi.tolerance_zone_forms.iter().enumerate() {
            if let Ok(step_id) = ToleranceZoneFormHandler::write(self, tzf.clone()) {
                self.tolerance_zone_form_step_ids[index] = step_id;
            }
        }
        for tq in pmi.type_qualifiers.iter() {
            let step = TypeQualifierHandler::write(self, tq.clone()).unwrap_or(0);
            self.type_qualifier_step_ids.push(step);
        }
        for vftq in pmi.value_format_type_qualifiers.iter() {
            let step = ValueFormatTypeQualifierHandler::write(self, vftq.clone()).unwrap_or(0);
            self.value_format_type_qualifier_step_ids.push(step);
        }
        self.dptf_step_ids = Vec::with_capacity(pmi.draughting_pre_defined_text_fonts.len());
        for font in pmi.draughting_pre_defined_text_fonts.iter() {
            let step = DraughtingPreDefinedTextFontHandler::write(self, font.clone()).unwrap_or(0);
            self.dptf_step_ids.push(step);
        }
    }

    /// Emit the `pmi` pool's `DRAUGHTING_MODEL_ITEM_ASSOCIATION` arena
    /// (phase dmia). Called from `emit_pools` after
    /// `emit_annotation_occurrences` + `emit_draughting_callouts` so that
    /// `ao_step_ids` / `draughting_callout_step_ids` are populated, and
    /// after the representation chain so `representation_step_ids` is
    /// ready.
    pub(in crate::writer::buffer) fn emit_dmia(&mut self) {
        use crate::entities::pmi::DraughtingModelItemAssociationHandler;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        self.dmia_step_ids = Vec::with_capacity(pmi.draughting_model_item_associations.len());
        for dmia in pmi.draughting_model_item_associations.iter() {
            let step =
                DraughtingModelItemAssociationHandler::write(self, dmia.clone()).unwrap_or(0);
            self.dmia_step_ids.push(step);
        }
    }

    /// Emit the `presentation_representations` + `presentation_sets`
    /// arenas (phase pr-core). Delayed emit — depends on every item ref
    /// cache (`styled_item` / `repr` / `repr_item` / per-geometry placements)
    /// populated by earlier passes.
    pub(in crate::writer::buffer) fn emit_presentation_repr_cluster(
        &mut self,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::visualization::presentation_representation::{
            PresentationAreaHandler, PresentationSetHandler, PresentationViewHandler,
        };
        use crate::ir::visualization::PresentationRepresentation;
        let Some(viz) = self.model.visualization.clone() else {
            return Ok(());
        };
        self.presentation_representation_step_ids =
            Vec::with_capacity(viz.presentation_representations.len());
        for repr in viz.presentation_representations.iter() {
            let step = match repr {
                PresentationRepresentation::View(d) => {
                    PresentationViewHandler::write(self, d.clone())?
                }
                PresentationRepresentation::Area(d) => {
                    PresentationAreaHandler::write(self, d.clone())?
                }
            };
            self.presentation_representation_step_ids.push(step);
        }
        self.presentation_set_step_ids = Vec::with_capacity(viz.presentation_sets.len());
        for set in viz.presentation_sets.iter() {
            let step = PresentationSetHandler::write(self, set.clone())?;
            self.presentation_set_step_ids.push(step);
        }
        Ok(())
    }

    /// Emit `presented_item_representation` + `applied_presented_item`
    /// arenas (phase pr-item).
    pub(in crate::writer::buffer) fn emit_pr_item(&mut self) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::visualization::presented_item::{
            AppliedPresentedItemHandler, PresentedItemRepresentationHandler,
        };
        let Some(viz) = self.model.visualization.clone() else {
            return Ok(());
        };
        for pir in viz.presented_item_representations.iter() {
            let _ = PresentedItemRepresentationHandler::write(self, *pir)?;
        }
        for api in viz.applied_presented_items.iter() {
            let _ = AppliedPresentedItemHandler::write(self, api.clone())?;
        }
        Ok(())
    }

    /// Emit `area_in_set` + `presentation_size` arenas (phase pr-size).
    /// Depends on `presentation_representation_step_ids`,
    /// `presentation_set_step_ids`, and the planar-extent cache.
    pub(in crate::writer::buffer) fn emit_pr_size(&mut self) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::visualization::presentation_size::{
            AreaInSetHandler, PresentationSizeHandler,
        };
        let Some(viz) = self.model.visualization.clone() else {
            return Ok(());
        };
        self.area_in_set_step_ids = Vec::with_capacity(viz.area_in_sets.len());
        for ais in viz.area_in_sets.iter() {
            let step = AreaInSetHandler::write(self, *ais)?;
            self.area_in_set_step_ids.push(step);
        }
        for ps in viz.presentation_sizes.iter() {
            let _ = PresentationSizeHandler::write(self, *ps)?;
        }
        Ok(())
    }

    /// Emit the `invisibilities` arena (phase invisibility). Called from
    /// `emit_pools` after `emit_draughting_callouts` and
    /// `emit_draughting_models` so every `invisible_item` cache
    /// (`styled_item_step_ids`, `representation_step_ids`,
    /// `draughting_callout_step_ids`) is populated.
    pub(in crate::writer::buffer) fn emit_invisibilities(&mut self) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::visualization::invisibility::InvisibilityHandler;
        let Some(viz) = self.model.visualization.clone() else {
            return Ok(());
        };
        self.invisibility_step_ids = Vec::with_capacity(viz.invisibilities.len());
        for inv in viz.invisibilities.iter() {
            let step = InvisibilityHandler::write(self, inv.clone())?;
            self.invisibility_step_ids.push(step);
        }
        Ok(())
    }

    /// Emit the `geometric_item_specific_usages` arena (phase gisu).
    /// Sibling of `emit_dmia` — same dependencies (shape-aspect family
    /// step ids, representation step ids, `representation_item` step ids).
    pub(in crate::writer::buffer) fn emit_gisu(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::geometric_item_specific_usage::GeometricItemSpecificUsageHandler;
        let entries: Vec<_> = self
            .model
            .geometric_item_specific_usages
            .iter()
            .cloned()
            .collect();
        self.gisu_step_ids = Vec::with_capacity(entries.len());
        for gisu in entries {
            let step = GeometricItemSpecificUsageHandler::write(self, gisu).unwrap_or(0);
            self.gisu_step_ids.push(step);
        }
    }

    /// Emit the `annotation_occurrence` arena (`ANNOTATION_PLANE` /
    /// `TESSELLATED_ANNOTATION_OCCURRENCE`). Called after
    /// `emit_visualization_if_set` (`styles` → `psa_step_ids`) and after
    /// `emit_tessellation` (`TessellatedAnnotationOccurrence.item` →
    /// `tessellated_item_step_ids`).
    pub(in crate::writer::buffer) fn emit_annotation_occurrences(&mut self) {
        use crate::entities::pmi::{
            AnnotationPlaneHandler, AnnotationSymbolOccurrenceHandler,
            AnnotationTextOccurrenceHandler, DraughtingAnnotationOccurrenceHandler,
            LeaderTerminatorHandler, TerminatorSymbolHandler,
            TessellatedAnnotationOccurrenceHandler,
        };
        use crate::ir::pmi::AnnotationOccurrence;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        for ao in pmi.annotation_occurrences.iter() {
            let step = match ao {
                AnnotationOccurrence::AnnotationPlane(ap) => {
                    AnnotationPlaneHandler::write(self, ap.clone())
                }
                AnnotationOccurrence::TessellatedAnnotationOccurrence(tao) => {
                    TessellatedAnnotationOccurrenceHandler::write(self, tao.clone())
                }
                AnnotationOccurrence::AnnotationSymbolOccurrence(aso) => {
                    AnnotationSymbolOccurrenceHandler::write(self, aso.clone())
                }
                AnnotationOccurrence::AnnotationTextOccurrence(ato) => {
                    AnnotationTextOccurrenceHandler::write(self, ato.clone())
                }
                AnnotationOccurrence::DraughtingAnnotationOccurrence(dao) => {
                    DraughtingAnnotationOccurrenceHandler::write(self, dao.clone())
                }
                AnnotationOccurrence::TerminatorSymbol(ts) => {
                    TerminatorSymbolHandler::write(self, ts.clone())
                }
                AnnotationOccurrence::LeaderTerminator(lt) => {
                    LeaderTerminatorHandler::write(self, lt.clone())
                }
            };
            // Index by AnnotationOccurrenceId.0 — arena order matches enum
            // iteration order. Drop errors (push 0) to keep the index aligned.
            self.ao_step_ids.push(step.unwrap_or(0));
        }
    }

    /// Emit the `draughting_callout` arena (phase draughting-callout).
    /// Fills `draughting_callout_step_ids` so
    /// `emit_draughting_callout_relationships` can resolve relating /
    /// related refs.
    pub(in crate::writer::buffer) fn emit_draughting_callouts(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::pmi::{DraughtingCalloutHandler, LeaderDirectedCalloutHandler};
        use crate::ir::pmi::DraughtingCallout;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        for dc in pmi.draughting_callouts.iter() {
            let step = match dc {
                DraughtingCallout::Plain(data) => {
                    DraughtingCalloutHandler::write(self, data.clone())
                }
                DraughtingCallout::LeaderDirected(data) => {
                    LeaderDirectedCalloutHandler::write(self, data.clone())
                }
            };
            self.draughting_callout_step_ids.push(step.unwrap_or(0));
        }
    }

    /// Emit the `geometric_tolerance_relationship` arena (phase
    /// gt-relationship). Both GT step-id caches must be filled by the
    /// preceding GT / `GT_with_datum_reference` emits.
    pub(in crate::writer::buffer) fn emit_geometric_tolerance_relationships(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::pmi::GeometricToleranceRelationshipHandler;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        for rel in pmi.geometric_tolerance_relationships.iter() {
            let _ = GeometricToleranceRelationshipHandler::write(self, rel.clone());
        }
    }

    /// Emit the `draughting_callout_relationship` arena.
    pub(in crate::writer::buffer) fn emit_draughting_callout_relationships(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::pmi::DraughtingCalloutRelationshipHandler;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        for rel in pmi.draughting_callout_relationships.iter() {
            let _ = DraughtingCalloutRelationshipHandler::write(self, rel.clone());
        }
    }

    /// Emit the `annotation_curve_occurrence` arena (currently
    /// `LEADER_CURVE` only). Fills `acoc_step_ids` so
    /// `TERMINATOR_SYMBOL` / `LEADER_TERMINATOR` in
    /// `emit_annotation_occurrences` can resolve `annotated_curve`.
    pub(in crate::writer::buffer) fn emit_annotation_curve_occurrences(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::pmi::LeaderCurveHandler;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        for lc in pmi.annotation_curve_occurrences.iter() {
            match LeaderCurveHandler::write(self, lc.clone()) {
                Ok(n) => self.acoc_step_ids.push(n),
                Err(_) => self.acoc_step_ids.push(0),
            }
        }
    }

    /// Emit the `SHAPE_ASPECT` subtype arenas (`COMPOSITE_GROUP_SHAPE_ASPECT`
    /// / `CENTRE_OF_SYMMETRY` / `ALL_AROUND_SHAPE_ASPECT`). Same `target` →
    /// `PRODUCT_DEFINITION_SHAPE` resolution as `SHAPE_ASPECT`; no step-id
    /// cache is kept since nothing references these entities yet.
    fn emit_shape_aspect_subtypes(&mut self) {
        // Snapshot each arena to release the &model borrow before emitting
        // (handler.write mutates self via push_simple). Each subtype keeps a
        // step-id cache (index-assigned, dropped entries stay 0) so
        // `emit_shape_aspect_ref` can resolve a `ShapeAspectRef`.
        let composite: Vec<_> = self
            .model
            .composite_group_shape_aspects
            .iter()
            .cloned()
            .collect();
        self.composite_shape_aspect_step_ids
            .resize(composite.len(), 0);
        for (index, sa) in composite.into_iter().enumerate() {
            let Some(&pds_step_id) = self.product_def_shape_ids.get(&sa.target) else {
                continue;
            };
            if let Ok(step_id) = CompositeGroupShapeAspectHandler::write(
                self,
                ShapeAspectSubtypeWriteInput {
                    name: sa.name,
                    description: sa.description,
                    pds_step_id,
                    product_definitional: sa.product_definitional,
                },
            ) {
                self.composite_shape_aspect_step_ids[index] = step_id;
            }
        }
        let centre: Vec<_> = self.model.centre_of_symmetries.iter().cloned().collect();
        self.centre_of_symmetry_step_ids.resize(centre.len(), 0);
        for (index, sa) in centre.into_iter().enumerate() {
            let Some(&pds_step_id) = self.product_def_shape_ids.get(&sa.target) else {
                continue;
            };
            if let Ok(step_id) = CentreOfSymmetryHandler::write(
                self,
                ShapeAspectSubtypeWriteInput {
                    name: sa.name,
                    description: sa.description,
                    pds_step_id,
                    product_definitional: sa.product_definitional,
                },
            ) {
                self.centre_of_symmetry_step_ids[index] = step_id;
            }
        }
        let all_around: Vec<_> = self
            .model
            .all_around_shape_aspects
            .iter()
            .cloned()
            .collect();
        self.all_around_shape_aspect_step_ids
            .resize(all_around.len(), 0);
        for (index, sa) in all_around.into_iter().enumerate() {
            let Some(&pds_step_id) = self.product_def_shape_ids.get(&sa.target) else {
                continue;
            };
            if let Ok(step_id) = AllAroundShapeAspectHandler::write(
                self,
                ShapeAspectSubtypeWriteInput {
                    name: sa.name,
                    description: sa.description,
                    pds_step_id,
                    product_definitional: sa.product_definitional,
                },
            ) {
                self.all_around_shape_aspect_step_ids[index] = step_id;
            }
        }
    }

    /// Resolve a [`ShapeAspectRef`] to its emitted STEP id — a pure cache
    /// lookup, since every shape-aspect-family arena is emitted up-front by
    /// `emit_pmi_if_set` / `emit_shape_aspect_subtypes`. A `0` slot means
    /// the target was dropped at emit (product chain unresolved); in
    /// practice unreachable because a `shape_aspect` that read successfully
    /// also writes successfully (symmetric product-chain resolution).
    pub(crate) fn emit_shape_aspect_ref(&self, item: ShapeAspectRef) -> u64 {
        match item {
            ShapeAspectRef::ShapeAspect(id) => self.shape_aspect_step_ids[id.0 as usize],
            ShapeAspectRef::CompositeGroupShapeAspect(id) => {
                self.composite_shape_aspect_step_ids[id.0 as usize]
            }
            ShapeAspectRef::CentreOfSymmetry(id) => self.centre_of_symmetry_step_ids[id.0 as usize],
            ShapeAspectRef::AllAroundShapeAspect(id) => {
                self.all_around_shape_aspect_step_ids[id.0 as usize]
            }
            ShapeAspectRef::Datum(id) => self.datum_step_ids[id.0 as usize],
            ShapeAspectRef::DatumFeature(id) => self.datum_feature_step_ids[id.0 as usize],
            ShapeAspectRef::DatumSystem(id) => self.datum_system_step_ids[id.0 as usize],
            ShapeAspectRef::DatumTarget(id) => self.datum_target_step_ids[id.0 as usize],
            ShapeAspectRef::PlacedDatumTargetFeature(id) => {
                self.placed_datum_target_feature_step_ids[id.0 as usize]
            }
        }
    }

    /// Emit the `SHAPE_ASPECT_RELATIONSHIP` arena. Runs after
    /// `emit_pmi_if_set` so every shape-aspect-family step-id cache is
    /// filled and both endpoints resolve through `emit_shape_aspect_ref`.
    pub(in crate::writer::buffer) fn emit_shape_aspect_relationships(
        &mut self,
    ) -> Result<(), WriteError> {
        use crate::entities::shape_rep::shape_aspect_relationship::ShapeAspectRelationshipHandler;
        let rels: Vec<_> = self
            .model
            .shape_aspect_relationships
            .iter()
            .cloned()
            .collect();
        for rel in rels {
            ShapeAspectRelationshipHandler::write(self, rel)?;
        }
        Ok(())
    }

    /// Emit the `pmi` pool's `DIMENSIONAL_SIZE` arena. Runs after
    /// `emit_pmi_if_set` so `applies_to` resolves through the filled
    /// shape-aspect-family step-id caches.
    pub(in crate::writer::buffer) fn emit_dimensional_sizes(&mut self) -> Result<(), WriteError> {
        use crate::entities::pmi::DimensionalSizeHandler;
        let Some(pmi) = self.model.pmi.clone() else {
            return Ok(());
        };
        self.dimensional_size_step_ids
            .resize(pmi.dimensional_sizes.len(), 0);
        for (index, ds) in pmi.dimensional_sizes.iter().enumerate() {
            let step_id = DimensionalSizeHandler::write(self, ds.clone())?;
            self.dimensional_size_step_ids[index] = step_id;
        }
        Ok(())
    }

    /// Emit the `pmi` pool's `dimensional_location` arena
    /// (`DIMENSIONAL_LOCATION` / `DIRECTED_DIMENSIONAL_LOCATION`). Runs
    /// after `emit_pmi_if_set` so both endpoints resolve through the filled
    /// shape-aspect-family step-id caches.
    pub(in crate::writer::buffer) fn emit_dimensional_locations(
        &mut self,
    ) -> Result<(), WriteError> {
        use crate::entities::pmi::DimensionalLocationHandler;
        let Some(pmi) = self.model.pmi.clone() else {
            return Ok(());
        };
        self.dimensional_location_step_ids
            .resize(pmi.dimensional_locations.len(), 0);
        for (index, dl) in pmi.dimensional_locations.iter().enumerate() {
            let step_id = DimensionalLocationHandler::write(self, dl.clone())?;
            self.dimensional_location_step_ids[index] = step_id;
        }
        Ok(())
    }

    /// Emit the `pmi` pool's `geometric_tolerance` arena (form tolerances).
    /// Runs after `emit_pmi_if_set` (shape-aspect step-id caches) and the
    /// units pass (`mwu_step_ids`) so `write_geometric_tolerance` resolves
    /// both `magnitude` and `toleranced_shape_aspect`.
    pub(in crate::writer::buffer) fn emit_geometric_tolerances(&mut self) {
        use crate::entities::pmi::write_geometric_tolerance;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        self.geometric_tolerance_step_ids
            .resize(pmi.geometric_tolerances.len(), 0);
        for (index, gt) in pmi.geometric_tolerances.iter().enumerate() {
            let step_id = write_geometric_tolerance(self, gt.clone());
            self.geometric_tolerance_step_ids[index] = step_id;
        }
    }

    /// Emit the `pmi` pool's `general_datum_reference` arena
    /// (`DATUM_REFERENCE_COMPARTMENT` / `DATUM_REFERENCE_ELEMENT`). Runs
    /// after `emit_pmi_if_set` so `datum_step_ids` (for `base`) and
    /// `product_def_shape_ids` (for `of_shape`) are filled. The emitted
    /// step id is index-cached in `general_datum_reference_step_ids` so
    /// `emit_datum_systems` can resolve a `DATUM_SYSTEM`'s `constituents`.
    pub(in crate::writer::buffer) fn emit_general_datum_references(&mut self) {
        use crate::entities::pmi::write_general_datum_reference;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        self.general_datum_reference_step_ids
            .resize(pmi.general_datum_references.len(), 0);
        for (index, gdr) in pmi.general_datum_references.iter().enumerate() {
            let step_id = write_general_datum_reference(self, gdr.clone());
            self.general_datum_reference_step_ids[index] = step_id;
        }
    }

    /// Emit the `datum_systems` arena (`DATUM_SYSTEM`). Runs after
    /// `emit_general_datum_references` so `constituents` resolve through
    /// `general_datum_reference_step_ids`, and before the `ShapeAspectRef`
    /// consumers so `datum_system_step_ids` is filled when
    /// `emit_shape_aspect_ref` runs.
    pub(in crate::writer::buffer) fn emit_datum_systems(&mut self) {
        use crate::entities::shape_rep::datum_system::DatumSystemHandler;
        let systems: Vec<_> = self.model.datum_systems.iter().cloned().collect();
        self.datum_system_step_ids.resize(systems.len(), 0);
        for (index, ds) in systems.into_iter().enumerate() {
            if let Ok(step_id) = DatumSystemHandler::write(self, ds) {
                self.datum_system_step_ids[index] = step_id;
            }
        }
    }

    /// Emit the `datum_targets` arena (`DATUM_TARGET`). Same surrounding
    /// constraints as `emit_datum_systems` — product-chain refs resolved
    /// through `product_def_shape_ids`, downstream `ShapeAspectRef`
    /// consumers see a filled `datum_target_step_ids` cache.
    pub(in crate::writer::buffer) fn emit_datum_targets(&mut self) {
        use crate::entities::shape_rep::datum_target::DatumTargetHandler;
        let targets: Vec<_> = self.model.datum_targets.iter().cloned().collect();
        self.datum_target_step_ids.resize(targets.len(), 0);
        for (index, dt) in targets.into_iter().enumerate() {
            if let Ok(step_id) = DatumTargetHandler::write(self, dt) {
                self.datum_target_step_ids[index] = step_id;
            }
        }
    }

    /// Emit the `placed_datum_target_features` arena.
    pub(in crate::writer::buffer) fn emit_placed_datum_target_features(&mut self) {
        use crate::entities::shape_rep::placed_datum_target_feature::PlacedDatumTargetFeatureHandler;
        let entries: Vec<_> = self
            .model
            .placed_datum_target_features
            .iter()
            .cloned()
            .collect();
        self.placed_datum_target_feature_step_ids
            .resize(entries.len(), 0);
        for (index, p) in entries.into_iter().enumerate() {
            if let Ok(step_id) = PlacedDatumTargetFeatureHandler::write(self, p) {
                self.placed_datum_target_feature_step_ids[index] = step_id;
            }
        }
    }

    /// Emit the `tolerance_zones` arena (`TOLERANCE_ZONE`). Runs after both
    /// `emit_geometric_tolerances` / `emit_geometric_tolerance_with_datum_references`
    /// (the `defining_tolerance` caches) and `emit_pmi_if_set`
    /// (`tolerance_zone_form_step_ids`). No step-id cache — nothing
    /// references a `TOLERANCE_ZONE`.
    pub(in crate::writer::buffer) fn emit_tolerance_zones(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::tolerance_zone::ToleranceZoneHandler;
        let zones: Vec<_> = self.model.tolerance_zones.iter().cloned().collect();
        for tz in zones {
            let step = ToleranceZoneHandler::write(self, tz).unwrap_or(0);
            self.tolerance_zone_step_ids.push(step);
        }
    }

    /// Emit the `measure_qualification` arena (phase
    /// measure-qualification). Runs after the qualifier emits in
    /// `emit_pmi_pool` (`type_qualifier_step_ids` /
    /// `value_format_type_qualifier_step_ids`) and the units pass
    /// (`mwu_step_ids`).
    pub(in crate::writer::buffer) fn emit_measure_qualifications(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::pmi::MeasureQualificationHandler;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        for mq in pmi.measure_qualifications.iter() {
            let _ = MeasureQualificationHandler::write(self, mq.clone());
        }
    }

    /// Emit the `tolerance_zone_definition` arena (phase projected-zone).
    /// Runs after `emit_tolerance_zones` (`tolerance_zone_step_ids`),
    /// `emit_pmi_if_set` (shape-aspect caches consumed by
    /// `emit_shape_aspect_ref`), and the units pass (`mwu_step_ids`).
    pub(in crate::writer::buffer) fn emit_tolerance_zone_definitions(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::pmi::ProjectedZoneDefinitionHandler;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        for pzd in pmi.tolerance_zone_definitions.iter() {
            let _ = ProjectedZoneDefinitionHandler::write(self, pzd.clone());
        }
    }

    /// Emit the `pmi` pool's `geometric_tolerance_with_datum_reference`
    /// arena. Runs after `emit_datum_systems` (`datum_system_step_ids`), the
    /// units pass (`mwu_step_ids`) and `emit_pmi_if_set` (shape-aspect
    /// caches) so `write_geometric_tolerance_with_datum_reference` resolves
    /// every ref.
    pub(in crate::writer::buffer) fn emit_geometric_tolerance_with_datum_references(&mut self) {
        use crate::entities::pmi::write_geometric_tolerance_with_datum_reference;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        self.geometric_tolerance_with_datum_reference_step_ids
            .resize(pmi.geometric_tolerance_with_datum_references.len(), 0);
        for (index, gt) in pmi
            .geometric_tolerance_with_datum_references
            .iter()
            .enumerate()
        {
            let step_id = write_geometric_tolerance_with_datum_reference(self, gt.clone());
            self.geometric_tolerance_with_datum_reference_step_ids[index] = step_id;
        }
    }

    /// Emit the `pmi` pool's `TOLERANCE_VALUE` arena, index-caching the step
    /// ids in `tolerance_value_step_ids` for `emit_plus_minus_tolerances`.
    pub(in crate::writer::buffer) fn emit_tolerance_values(&mut self) {
        use crate::entities::pmi::write_tolerance_value;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        self.tolerance_value_step_ids
            .resize(pmi.tolerance_values.len(), 0);
        for (index, tv) in pmi.tolerance_values.iter().enumerate() {
            let step_id = write_tolerance_value(self, tv);
            self.tolerance_value_step_ids[index] = step_id;
        }
    }

    /// Emit the `pmi` pool's `LIMITS_AND_FITS` arena, index-caching the step
    /// ids in `limits_and_fits_step_ids` for `emit_plus_minus_tolerances`.
    pub(in crate::writer::buffer) fn emit_limits_and_fits(&mut self) {
        use crate::entities::pmi::write_limits_and_fits;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        self.limits_and_fits_step_ids
            .resize(pmi.limits_and_fits.len(), 0);
        for (index, lf) in pmi.limits_and_fits.iter().enumerate() {
            let step_id = write_limits_and_fits(self, lf.clone());
            self.limits_and_fits_step_ids[index] = step_id;
        }
    }

    /// Emit the `pmi` pool's `PLUS_MINUS_TOLERANCE` arena. Runs after
    /// `emit_tolerance_values` / `emit_limits_and_fits` (`range`) and
    /// `emit_dimensional_*` (`toleranced_dimension`).
    pub(in crate::writer::buffer) fn emit_plus_minus_tolerances(&mut self) {
        use crate::entities::pmi::write_plus_minus_tolerance;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        for pmt in pmi.plus_minus_tolerances.iter() {
            write_plus_minus_tolerance(self, pmt);
        }
    }
}
