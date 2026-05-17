//! `GLOBAL_UNIT_ASSIGNED_CONTEXT` handler — Pass 0-2 orchestrator.

use crate::entities::units::length_unit::LengthUnitHandler;
use crate::entities::units::plane_angle_unit::PlaneAngleUnitHandler;
use crate::entities::units::solid_angle_unit::SolidAngleUnitHandler;
use crate::entities::units::uncertainty_measure_with_unit::UncertaintyMeasureWithUnitHandler;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::attr::{check_count, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{AngleUnit, LengthUncertainty, LengthUnit, SolidAngleUnit, UnitContext};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct GlobalUnitAssignedContextHandler;

#[step_entity_complex(name = "GLOBAL_UNIT_ASSIGNED_CONTEXT", pass = Pass0Context, required = ["GLOBAL_UNIT_ASSIGNED_CONTEXT"])]
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

        let mut length = None;
        let mut plane_angle = None;
        let mut solid_angle = None;
        for r in &unit_refs {
            if let Some(&u) = ctx.length_unit_map.get(r) {
                length = Some(u);
            } else if let Some(&u) = ctx.angle_unit_map.get(r) {
                plane_angle = Some(u);
            } else if let Some(&u) = ctx.solid_angle_unit_map.get(r) {
                solid_angle = Some(u);
            }
        }

        // Fallback for incomplete unit contexts (unrecognized CBU names,
        // anonymised fixtures, vendor quirks): fill missing components with
        // SI defaults (Millimetre / Radian / Steradian) and emit a warning.
        // Without this fallback, an unrecognised plane-angle CBU name would
        // leave `model.units` empty and the writer's PRODUCT chain emit path
        // would short-circuit, silently losing the entire assembly.
        let (length, plane_angle, solid_angle) = match (length, plane_angle, solid_angle) {
            (Some(l), Some(p), Some(s)) => (l, p, s),
            (l, p, s) => {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "incomplete unit context (defaulting missing to SI): length={}, plane_angle={}, solid_angle={}",
                        l.is_some(),
                        p.is_some(),
                        s.is_some(),
                    ),
                });
                (
                    l.unwrap_or(LengthUnit::Millimetre),
                    p.unwrap_or(AngleUnit::Radian),
                    s.unwrap_or(SolidAngleUnit::Steradian),
                )
            }
        };
        let (length_uncertainty, plane_angle_uncertainty, solid_angle_uncertainty) =
            extract_uncertainties(ctx, parts);
        let ctx_id = ctx.units.push(UnitContext {
            length,
            plane_angle,
            solid_angle,
            length_uncertainty,
            plane_angle_uncertainty,
            solid_angle_uncertainty,
            length_cbu_wrapped: ctx.length_cbu_wrapped,
            plane_angle_cbu_wrapped: ctx.plane_angle_cbu_wrapped,
            dim_exp_explicit: ctx.dim_exp_explicit,
        });
        ctx.context_id_map.insert(entity_id, ctx_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, units: UnitContext) -> Result<u64, WriteError> {
        // No writer-side dedup — `emit_all` calls this once per
        // `UnitContext` arena entry, and each call emits its own leaf
        // entities. The resulting `(length, angle, solid)` tuple is
        // pushed to `unit_leaf_ids` so the property emitter can resolve
        // a measure's unit ref by `UnitContextId` index.
        let length = LengthUnitHandler::write(
            buf,
            (
                units.length,
                units.length_cbu_wrapped,
                units.dim_exp_explicit,
            ),
        )?;
        let angle = PlaneAngleUnitHandler::write(
            buf,
            (
                units.plane_angle,
                units.plane_angle_cbu_wrapped,
                units.dim_exp_explicit,
            ),
        )?;
        let solid = SolidAngleUnitHandler::write(buf, (units.solid_angle, units.dim_exp_explicit))?;
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
