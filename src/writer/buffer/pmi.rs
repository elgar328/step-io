//! PMI emission — `SHAPE_ASPECT` entries that anchor future Tolerance /
//! Datum / GD&T work. Each `ShapeAspect` re-emits as one `SHAPE_ASPECT`
//! complex entity referencing the cached `PRODUCT_DEFINITION_SHAPE` step
//! id (populated by the assembly emitter).

use super::WriteBuffer;
use crate::ir::shape_rep::ShapeAspect;
use crate::parser::entity::Attribute;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_pmi_if_set(&mut self) {
        // Snapshot to release the &model borrow before per-SA emission
        // (emit_shape_aspect mutates self via push_simple).
        let entries: Vec<ShapeAspect> = self.model.shape_aspects.iter().cloned().collect();
        for sa in &entries {
            self.emit_shape_aspect(sa);
        }
    }

    pub(crate) fn emit_shape_aspect(&mut self, sa: &ShapeAspect) {
        // Defensive: silent skip when product chain wasn't emitted (e.g.
        // kernel-built IR with PMI only, or model.units empty so the
        // assembly pass bailed out). Reader symmetry — reader silent
        // skips SAs whose target ref doesn't resolve.
        let Some(&pds_step_id) = self.product_def_shape_ids.get(&sa.target) else {
            return;
        };
        let bool_attr = if sa.product_definitional { "T" } else { "F" };
        self.push_simple(
            "SHAPE_ASPECT",
            vec![
                Attribute::String(sa.name.clone()),
                Attribute::String(sa.description.clone()),
                Attribute::EntityRef(pds_step_id),
                Attribute::Enum(bool_attr.into()),
            ],
        );
    }
}
