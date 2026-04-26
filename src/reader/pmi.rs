//! PMI entity converters (Pass 8).
//!
//! Currently a single converter — `convert_shape_aspect` — that resolves
//! `SHAPE_ASPECT.of_shape` through the assembly pass's `pdef_shape_to_pdef`
//! and `pdef_to_product` maps to a `ProductId`. Future PMI work
//! (Tolerance / Datum / GD&T per ROADMAP Phase 2) hangs additional
//! converters off the same pass.

use super::ReaderContext;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::pmi::{PmiPool, ShapeAspect};
use crate::parser::entity::Attribute;

impl ReaderContext {
    pub(super) fn convert_shape_aspect(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "SHAPE_ASPECT")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let of_shape_ref = read_entity_ref(attrs, 2, entity_id, "of_shape")?;
        let product_definitional = read_bool(attrs, 3, entity_id, "product_definitional")?;

        // Lookup chain: SHAPE_ASPECT.of_shape → PRODUCT_DEFINITION_SHAPE
        //   → PRODUCT_DEFINITION → ProductId
        let Some(&pdef_step_id) = self.pdef_shape_to_pdef.get(&of_shape_ref) else {
            return Ok(()); // unresolved (rare — non-PDS targets)
        };
        let Some(&product_step_id) = self.pdef_to_product.get(&pdef_step_id) else {
            return Ok(());
        };
        let Some(&product_id) = self.product_arena_map.get(&product_step_id) else {
            return Ok(());
        };

        let pmi = self.pmi.get_or_insert_with(PmiPool::default);
        pmi.shape_aspects.push(ShapeAspect {
            name,
            description,
            target: product_id,
            product_definitional,
        });
        Ok(())
    }
}
