//! `GLOBAL_UNIT_ASSIGNED_CONTEXT` handler — Pass 0-2 orchestrator.

use crate::entities::units::uncertainty_measure_with_unit::UncertaintyMeasureWithUnitHandler;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::attr::{check_count, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{AngleUnit, LengthUncertainty, LengthUnit, SolidAngleUnit, UnitContext};
use crate::ir::units::{LengthFlavor, NamedUnit, PlaneAngleFlavor, SolidAngleFlavor};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct GlobalUnitAssignedContextHandler;

#[step_entity_complex(name = "GLOBAL_UNIT_ASSIGNED_CONTEXT", pass = Pass0Context, cases = [
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

        // Walk each ref — variant-classify via the already-populated
        // named_unit arena (units-2: every LENGTH/PLANE_ANGLE/SOLID_ANGLE
        // complex registers a NamedUnit entry; this loop picks the entry
        // that matches each GUAC ref).
        let mut length = None;
        let mut plane_angle = None;
        let mut solid_angle = None;
        for r in &unit_refs {
            if let Some(&nu_id) = ctx.named_unit_id_map.get(r) {
                match ctx.named_units_arena[nu_id] {
                    NamedUnit::Length(_) => length = Some(nu_id),
                    NamedUnit::PlaneAngle(_) => plane_angle = Some(nu_id),
                    NamedUnit::SolidAngle(_) => solid_angle = Some(nu_id),
                    // GUAC does not reference mass or ratio.
                    NamedUnit::Mass(_) | NamedUnit::Ratio(_) => {}
                }
            }
        }

        // Fallback for incomplete unit contexts: synthesize missing slots
        // with SI defaults so `UnitContext` always has a valid arena ref.
        let incomplete = length.is_none() || plane_angle.is_none() || solid_angle.is_none();
        let length = length.unwrap_or_else(|| {
            ctx.named_units_arena.push(NamedUnit::Length(LengthFlavor {
                unit: LengthUnit::Millimetre,
                cbu_base: None,
                dim_exp: None,
            }))
        });
        let plane_angle = plane_angle.unwrap_or_else(|| {
            ctx.named_units_arena
                .push(NamedUnit::PlaneAngle(PlaneAngleFlavor {
                    unit: AngleUnit::Radian,
                    cbu_base: None,
                    dim_exp: None,
                }))
        });
        let solid_angle = solid_angle.unwrap_or_else(|| {
            ctx.named_units_arena
                .push(NamedUnit::SolidAngle(SolidAngleFlavor {
                    unit: SolidAngleUnit::Steradian,
                    dim_exp: None,
                }))
        });
        if incomplete {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: "incomplete unit context (synthesized missing components as SI defaults)"
                    .into(),
            });
        }
        let (length_uncertainty, plane_angle_uncertainty, solid_angle_uncertainty) =
            extract_uncertainties(ctx, parts);
        let ctx_id = ctx.units.push(UnitContext {
            length,
            plane_angle,
            solid_angle,
            length_uncertainty,
            plane_angle_uncertainty,
            solid_angle_uncertainty,
        });
        ctx.context_id_map.insert(entity_id, ctx_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, units: UnitContext) -> Result<u64, WriteError> {
        // units-2: leaf entities are emitted once by `emit_units_pool_if_set`
        // before `emit_all` reaches the GUAC loop. Here we resolve each
        // `NamedUnitId` to the already-emitted step id via the cache.
        let length = buf.named_unit_step_ids[units.length.0 as usize];
        let angle = buf.named_unit_step_ids[units.plane_angle.0 as usize];
        let solid = buf.named_unit_step_ids[units.solid_angle.0 as usize];
        buf.unit_leaf_ids.push((length, angle, solid));

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
                vec![Attribute::List(vec![
                    Attribute::EntityRef(length),
                    Attribute::EntityRef(angle),
                    Attribute::EntityRef(solid),
                ])],
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
        if let Some(uncertainty) = units.length_uncertainty.clone() {
            let id = UncertaintyMeasureWithUnitHandler::write(
                buf,
                (uncertainty, length, "LENGTH_MEASURE"),
            )?;
            unc_refs.push(Attribute::EntityRef(id));
        }
        if let Some(uncertainty) = units.plane_angle_uncertainty.clone() {
            let id = UncertaintyMeasureWithUnitHandler::write(
                buf,
                (uncertainty, angle, "PLANE_ANGLE_MEASURE"),
            )?;
            unc_refs.push(Attribute::EntityRef(id));
        }
        if let Some(uncertainty) = units.solid_angle_uncertainty.clone() {
            let id = UncertaintyMeasureWithUnitHandler::write(
                buf,
                (uncertainty, solid, "SOLID_ANGLE_MEASURE"),
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
