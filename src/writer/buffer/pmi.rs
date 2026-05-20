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
