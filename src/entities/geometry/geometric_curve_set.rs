//! `GEOMETRIC_CURVE_SET` handler — Pass 6-4f.
//!
//! Hosts the shared reader/writer body that the `GEOMETRIC_SET` sister
//! imports. Both names share the same EXPRESS shape; `GEOMETRIC_CURVE_SET`
//! is a subtype restricting `items` to curves, while `GEOMETRIC_SET` allows
//! points and (rarely) surfaces too. Reader splits the items into `curves`
//! and `points` buckets; writer emits a single line under the requested
//! entity name with the IR's curve / point ids inlined as entity refs.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::id::{CurveId, PointId};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct CurveSetWriteInput {
    pub(crate) curves: Vec<CurveId>,
    pub(crate) points: Vec<PointId>,
}

/// Shared reader body for `GEOMETRIC_CURVE_SET` and `GEOMETRIC_SET`.
/// `entity_name` is the source entity name used for diagnostics; the
/// behaviour is identical for both — items resolve through `curve_map` /
/// `point_map`, anything else is silently skipped (e.g. stray surface refs).
pub(crate) fn read_geometric_curve_set_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    entity_name: &'static str,
) -> Result<(), ConvertError> {
    check_count(attrs, 2, entity_id, entity_name)?;
    let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
    let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
    let mut curves = Vec::new();
    let mut points = Vec::new();
    for r in item_refs {
        if let Some(&cid) = ctx.curve_map.get(&r) {
            curves.push(cid);
        } else if let Some(&pid) = ctx.point_map.get(&r) {
            points.push(pid);
        }
    }
    ctx.curve_set_map.insert(entity_id, (curves, points));
    Ok(())
}

/// Shared writer body. Emits the requested entity name with a list of
/// curve refs followed by point refs (matching the `(curves, points)`
/// IR split). Caller picks the entity name based on whether loose points
/// are present (the writer's existing convention).
pub(crate) fn write_geometric_curve_set(
    buf: &mut WriteBuffer,
    entity_name: &'static str,
    input: CurveSetWriteInput,
) -> Result<u64, WriteError> {
    let CurveSetWriteInput { curves, points } = input;
    let mut item_refs = Vec::with_capacity(curves.len() + points.len());
    for cid in curves {
        item_refs.push(Attribute::EntityRef(buf.emit_curve(cid)?));
    }
    for pid in points {
        item_refs.push(Attribute::EntityRef(buf.emit_point(pid)?));
    }
    Ok(buf.push_simple(
        entity_name,
        vec![Attribute::String(String::new()), Attribute::List(item_refs)],
    ))
}

pub(crate) struct GeometricCurveSetHandler;

impl SimpleEntityHandler for GeometricCurveSetHandler {
    const NAME: &'static str = "GEOMETRIC_CURVE_SET";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6CurveSet;
    type WriteInput = CurveSetWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_geometric_curve_set_body(ctx, entity_id, attrs, "GEOMETRIC_CURVE_SET")
    }

    fn write(buf: &mut WriteBuffer, input: CurveSetWriteInput) -> Result<u64, WriteError> {
        write_geometric_curve_set(buf, "GEOMETRIC_CURVE_SET", input)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static GCS_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: GeometricCurveSetHandler::NAME,
    pass_level: GeometricCurveSetHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: GeometricCurveSetHandler::read,
    },
};
