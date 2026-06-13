//! `MODEL_GEOMETRIC_VIEW` handler — shape-rep domain (2-layer path).
//! Standalone arena emit goes through `emit_characterized_objects` (under a
//! reserved id, via `serialize_model_geometric_view_with_id`); this trait
//! `write` mirrors the same shape for completeness.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::ModelGeometricView;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ModelGeometricViewHandler;

#[step_entity(name = "MODEL_GEOMETRIC_VIEW")]
impl SimpleEntityHandler for ModelGeometricViewHandler {
    type WriteInput = ModelGeometricView;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_model_geometric_view(entity_id, attrs)?;
        lower::lower_model_geometric_view(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, mgv: ModelGeometricView) -> Result<u64, WriteError> {
        let item_step = buf.step_id(mgv.item);
        let rep_step = buf.step_id(mgv.rep);
        let early = lift::lift_model_geometric_view(
            mgv.inherited.name,
            mgv.inherited.description,
            item_step,
            rep_step,
        );
        Ok(serialize::serialize_model_geometric_view(buf, &early))
    }
}
