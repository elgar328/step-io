//! `GLOBAL_UNIT_ASSIGNED_CONTEXT` handler orchestrator.

use crate::entities::units::uncertainty_measure_with_unit::UncertaintyMeasureWithUnitHandler;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::attr::{check_count, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{LengthUncertainty, UnitContext};
use crate::ir::units::NamedUnit;
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

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
        let guac_attrs = require_part_attrs(parts, "GLOBAL_UNIT_ASSIGNED_CONTEXT", entity_id)?;
        check_count(guac_attrs, 1, entity_id, "GLOBAL_UNIT_ASSIGNED_CONTEXT")?;
        let unit_refs = read_entity_ref_list(guac_attrs, 0, entity_id, "units")?;

        // `units` is `SET[1:?] OF unit` — collect each ref (any kind: length /
        // plane_angle / solid_angle / mass / ratio, in source order) into the
        // set. A source may legitimately omit a kind (e.g. solid_angle); no
        // synthesis, no "incomplete" warning. An unresolved ref (unit step-io
        // did not model) is surfaced and skipped.
        let mut units = Vec::with_capacity(unit_refs.len());
        for r in &unit_refs {
            if let Some(&nu_id) = ctx.named_unit_id_map.get(r) {
                units.push(nu_id);
            } else {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("GLOBAL_UNIT_ASSIGNED_CONTEXT.units ref #{r} unresolved"),
                });
            }
        }
        let (length_uncertainty, plane_angle_uncertainty, solid_angle_uncertainty) =
            extract_uncertainties(ctx, parts);
        let ctx_id = ctx.units.push(UnitContext {
            units,
            length_uncertainty,
            plane_angle_uncertainty,
            solid_angle_uncertainty,
        });
        ctx.context_id_map.insert(entity_id, ctx_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, units: UnitContext) -> Result<u64, WriteError> {
        // units-2: leaf entities are emitted once by `emit_units_pool_if_set`
        // before `emit_all` reaches the GUAC loop. Resolve each `NamedUnitId`
        // to the emitted step id, preserving the source `units` set order.
        let unit_steps: Vec<u64> = units
            .units
            .iter()
            .map(|id| buf.named_unit_step_ids[id.0 as usize])
            .collect();

        // For uncertainty binding, find the step id of the first unit of a
        // given kind in `units` (a kind may be absent — the schema permits it,
        // and an uncertainty with no matching unit kind is dropped). Resolved
        // before the mutable uncertainty emits below (immutable borrow).
        let kind_step = |want: fn(&NamedUnit) -> bool| -> Option<u64> {
            let pool = buf.model.units_pool.as_ref()?;
            units
                .units
                .iter()
                .find(|id| want(&pool.named_units[**id]))
                .map(|id| buf.named_unit_step_ids[id.0 as usize])
        };
        let length_step = kind_step(|u| matches!(u, NamedUnit::Length(_)));
        let angle_step = kind_step(|u| matches!(u, NamedUnit::PlaneAngle(_)));
        let solid_step = kind_step(|u| matches!(u, NamedUnit::SolidAngle(_)));

        // ISO 10303-21:2016 §11.2.5.1 — complex entity parts serialize in
        // alphabetical order. Final order with uncertainty present:
        //   GEOMETRIC_REPRESENTATION_CONTEXT
        //   GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT   (UNCERTAINTY < UNIT)
        //   GLOBAL_UNIT_ASSIGNED_CONTEXT
        //   REPRESENTATION_CONTEXT
        let mut parts = vec![
            (
                "GEOMETRIC_REPRESENTATION_CONTEXT".into(),
                vec![Attribute::Integer(3)],
            ),
            (
                "GLOBAL_UNIT_ASSIGNED_CONTEXT".into(),
                vec![Attribute::List(
                    unit_steps
                        .iter()
                        .map(|&s| Attribute::EntityRef(s))
                        .collect(),
                )],
            ),
            (
                "REPRESENTATION_CONTEXT".into(),
                vec![
                    Attribute::String(String::new()),
                    Attribute::String(String::new()),
                ],
            ),
        ];
        let mut unc_refs: Vec<Attribute> = Vec::new();
        // Emit each uncertainty only when its corresponding unit kind is
        // present in the set (cannot reference a unit that was not emitted).
        if let (Some(uncertainty), Some(unit_step)) =
            (units.length_uncertainty.clone(), length_step)
        {
            let id = UncertaintyMeasureWithUnitHandler::write(
                buf,
                (uncertainty, unit_step, "LENGTH_MEASURE"),
            )?;
            unc_refs.push(Attribute::EntityRef(id));
        }
        if let (Some(uncertainty), Some(unit_step)) =
            (units.plane_angle_uncertainty.clone(), angle_step)
        {
            let id = UncertaintyMeasureWithUnitHandler::write(
                buf,
                (uncertainty, unit_step, "PLANE_ANGLE_MEASURE"),
            )?;
            unc_refs.push(Attribute::EntityRef(id));
        }
        if let (Some(uncertainty), Some(unit_step)) =
            (units.solid_angle_uncertainty.clone(), solid_step)
        {
            let id = UncertaintyMeasureWithUnitHandler::write(
                buf,
                (uncertainty, unit_step, "SOLID_ANGLE_MEASURE"),
            )?;
            unc_refs.push(Attribute::EntityRef(id));
        }
        if !unc_refs.is_empty() {
            parts.insert(
                1,
                (
                    "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT".into(),
                    vec![Attribute::List(unc_refs)],
                ),
            );
        }

        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex { parts },
        });
        Ok(n)
    }
}

/// Collect the length / plane-angle / solid-angle uncertainty entries
/// referenced by the `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT` part of the
/// same complex entity. Each component is `Some` if any referenced
/// `UNCERTAINTY_MEASURE_WITH_UNIT` resolved to a unit of that kind on
/// read; `None` otherwise. Order of refs within the source does not
/// matter — first match per category wins.
type UncertaintyTriple = (
    Option<LengthUncertainty>,
    Option<LengthUncertainty>,
    Option<LengthUncertainty>,
);

fn extract_uncertainties(ctx: &ReaderContext, parts: &[RawEntityPart]) -> UncertaintyTriple {
    let Some(guac) = parts
        .iter()
        .find(|p| p.name == "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT")
    else {
        return (None, None, None);
    };
    let Ok(refs) = read_entity_ref_list(&guac.attributes, 0, 0, "uncertainty") else {
        return (None, None, None);
    };
    let mut length = None;
    let mut plane_angle = None;
    let mut solid_angle = None;
    for r in &refs {
        if length.is_none() {
            if let Some(u) = ctx.length_uncertainty_map.get(r) {
                length = Some(u.clone());
                continue;
            }
        }
        if plane_angle.is_none() {
            if let Some(u) = ctx.plane_angle_uncertainty_map.get(r) {
                plane_angle = Some(u.clone());
                continue;
            }
        }
        if solid_angle.is_none() {
            if let Some(u) = ctx.solid_angle_uncertainty_map.get(r) {
                solid_angle = Some(u.clone());
            }
        }
    }
    (length, plane_angle, solid_angle)
}
