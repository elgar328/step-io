//! `(DRAUGHTING_MODEL REPRESENTATION SHAPE_REPRESENTATION TESSELLATED_SHAPE_REPRESENTATION)`
//! complex-MI handler — phase dm-rep-tsr-complex.
//!
//! The geometric-validation draughting model that PMI
//! `CHARACTERIZED_ITEM_WITHIN_REPRESENTATION`s reference as their `rep`. No
//! prior handler matched this four-part set, so it was silently skipped and
//! every referencing CIWR (with its `PROPERTY_DEFINITION` + measures)
//! dropped. Reads the `REPRESENTATION` part into a
//! `Representation::DraughtingModel` and registers `repr_id_map` so the CIWR
//! pass (scheduled later) resolves its `rep`. Writing is handled by
//! `DraughtingModelHandler::write` (the `ShapeTessellated` form arm), so this
//! handler is read-only.

use crate::entities::ComplexEntityHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{read_entity_ref_list, read_optional_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{
    DraughtingModel, DraughtingModelForm, Representation, RepresentationContextRef,
};
use crate::parser::entity::{EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct DraughtingModelStComplexHandler;

#[step_entity_complex(
    name = "DRAUGHTING_MODEL",
    pass = Pass8DraughtingModelStComplex,
    required = [
        "DRAUGHTING_MODEL",
        "REPRESENTATION",
        "SHAPE_REPRESENTATION",
        "TESSELLATED_SHAPE_REPRESENTATION"
    ]
)]
impl ComplexEntityHandler for DraughtingModelStComplexHandler {
    type WriteInput = ();

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // `name` / `items` / `context_of_items` all live in the REPRESENTATION
        // part; the other three parts carry no data.
        let repr_attrs = require_part_attrs(parts, "REPRESENTATION", entity_id)?;
        let name = read_string_or_unset(repr_attrs, 0, entity_id, "name")?.to_owned();
        let item_refs = read_entity_ref_list(repr_attrs, 1, entity_id, "items")?;
        let ctx_ref_opt = read_optional_entity_ref(repr_attrs, 2, entity_id, "context_of_items")?;
        let context = ctx_ref_opt.and_then(|r| ctx.resolve_repr_context(r));
        // Mirror the simple `DRAUGHTING_MODEL` reader: surface a Unitful
        // context through `repr_context_map` so an SDR/PDR referencing this
        // draughting model can resolve its unit context.
        if let Some(RepresentationContextRef::Unitful(ctx_id)) = context {
            ctx.repr_context_map.insert(entity_id, ctx_id);
        }
        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(item) = resolve_representation_item_ref(ctx, r) {
                items.push(item);
            }
        }
        if items.is_empty() {
            return Ok(());
        }
        let repr_id = ctx
            .representations
            .push(Representation::DraughtingModel(DraughtingModel {
                name,
                items,
                context,
                form: DraughtingModelForm::ShapeTessellated,
            }));
        ctx.repr_id_map.insert(entity_id, repr_id);
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _input: Self::WriteInput) -> Result<u64, WriteError> {
        // All `DraughtingModel` arena entries are emitted by
        // `DraughtingModelHandler::write` (via `emit_draughting_models`);
        // this complex handler only reads.
        unreachable!("ShapeTessellated DraughtingModel is written by DraughtingModelHandler::write")
    }
}
