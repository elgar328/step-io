//! `(GRC PRC REP_CONTEXT)` unitless context handler (`ComplexEntityHandler`,
//! 2-layer path). Matches the complex MI instance of
//! `GEOMETRIC_REPRESENTATION_CONTEXT` + `PARAMETRIC_REPRESENTATION_CONTEXT` +
//! `REPRESENTATION_CONTEXT` (a `GLOBAL_UNIT_ASSIGNED_CONTEXT` part would form a
//! different exact part-set claimed by the GUAC handler, so it never reaches
//! here). L2 = `UnitlessContext`; the writer's `emit_unitless_contexts` pass
//! routes the dimension-present (parametric) form to this `write`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::ComplexEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::UnitlessContext;
use crate::parser::entity::{EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct ParametricRepresentationContextHandler;

#[step_entity_complex(
    name = "PARAMETRIC_REPRESENTATION_CONTEXT",
    cases = [["GEOMETRIC_REPRESENTATION_CONTEXT", "PARAMETRIC_REPRESENTATION_CONTEXT", "REPRESENTATION_CONTEXT"]]
)]
impl ComplexEntityHandler for ParametricRepresentationContextHandler {
    type WriteInput = UnitlessContext;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_parametric_representation_context(entity_id, parts)?;
        lower::lower_parametric_representation_context(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, uc: UnitlessContext) -> Result<u64, WriteError> {
        let early = lift::lift_parametric_representation_context(uc);
        Ok(serialize::serialize_parametric_representation_context(
            buf, &early,
        ))
    }
}
