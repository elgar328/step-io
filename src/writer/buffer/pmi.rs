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
        use crate::entities::pmi::DatumFeatureHandler;
        use crate::entities::shape_rep::shape_aspect::ShapeAspectWriteInput;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        self.datum_feature_step_ids
            .resize(pmi.datum_features.len(), 0);
        for (index, df) in pmi.datum_features.iter().enumerate() {
            let Some(&pds_step_id) = self.product_def_shape_ids.get(&df.target) else {
                continue;
            };
            if let Ok(step_id) = DatumFeatureHandler::write(
                self,
                ShapeAspectWriteInput {
                    name: df.name.clone(),
                    description: df.description.clone(),
                    pds_step_id,
                    product_definitional: df.product_definitional,
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
        for tzf in pmi.tolerance_zone_forms.iter() {
            let _ = ToleranceZoneFormHandler::write(self, tzf.clone());
        }
        for tq in pmi.type_qualifiers.iter() {
            let _ = TypeQualifierHandler::write(self, tq.clone());
        }
        for vftq in pmi.value_format_type_qualifiers.iter() {
            let _ = ValueFormatTypeQualifierHandler::write(self, vftq.clone());
        }
        for font in pmi.draughting_pre_defined_text_fonts.iter() {
            let _ = DraughtingPreDefinedTextFontHandler::write(self, font.clone());
        }
    }

    /// Emit the `annotation_occurrence` arena (`ANNOTATION_PLANE` /
    /// `TESSELLATED_ANNOTATION_OCCURRENCE`). Called after
    /// `emit_visualization_if_set` (`styles` → `psa_step_ids`) and after
    /// `emit_tessellation` (`TessellatedAnnotationOccurrence.item` →
    /// `tessellated_item_step_ids`).
    pub(in crate::writer::buffer) fn emit_annotation_occurrences(&mut self) {
        use crate::entities::pmi::{
            AnnotationPlaneHandler, TessellatedAnnotationOccurrenceHandler,
        };
        use crate::ir::pmi::AnnotationOccurrence;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        for ao in pmi.annotation_occurrences.iter() {
            match ao {
                AnnotationOccurrence::AnnotationPlane(ap) => {
                    let _ = AnnotationPlaneHandler::write(self, ap.clone());
                }
                AnnotationOccurrence::TessellatedAnnotationOccurrence(tao) => {
                    let _ = TessellatedAnnotationOccurrenceHandler::write(self, tao.clone());
                }
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
        for ds in pmi.dimensional_sizes.iter() {
            DimensionalSizeHandler::write(self, ds.clone())?;
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
        for dl in pmi.dimensional_locations.iter() {
            DimensionalLocationHandler::write(self, dl.clone())?;
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
        for gt in pmi.geometric_tolerances.iter() {
            write_geometric_tolerance(self, gt.clone());
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
}
