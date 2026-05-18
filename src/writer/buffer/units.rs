//! Unit context emission. Plan 5.6 C4 lifted the entire emit chain
//! into `entities/units/global_unit_assigned_context.rs` (the
//! orchestrator) plus the three leaf handlers and the
//! `UncertaintyMeasureWithUnit` handler. This file remains as a 1-line
//! dispatcher so `emit_all` keeps a stable entry point — analogous to
//! the `emit_face` / `emit_curve` wrappers in geometry / topology.

use super::WriteBuffer;
use crate::ir::{MeasureWithUnit, NamedUnit, UnitContext};
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_unit_context(
        &mut self,
        units: UnitContext,
    ) -> Result<u64, WriteError> {
        // Plan 5.6 stage C4: dispatch through EntityHandler trait. Body
        // lives in
        // `src/entities/shape_rep/global_unit_assigned_context.rs`.
        use crate::entities::ComplexEntityHandler;
        crate::entities::shape_rep::global_unit_assigned_context::GlobalUnitAssignedContextHandler::write(self, units)
    }

    /// units-1: emit the [`UnitsPool`] arenas (per-instance `NamedUnit`,
    /// `MeasureWithUnit`, `DerivedUnitElement`) and populate the three
    /// step-id caches.
    ///
    /// These are emitted in addition to (not in place of) the existing
    /// `UnitContext` leaves — the dual-tracking period. The extra
    /// `NAMED_UNIT` complexes are not referenced by GUAC; they exist only
    /// so MWU / DUE arena emits have well-formed `unit` refs. See the
    /// units-1 plan for the round-trip-fidelity trade-off note.
    ///
    /// [`UnitsPool`]: crate::ir::UnitsPool
    pub(crate) fn emit_units_pool_if_set(&mut self) -> Result<(), WriteError> {
        let Some(pool) = self.model.units_pool.as_ref() else {
            return Ok(());
        };
        // Pre-size caches so writers can index without `Vec::push` order
        // worries. We then walk arenas in id order and assign.
        self.named_unit_step_ids.resize(pool.named_units.len(), 0);
        for (id, named) in pool.named_units.iter_with_ids() {
            let step = emit_named_unit(self, named)?;
            self.named_unit_step_ids[id.0 as usize] = step;
        }
        self.mwu_step_ids.resize(pool.measure_with_units.len(), 0);
        for (id, mwu) in pool.measure_with_units.iter_with_ids() {
            let step = emit_measure_with_unit(self, mwu)?;
            self.mwu_step_ids[id.0 as usize] = step;
        }
        self.due_step_ids
            .resize(pool.derived_unit_elements.len(), 0);
        for (id, due) in pool.derived_unit_elements.iter_with_ids() {
            let unit_step = self.named_unit_step_ids[due.unit.0 as usize];
            let step = emit_derived_unit_element(self, unit_step, due.exponent)?;
            self.due_step_ids[id.0 as usize] = step;
        }
        // units-1b: DERIVED_UNIT wraps DUE refs — emit after the DUE
        // loop so `due_step_ids` is fully populated.
        self.derived_unit_step_ids
            .resize(pool.derived_units.len(), 0);
        for (id, du) in pool.derived_units.iter_with_ids() {
            let element_steps: Vec<u64> = du
                .elements
                .iter()
                .map(|e| self.due_step_ids[e.0 as usize])
                .collect();
            let step = emit_derived_unit(self, element_steps)?;
            self.derived_unit_step_ids[id.0 as usize] = step;
        }
        Ok(())
    }
}

fn emit_derived_unit(buf: &mut WriteBuffer<'_>, elements: Vec<u64>) -> Result<u64, WriteError> {
    use crate::entities::SimpleEntityHandler;
    crate::entities::units::derived_unit::DerivedUnitHandler::write(buf, elements)
}

fn emit_derived_unit_element(
    buf: &mut WriteBuffer<'_>,
    unit_step: u64,
    exponent: f64,
) -> Result<u64, WriteError> {
    use crate::entities::SimpleEntityHandler;
    crate::entities::units::derived_unit_element::DerivedUnitElementHandler::write(
        buf,
        (unit_step, exponent),
    )
}

