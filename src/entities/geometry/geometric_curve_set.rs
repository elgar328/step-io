//! `GEOMETRIC_CURVE_SET` handler (2-layer path).
//!
//! Hosts the shared child-emit helper that the `GEOMETRIC_SET` sister imports.
//! Both names share the same EXPRESS shape; `GEOMETRIC_CURVE_SET` is a subtype
//! restricting `items` to curves, while `GEOMETRIC_SET` allows points and
//! (rarely) surfaces too. `lower` splits the items into `curves` / `points`
//! buckets; the writer emits curve refs followed by point refs (matching the
//! IR's `(curves, points)` split) under the requested entity name.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::id::{CurveId, PointId};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CurveSetWriteInput {
    pub(crate) curves: Vec<CurveId>,
    pub(crate) points: Vec<PointId>,
}

/// Shared child-emit: curve refs followed by point refs (the `(curves, points)`
/// IR split). Used by both `GEOMETRIC_CURVE_SET` and `GEOMETRIC_SET` writers.
pub(crate) fn emit_curve_set_elements(
    buf: &mut WriteBuffer,
    input: CurveSetWriteInput,
) -> Result<Vec<u64>, WriteError> {
    let CurveSetWriteInput { curves, points } = input;
    let mut elements = Vec::with_capacity(curves.len() + points.len());
    for cid in curves {
        elements.push(buf.emit_curve(cid)?);
    }
    for pid in points {
        elements.push(buf.emit_point(pid)?);
    }
    Ok(elements)
}

pub(crate) struct GeometricCurveSetHandler;

#[step_entity(name = "GEOMETRIC_CURVE_SET")]
impl SimpleEntityHandler for GeometricCurveSetHandler {
    type WriteInput = CurveSetWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_geometric_curve_set(entity_id, attrs)?;
        lower::lower_geometric_curve_set(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, input: CurveSetWriteInput) -> Result<u64, WriteError> {
        let elements = emit_curve_set_elements(buf, input)?;
        let early = lift::lift_geometric_curve_set(elements);
        Ok(serialize::serialize_geometric_curve_set(buf, &early))
    }
}
