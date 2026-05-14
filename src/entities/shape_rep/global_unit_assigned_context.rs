//! `GLOBAL_UNIT_ASSIGNED_CONTEXT` handler — Pass 0-2 orchestrator.

use crate::entities::units::length_unit::LengthUnitHandler;
use crate::entities::units::plane_angle_unit::PlaneAngleUnitHandler;
use crate::entities::units::solid_angle_unit::SolidAngleUnitHandler;
use crate::entities::units::uncertainty_measure_with_unit::UncertaintyMeasureWithUnitHandler;
use crate::entities::{
    ComplexEntityHandler, ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind,
    SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::UnitContext;
use crate::parser::entity::{Attribute, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct GlobalUnitAssignedContextHandler;

impl ComplexEntityHandler for GlobalUnitAssignedContextHandler {
    const NAME: &'static str = "GLOBAL_UNIT_ASSIGNED_CONTEXT";
    const PASS_LEVEL: PassLevel = PassLevel::Pass0Context;
    const REQUIRED_PARTS: &'static [&'static str] = &["GLOBAL_UNIT_ASSIGNED_CONTEXT"];
    type WriteInput = UnitContext;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
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

        match (length, plane_angle, solid_angle) {
            (Some(length), Some(plane_angle), Some(solid_angle)) => {
                let length_uncertainty = extract_length_uncertainty(ctx, parts);
                let ctx_id = ctx.units.push(UnitContext {
                    length,
                    plane_angle,
                    solid_angle,
                    length_uncertainty,
                    length_cbu_wrapped: ctx.length_cbu_wrapped,
                    plane_angle_cbu_wrapped: ctx.plane_angle_cbu_wrapped,
                    dim_exp_explicit: ctx.dim_exp_explicit,
                });
                ctx.context_id_map.insert(entity_id, ctx_id);
            }
            _ => {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "incomplete unit context: length={}, plane_angle={}, solid_angle={}",
                        length.is_some(),
                        plane_angle.is_some(),
                        solid_angle.is_some(),
                    ),
                });
            }
        }
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, units: UnitContext) -> Result<u64, WriteError> {
        // No top-level cache — `emit_all` calls this once per
        // `UnitContext` arena entry. The length / angle / solid_angle
        // leaf caches still dedup the underlying unit references across
        // contexts that share leaves (the common case for Fusion 360,
        // where two contexts share #1034..#1036).
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
        if let Some(value) = units.length_uncertainty {
            let unc = UncertaintyMeasureWithUnitHandler::write(buf, (value, length))?;
            parts.insert(
                1,
                (
                    "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT".into(),
                    vec![Attribute::List(vec![Attribute::EntityRef(unc)])],
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

/// Pick the first length-flavour uncertainty value referenced by the
/// `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT` part of the same complex
/// entity, if any. Returns `None` when the part is absent or holds no
/// length uncertainty.
fn extract_length_uncertainty(ctx: &ReaderContext, parts: &[RawEntityPart]) -> Option<f64> {
    let guac = parts
        .iter()
        .find(|p| p.name == "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT")?;
    let refs = read_entity_ref_list(&guac.attributes, 0, 0, "uncertainty").ok()?;
    refs.iter()
        .find_map(|r| ctx.length_uncertainty_map.get(r).copied())
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static GLOBAL_UNIT_ASSIGNED_CONTEXT_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: GlobalUnitAssignedContextHandler::NAME,
    pass_level: GlobalUnitAssignedContextHandler::PASS_LEVEL,
    kind: ReadKind::Complex {
        required_parts: GlobalUnitAssignedContextHandler::REQUIRED_PARTS,
        read: GlobalUnitAssignedContextHandler::read_complex,
    },
};
