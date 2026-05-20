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
use crate::ir::shape_rep::ShapeAspect;

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
        self.emit_pmi_pool();
    }

    /// Emit the `pmi` pool's `DATUM` arena. Each datum resolves its
    /// `target` (`ProductId`) to a `PRODUCT_DEFINITION_SHAPE` step id via
    /// `product_def_shape_ids` — the same chain as `SHAPE_ASPECT`. Silent
    /// skip when the product chain was not emitted (reader symmetry).
    fn emit_datums(&mut self) {
        use crate::entities::pmi::{DatumHandler, DatumWriteInput};
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        for datum in pmi.datums.iter() {
            let Some(&pds_step_id) = self.product_def_shape_ids.get(&datum.target) else {
                continue;
            };
            let _ = DatumHandler::write(
                self,
                DatumWriteInput {
                    name: datum.name.clone(),
                    description: datum.description.clone(),
                    pds_step_id,
                    product_definitional: datum.product_definitional,
                    identification: datum.identification.clone(),
                },
            );
        }
    }

    /// Emit the `pmi` pool's leaf primitives (`TOLERANCE_ZONE_FORM` /
    /// `TYPE_QUALIFIER` / `VALUE_FORMAT_TYPE_QUALIFIER`). No cross-refs —
    /// each is a standalone 1-attr entity.
    fn emit_pmi_pool(&mut self) {
        use crate::entities::pmi::{
            ToleranceZoneFormHandler, TypeQualifierHandler, ValueFormatTypeQualifierHandler,
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
    }

    /// Emit the `annotation_occurrence` arena (`ANNOTATION_PLANE` only).
    /// Called after `emit_visualization_if_set` — `AnnotationPlane.styles`
    /// resolves through `psa_step_ids`, which the visualization pass fills.
    /// Separate from [`Self::emit_pmi_if_set`] (which runs before
    /// visualization) for that ordering reason.
    pub(in crate::writer::buffer) fn emit_annotation_occurrences(&mut self) {
        use crate::entities::pmi::AnnotationPlaneHandler;
        use crate::ir::pmi::AnnotationOccurrence;
        let Some(pmi) = self.model.pmi.clone() else {
            return;
        };
        for ao in pmi.annotation_occurrences.iter() {
            match ao {
                AnnotationOccurrence::AnnotationPlane(ap) => {
                    let _ = AnnotationPlaneHandler::write(self, ap.clone());
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
        // (handler.write mutates self via push_simple).
        let composite: Vec<_> = self
            .model
            .composite_group_shape_aspects
            .iter()
            .cloned()
            .collect();
        for sa in composite {
            let Some(&pds_step_id) = self.product_def_shape_ids.get(&sa.target) else {
                continue;
            };
            let _ = CompositeGroupShapeAspectHandler::write(
                self,
                ShapeAspectSubtypeWriteInput {
                    name: sa.name,
                    description: sa.description,
                    pds_step_id,
                    product_definitional: sa.product_definitional,
                },
            );
        }
        let centre: Vec<_> = self.model.centre_of_symmetries.iter().cloned().collect();
        for sa in centre {
            let Some(&pds_step_id) = self.product_def_shape_ids.get(&sa.target) else {
                continue;
            };
            let _ = CentreOfSymmetryHandler::write(
                self,
                ShapeAspectSubtypeWriteInput {
                    name: sa.name,
                    description: sa.description,
                    pds_step_id,
                    product_definitional: sa.product_definitional,
                },
            );
        }
        let all_around: Vec<_> = self
            .model
            .all_around_shape_aspects
            .iter()
            .cloned()
            .collect();
        for sa in all_around {
            let Some(&pds_step_id) = self.product_def_shape_ids.get(&sa.target) else {
                continue;
            };
            let _ = AllAroundShapeAspectHandler::write(
                self,
                ShapeAspectSubtypeWriteInput {
                    name: sa.name,
                    description: sa.description,
                    pds_step_id,
                    product_definitional: sa.product_definitional,
                },
            );
        }
    }
}
