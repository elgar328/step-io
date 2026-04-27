//! Unit context emission: orchestrates the entity-handler chain that
//! produces the enclosing `GLOBAL_UNIT_ASSIGNED_CONTEXT` complex entity
//! and its length / plane-angle / solid-angle leaves. The leaf bodies
//! live under `src/entities/units/`; this file is a thin wrapper that
//! `emit_all` calls once per `UnitContext` arena entry.

use super::WriteBuffer;
use crate::ir::UnitContext;
use crate::parser::entity::Attribute;
use crate::writer::WriteError;
use crate::writer::entity::{WriterBody, WriterEntity};

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_unit_context(
        &mut self,
        units: UnitContext,
    ) -> Result<u64, WriteError> {
        // No top-level cache — `emit_all` calls this once per `UnitContext`
        // arena entry and stores each result in `unit_context_ids`. The
        // length/angle/solid_angle leaf caches still dedup the underlying
        // unit references across contexts that share leaves (the common case
        // for Fusion 360, where two contexts share #1034..#1036).
        use crate::entities::ComplexEntityHandler;
        let length = crate::entities::units::length_unit::LengthUnitHandler::write(
            self,
            (
                units.length,
                units.length_cbu_wrapped,
                units.dim_exp_explicit,
            ),
        )?;
        let angle = crate::entities::units::plane_angle_unit::PlaneAngleUnitHandler::write(
            self,
            (
                units.plane_angle,
                units.plane_angle_cbu_wrapped,
                units.dim_exp_explicit,
            ),
        )?;
        let solid = crate::entities::units::solid_angle_unit::SolidAngleUnitHandler::write(
            self,
            (units.solid_angle, units.dim_exp_explicit),
        )?;

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
            let unc = self.emit_uncertainty_measure(value, length);
            parts.insert(
                1,
                (
                    "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT".into(),
                    vec![Attribute::List(vec![Attribute::EntityRef(unc)])],
                ),
            );
        }

        let ctx = self.fresh();
        self.entities.push(WriterEntity {
            id: ctx,
            body: WriterBody::Complex { parts },
        });
        Ok(ctx)
    }

    fn emit_uncertainty_measure(&mut self, value: f64, length_unit: u64) -> u64 {
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "UNCERTAINTY_MEASURE_WITH_UNIT".into(),
                attrs: vec![
                    Attribute::Typed {
                        type_name: "LENGTH_MEASURE".into(),
                        value: Box::new(Attribute::Real(value)),
                    },
                    Attribute::EntityRef(length_unit),
                    Attribute::String("distance_accuracy_value".into()),
                    Attribute::String("confusion accuracy".into()),
                ],
            },
        });
        n
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
                length_cbu_wrapped: false,
                plane_angle_cbu_wrapped: false,
                dim_exp_explicit: false,
            },
            UnitContext {
                length: LengthUnit::Foot,
                plane_angle: AngleUnit::Radian,
                solid_angle: SolidAngleUnit::Steradian,
                length_uncertainty: None,
                length_cbu_wrapped: false,
                plane_angle_cbu_wrapped: false,
                dim_exp_explicit: false,
            },
            UnitContext {
                length: LengthUnit::Metre,
                plane_angle: AngleUnit::Degree,
                solid_angle: SolidAngleUnit::Steradian,
                length_uncertainty: None,
                length_cbu_wrapped: false,
                plane_angle_cbu_wrapped: false,
                dim_exp_explicit: false,
            },
        ];
        for unit in cases {
            let model = model_with_units(unit);
            let text = model.write_to_string().expect("write");
            let graph = parse(&text).expect("re-parse");
            let back = ReaderContext::convert(&graph);
            assert!(
                back.warnings.is_empty(),
                "warnings for {unit:?}: {:#?}",
                back.warnings
            );
            assert_eq!(
                back.model.units.iter().next().copied(),
                Some(unit),
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
            length_cbu_wrapped: true,
            plane_angle_cbu_wrapped: false,
            dim_exp_explicit: false,
        };
        let model = model_with_units(unit);
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
            back.model.units.iter().next().copied(),
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
            length_cbu_wrapped: false,
            plane_angle_cbu_wrapped: true,
            dim_exp_explicit: false,
        };
        let model = model_with_units(unit);
        let text = model.write_to_string().expect("write");
        assert!(
            text.contains("CONVERSION_BASED_UNIT('RADIAN'"),
            "writer must emit CBU('RADIAN') when plane_angle_cbu_wrapped: {text}"
        );

        let graph = parse(&text).expect("re-parse");
        let back = ReaderContext::convert(&graph);
        assert!(back.warnings.is_empty(), "{:#?}", back.warnings);
        assert_eq!(back.model.units.iter().next().copied(), Some(unit));
    }
}