fn emit_named_unit(buf: &mut WriteBuffer<'_>, named: &NamedUnit) -> Result<u64, WriteError> {
    use crate::entities::ComplexEntityHandler;
    use crate::entities::units::length_unit::LengthUnitHandler;
    use crate::entities::units::mass_unit::MassUnitHandler;
    use crate::entities::units::plane_angle_unit::PlaneAngleUnitHandler;
    use crate::entities::units::solid_angle_unit::SolidAngleUnitHandler;
    // Use the existing leaf writers with no CBU-wrap / no explicit DE.
    // The units-1 NamedUnit arena entries are stand-alone NAMED_UNIT
    // complexes detached from any GUAC; they don't carry the per-context
    // sticky flags the UnitContext path threads through.
    match *named {
        NamedUnit::Length(u) => LengthUnitHandler::write(buf, (u, false, false)),
        NamedUnit::PlaneAngle(u) => PlaneAngleUnitHandler::write(buf, (u, false, false)),
        NamedUnit::SolidAngle(u) => SolidAngleUnitHandler::write(buf, (u, false)),
        NamedUnit::Mass(u) => MassUnitHandler::write(buf, u),
    }
}

fn emit_measure_with_unit(
    buf: &mut WriteBuffer<'_>,
    mwu: &MeasureWithUnit,
) -> Result<u64, WriteError> {
    use crate::entities::SimpleEntityHandler;
    use crate::entities::units::{
        length_measure_with_unit::LengthMeasureWithUnitHandler,
        mass_measure_with_unit::MassMeasureWithUnitHandler,
        plane_angle_measure_with_unit::PlaneAngleMeasureWithUnitHandler,
        ratio_measure_with_unit::RatioMeasureWithUnitHandler,
    };
    match *mwu {
        MeasureWithUnit::Length { value, unit } => {
            let unit_step = buf.named_unit_step_ids[unit.0 as usize];
            LengthMeasureWithUnitHandler::write(buf, (value, unit_step))
        }
        MeasureWithUnit::Mass { value, unit } => {
            let unit_step = buf.named_unit_step_ids[unit.0 as usize];
            MassMeasureWithUnitHandler::write(buf, (value, unit_step))
        }
        MeasureWithUnit::PlaneAngle { value, unit } => {
            let unit_step = buf.named_unit_step_ids[unit.0 as usize];
            PlaneAngleMeasureWithUnitHandler::write(buf, (value, unit_step))
        }
        MeasureWithUnit::Ratio { value, unit } => {
            let unit_step = buf.named_unit_step_ids[unit.0 as usize];
            RatioMeasureWithUnitHandler::write(buf, (value, unit_step))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::arena::Arena;
    use crate::ir::{AngleUnit, LengthUnit, SolidAngleUnit, StepModel, UnitContext};
    use crate::parse;
    use crate::reader::ReaderContext;

    fn model_with_units(units: UnitContext) -> StepModel {
        let mut arena: Arena<UnitContext> = Arena::default();
        arena.push(units);
        StepModel {
            units: arena,
            ..StepModel::default()
        }
    }

    #[test]
    fn writes_length_unit_inch_as_conversion_based_unit() {
        let model = model_with_units(UnitContext {
            length: LengthUnit::Inch,
            plane_angle: AngleUnit::Radian,
            solid_angle: SolidAngleUnit::Steradian,
            length_uncertainty: None,
            plane_angle_uncertainty: None,
            solid_angle_uncertainty: None,
            length_cbu_wrapped: false,
            plane_angle_cbu_wrapped: false,
            dim_exp_explicit: false,
        });
        let out = model.write_to_string().expect("write");
        assert!(out.contains("CONVERSION_BASED_UNIT('INCH'"), "{out}");
        assert!(out.contains("LENGTH_MEASURE_WITH_UNIT(25.4"), "{out}");
        assert!(out.contains("DIMENSIONAL_EXPONENTS(1."), "{out}");
        assert!(out.contains("(.MILLI.,.METRE.)"), "{out}");
    }

    #[test]
    fn writes_length_unit_foot_as_conversion_based_unit() {
        let model = model_with_units(UnitContext {
            length: LengthUnit::Foot,
            plane_angle: AngleUnit::Radian,
            solid_angle: SolidAngleUnit::Steradian,
            length_uncertainty: None,
            plane_angle_uncertainty: None,
            solid_angle_uncertainty: None,
            length_cbu_wrapped: false,
            plane_angle_cbu_wrapped: false,
            dim_exp_explicit: false,
        });
        let out = model.write_to_string().expect("write");
        assert!(out.contains("CONVERSION_BASED_UNIT('FOOT'"), "{out}");
        assert!(out.contains("LENGTH_MEASURE_WITH_UNIT(304.8"), "{out}");
        assert!(out.contains("(.MILLI.,.METRE.)"), "{out}");
    }

    #[test]
    fn writes_angle_unit_degree_as_conversion_based_unit() {
        let model = model_with_units(UnitContext {
            length: LengthUnit::Millimetre,
            plane_angle: AngleUnit::Degree,
            solid_angle: SolidAngleUnit::Steradian,
            length_uncertainty: None,
            plane_angle_uncertainty: None,
            solid_angle_uncertainty: None,
            length_cbu_wrapped: false,
            plane_angle_cbu_wrapped: false,
            dim_exp_explicit: false,
        });
        let out = model.write_to_string().expect("write");
        assert!(out.contains("CONVERSION_BASED_UNIT('DEGREE'"), "{out}");
        assert!(
            out.contains("PLANE_ANGLE_MEASURE_WITH_UNIT(0.017453"),
            "{out}"
        );
        assert!(out.contains("DIMENSIONAL_EXPONENTS(0."), "{out}");
    }

    #[test]
    fn writes_millimetre_omits_conversion_based_unit() {
        let model = model_with_units(UnitContext {
            length: LengthUnit::Millimetre,
            plane_angle: AngleUnit::Radian,
            solid_angle: SolidAngleUnit::Steradian,
            length_uncertainty: None,
            plane_angle_uncertainty: None,
            solid_angle_uncertainty: None,
            length_cbu_wrapped: false,
            plane_angle_cbu_wrapped: false,
            dim_exp_explicit: false,
        });
        let out = model.write_to_string().expect("write");
        assert!(
            !out.contains("CONVERSION_BASED_UNIT"),
            "plain mm should not wrap in CBU: {out}"
        );
        assert!(out.contains("(.MILLI.,.METRE.)"), "{out}");
    }

    #[test]
    fn writes_centimetre_omits_conversion_based_unit() {
        let model = model_with_units(UnitContext {
            length: LengthUnit::Centimetre,
            plane_angle: AngleUnit::Radian,
            solid_angle: SolidAngleUnit::Steradian,
            length_uncertainty: None,
            plane_angle_uncertainty: None,
            solid_angle_uncertainty: None,
            length_cbu_wrapped: false,
            plane_angle_cbu_wrapped: false,
            dim_exp_explicit: false,
        });
        let out = model.write_to_string().expect("write");
        assert!(!out.contains("CONVERSION_BASED_UNIT"), "{out}");
        assert!(out.contains("(.CENTI.,.METRE.)"), "{out}");
    }

    #[test]
    fn writes_metre_omits_conversion_based_unit() {
        let model = model_with_units(UnitContext {
            length: LengthUnit::Metre,
            plane_angle: AngleUnit::Radian,
            solid_angle: SolidAngleUnit::Steradian,
            length_uncertainty: None,
            plane_angle_uncertainty: None,
            solid_angle_uncertainty: None,
            length_cbu_wrapped: false,
            plane_angle_cbu_wrapped: false,
            dim_exp_explicit: false,
        });
        let out = model.write_to_string().expect("write");
        assert!(!out.contains("CONVERSION_BASED_UNIT"), "{out}");
        assert!(out.contains("SI_UNIT($,.METRE.)"), "{out}");
    }

    /// Writer output must re-parse into an identical `UnitContext` — confirms
    /// reader/writer are paired for every supported unit flavour.
    #[test]
    fn non_metric_units_survive_write_then_read() {
        let cases = [
            UnitContext {
                length: LengthUnit::Inch,
                plane_angle: AngleUnit::Radian,
                solid_angle: SolidAngleUnit::Steradian,
                length_uncertainty: None,
                plane_angle_uncertainty: None,
                solid_angle_uncertainty: None,
                length_cbu_wrapped: false,
                plane_angle_cbu_wrapped: false,
                dim_exp_explicit: false,
            },
            UnitContext {
                length: LengthUnit::Foot,
                plane_angle: AngleUnit::Radian,
                solid_angle: SolidAngleUnit::Steradian,
                length_uncertainty: None,
                plane_angle_uncertainty: None,
                solid_angle_uncertainty: None,
                length_cbu_wrapped: false,
                plane_angle_cbu_wrapped: false,
                dim_exp_explicit: false,
            },
            UnitContext {
                length: LengthUnit::Metre,
                plane_angle: AngleUnit::Degree,
                solid_angle: SolidAngleUnit::Steradian,
                length_uncertainty: None,
                plane_angle_uncertainty: None,
                solid_angle_uncertainty: None,
                length_cbu_wrapped: false,
                plane_angle_cbu_wrapped: false,
                dim_exp_explicit: false,
            },
        ];
        for unit in cases {
            let model = model_with_units(unit.clone());
            let text = model.write_to_string().expect("write");
            let graph = parse(&text).expect("re-parse");
            let back = ReaderContext::convert(&graph);
            assert!(
                back.warnings.is_empty(),
                "warnings for {unit:?}: {:#?}",
                back.warnings
            );
            assert_eq!(
                back.model.units.iter().next().cloned(),
                Some(unit.clone()),
                "unit not preserved for {unit:?}"
            );
        }
    }

    /// Verify that an SI length unit wrapped in `CONVERSION_BASED_UNIT`
    /// (`'METRE'` self-wrap, factor 1.0) round-trips through both reader
    /// and writer faithfully — flag set on read, CBU re-emitted on write.
    #[test]
    fn cbu_wrapped_metre_round_trips() {
        let unit = UnitContext {
            length: LengthUnit::Metre,
            plane_angle: AngleUnit::Radian,
            solid_angle: SolidAngleUnit::Steradian,
            length_uncertainty: None,
            plane_angle_uncertainty: None,
            solid_angle_uncertainty: None,
            length_cbu_wrapped: true,
            plane_angle_cbu_wrapped: false,
            dim_exp_explicit: false,
        };
        let model = model_with_units(unit.clone());
        let text = model.write_to_string().expect("write");
        assert!(
            text.contains("CONVERSION_BASED_UNIT('METRE'"),
            "writer must emit CBU('METRE') when length_cbu_wrapped: {text}"
        );
        assert!(text.contains("LENGTH_MEASURE_WITH_UNIT(1."));
        assert!(text.contains("DIMENSIONAL_EXPONENTS(1."));

        let graph = parse(&text).expect("re-parse");
        let back = ReaderContext::convert(&graph);
        assert!(back.warnings.is_empty(), "{:#?}", back.warnings);
        assert_eq!(
            back.model.units.iter().next().cloned(),
            Some(unit),
            "CBU wrap flag preserved"
        );
    }

    #[test]
    fn cbu_wrapped_radian_round_trips() {
        let unit = UnitContext {
            length: LengthUnit::Millimetre,
            plane_angle: AngleUnit::Radian,
            solid_angle: SolidAngleUnit::Steradian,
            length_uncertainty: None,
            plane_angle_uncertainty: None,
            solid_angle_uncertainty: None,
            length_cbu_wrapped: false,
            plane_angle_cbu_wrapped: true,
            dim_exp_explicit: false,
        };
        let model = model_with_units(unit.clone());
        let text = model.write_to_string().expect("write");
        assert!(
            text.contains("CONVERSION_BASED_UNIT('RADIAN'"),
            "writer must emit CBU('RADIAN') when plane_angle_cbu_wrapped: {text}"
        );

        let graph = parse(&text).expect("re-parse");
        let back = ReaderContext::convert(&graph);
        assert!(back.warnings.is_empty(), "{:#?}", back.warnings);
        assert_eq!(back.model.units.iter().next().cloned(), Some(unit));
    }

    /// Synthetic round-trip for plane-angle / solid-angle uncertainty.
    /// No production fixture exercises these (every observed fixture stores
    /// only a length uncertainty), so this test pins the read/write paths
    /// against a hand-built `UnitContext`.
    #[test]
    fn angle_and_solid_angle_uncertainty_round_trip() {
        use crate::ir::shape_rep::LengthUncertainty;
        let unit = UnitContext {
            length: LengthUnit::Millimetre,
            plane_angle: AngleUnit::Radian,
            solid_angle: SolidAngleUnit::Steradian,
            length_uncertainty: Some(LengthUncertainty {
                value: 1e-7,
                name: "distance_accuracy_value".into(),
                description: "confusion accuracy".into(),
            }),
            plane_angle_uncertainty: Some(LengthUncertainty {
                value: 1e-5,
                name: "angle_accuracy".into(),
                description: "angle uncertainty".into(),
            }),
            solid_angle_uncertainty: Some(LengthUncertainty {
                value: 1e-3,
                name: "solid_angle_accuracy".into(),
                description: "solid angle uncertainty".into(),
            }),
            length_cbu_wrapped: false,
            plane_angle_cbu_wrapped: false,
            dim_exp_explicit: false,
        };
        let model = model_with_units(unit.clone());
        let text = model.write_to_string().expect("write");
        assert!(text.contains("LENGTH_MEASURE("), "{text}");
        assert!(text.contains("PLANE_ANGLE_MEASURE("), "{text}");
        assert!(text.contains("SOLID_ANGLE_MEASURE("), "{text}");
        assert!(text.contains("'angle_accuracy'"), "{text}");
        assert!(text.contains("'solid_angle_accuracy'"), "{text}");

        let graph = parse(&text).expect("re-parse");
        let back = ReaderContext::convert(&graph);
        assert!(back.warnings.is_empty(), "{:#?}", back.warnings);
        assert_eq!(back.model.units.iter().next().cloned(), Some(unit));
    }
}
