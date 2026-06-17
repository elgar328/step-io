//! `GLOBAL_UNIT_ASSIGNED_CONTEXT` handler orchestrator.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{UnitContext, UnitContextForm};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::{step_entity, step_entity_complex};

pub(crate) struct GlobalUnitAssignedContextHandler;

#[step_entity_complex(name = "GLOBAL_UNIT_ASSIGNED_CONTEXT", cases = [
    ["GEOMETRIC_REPRESENTATION_CONTEXT", "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT", "GLOBAL_UNIT_ASSIGNED_CONTEXT", "REPRESENTATION_CONTEXT"],
    // No-uncertainty form (GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT is optional).
    // Absent from the round-trip corpus, present in older / FreeCAD fixtures.
    ["GEOMETRIC_REPRESENTATION_CONTEXT", "GLOBAL_UNIT_ASSIGNED_CONTEXT", "REPRESENTATION_CONTEXT"],
])]
impl ComplexEntityHandler for GlobalUnitAssignedContextHandler {
    type WriteInput = UnitContext;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_global_unit_assigned_context(entity_id, parts)?;
        lower::lower_global_unit_assigned_context(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, units: UnitContext) -> Result<u64, WriteError> {
        // A simple GUAC carries no geometric / uncertainty parts — emit the
        // standalone `GLOBAL_UNIT_ASSIGNED_CONTEXT(identifier, type, units)`
        // entity by hand, reproducing its `representation_context` strings.
        // (The simple form is a non-complex instance; only the complex MI is
        // codegen'd.) Leaf units are emitted by `emit_units_pool_if_set`.
        if let UnitContextForm::Simple {
            identifier,
            context_type,
        } = &units.form
        {
            let unit_steps: Vec<u64> = units.units.iter().map(|id| buf.step_id(id)).collect();
            return Ok(buf.push_simple(
                "GLOBAL_UNIT_ASSIGNED_CONTEXT",
                vec![
                    Attribute::String(identifier.clone()),
                    Attribute::String(context_type.clone()),
                    Attribute::List(
                        unit_steps
                            .iter()
                            .map(|&s| Attribute::EntityRef(s))
                            .collect(),
                    ),
                ],
            ));
        }

        // Complex form: lift to the generated L1 (case selection + step-id
        // resolution) and emit via the generated multi-case serialize.
        let early = lift::lift_global_unit_assigned_context(buf, &units);
        Ok(serialize::serialize_global_unit_assigned_context(
            buf, &early,
        ))
    }
}

/// Standalone simple `GLOBAL_UNIT_ASSIGNED_CONTEXT(identifier, type, units)`
/// entity (c3d kernel) — a property's unit context with no geometric /
/// uncertainty parts. `GLOBAL_UNIT_ASSIGNED_CONTEXT SUBTYPE OF
/// (representation_context)` carries the two inherited `representation_context`
/// strings ahead of `units`. Shares the `units` arena with the complex handler
/// via [`UnitContextForm::Simple`]; emission flows through the unified
/// `emit_unit_context` -> [`GlobalUnitAssignedContextHandler::write`] path,
/// which branches on the form (so this handler's `write` is never reached).
pub(crate) struct GlobalUnitAssignedContextSimpleHandler;

#[step_entity(name = "GLOBAL_UNIT_ASSIGNED_CONTEXT")]
impl SimpleEntityHandler for GlobalUnitAssignedContextSimpleHandler {
    type WriteInput = ();

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "GLOBAL_UNIT_ASSIGNED_CONTEXT")?;
        let identifier =
            read_string_or_unset(attrs, 0, entity_id, "context_identifier")?.to_owned();
        let context_type = read_string_or_unset(attrs, 1, entity_id, "context_type")?.to_owned();
        let unit_refs = read_entity_ref_list(attrs, 2, entity_id, "units")?;
        let units = lower::resolve_unit_refs(ctx, entity_id, &unit_refs);
        // The simple form has no GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT part.
        let ctx_id = ctx.units.push(UnitContext {
            units,
            uncertainty: Vec::new(),
            form: UnitContextForm::Simple {
                identifier,
                context_type,
            },
        });
        ctx.context_id_map.insert(entity_id, ctx_id);
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _input: ()) -> Result<u64, WriteError> {
        // Simple GUACs are emitted through the unified `emit_unit_context`
        // path, which dispatches on `UnitContextForm` in the complex handler's
        // `write`. This dispatch handler's `write` is never invoked.
        unreachable!("simple GUAC is emitted via emit_unit_context form branch")
    }
}
