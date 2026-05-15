//! Unit context emission. Plan 5.6 C4 lifted the entire emit chain
//! into `entities/units/global_unit_assigned_context.rs` (the
//! orchestrator) plus the three leaf handlers and the
//! `UncertaintyMeasureWithUnit` handler. This file remains as a 1-line
//! dispatcher so `emit_all` keeps a stable entry point — analogous to
//! the `emit_face` / `emit_curve` wrappers in geometry / topology.

use super::WriteBuffer;
use crate::ir::UnitContext;
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
