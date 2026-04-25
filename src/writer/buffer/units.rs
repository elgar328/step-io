//! Unit context emission: SI length / plane-angle / solid-angle leaves plus
//! the enclosing `GLOBAL_UNIT_ASSIGNED_CONTEXT` complex entity.

use super::WriteBuffer;
use crate::ir::{AngleUnit, LengthUnit, SolidAngleUnit, UnitContext};
use crate::parser::entity::Attribute;
use crate::writer::entity::{WriterBody, WriterEntity};

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_unit_context(&mut self, units: UnitContext) -> u64 {
        if let Some(n) = self.global_unit_context_id {
            return n;
        }
        let length = self.emit_length_unit(&units);
        let angle = self.emit_angle_unit(&units);
        let solid = self.emit_solid_angle_unit(units.solid_angle);

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
        self.global_unit_context_id = Some(ctx);
        ctx
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

    fn emit_length_unit(&mut self, units: &UnitContext) -> u64 {
        if let Some(n) = self.length_unit_id {
            return n;
        }
        match units.length {
            LengthUnit::Millimetre if units.length_cbu_wrapped => {
                self.emit_conversion_based_length("MILLIMETRE", Some("MILLI"), 1.0)
            }
            LengthUnit::Centimetre if units.length_cbu_wrapped => {
                self.emit_conversion_based_length("CENTIMETRE", Some("CENTI"), 1.0)
            }
            LengthUnit::Metre if units.length_cbu_wrapped => {
                self.emit_conversion_based_length("METRE", None, 1.0)
            }
            LengthUnit::Millimetre => self.emit_plain_si_length(Some("MILLI")),
            LengthUnit::Centimetre => self.emit_plain_si_length(Some("CENTI")),
            LengthUnit::Metre => self.emit_plain_si_length(None),
            LengthUnit::Inch => self.emit_conversion_based_length("INCH", Some("MILLI"), 25.4),
            LengthUnit::Foot => self.emit_conversion_based_length("FOOT", Some("MILLI"), 304.8),
        }
    }

    /// Emit a plain SI-based length unit and cache the id as `length_unit_id`.
    fn emit_plain_si_length(&mut self, prefix: Option<&'static str>) -> u64 {
        let si_attrs = match prefix {
            Some(p) => vec![Attribute::Enum(p.into()), Attribute::Enum("METRE".into())],
            None => vec![Attribute::Unset, Attribute::Enum("METRE".into())],
        };
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex {
                parts: vec![
                    ("LENGTH_UNIT".into(), vec![]),
                    ("NAMED_UNIT".into(), vec![Attribute::Derived]),
                    ("SI_UNIT".into(), si_attrs),
                ],
            },
        });
        self.length_unit_id = Some(n);
        n
    }

    /// Emit an SI length unit complex without populating `length_unit_id`
    /// — used internally as the base for a `CONVERSION_BASED_UNIT` length
    /// chain. `prefix = None` for plain METRE, `Some("MILLI")` for
    /// MILLIMETRE, `Some("CENTI")` for CENTIMETRE, etc.
    fn emit_base_si_length(&mut self, prefix: Option<&'static str>) -> u64 {
        let prefix_attr = match prefix {
            Some(p) => Attribute::Enum(p.into()),
            None => Attribute::Unset,
        };
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex {
                parts: vec![
                    ("LENGTH_UNIT".into(), vec![]),
                    ("NAMED_UNIT".into(), vec![Attribute::Derived]),
                    (
                        "SI_UNIT".into(),
                        vec![prefix_attr, Attribute::Enum("METRE".into())],
                    ),
                ],
            },
        });
        n
    }

    /// Emit the length-flavour `DIMENSIONAL_EXPONENTS` (1,0,0,0,0,0,0), cached.
    fn emit_length_dim_exponents(&mut self) -> u64 {
        if let Some(n) = self.length_dim_exp_id {
            return n;
        }
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "DIMENSIONAL_EXPONENTS".into(),
                attrs: vec![
                    Attribute::Real(1.0),
                    Attribute::Real(0.0),
                    Attribute::Real(0.0),
                    Attribute::Real(0.0),
                    Attribute::Real(0.0),
                    Attribute::Real(0.0),
                    Attribute::Real(0.0),
                ],
            },
        });
        self.length_dim_exp_id = Some(n);
        n
    }

    /// Emit the dimensionless `DIMENSIONAL_EXPONENTS` (0,0,0,0,0,0,0), cached.
    fn emit_dimensionless_exponents(&mut self) -> u64 {
        if let Some(n) = self.dimensionless_dim_exp_id {
            return n;
        }
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "DIMENSIONAL_EXPONENTS".into(),
                attrs: vec![Attribute::Real(0.0); 7],
            },
        });
        self.dimensionless_dim_exp_id = Some(n);
        n
    }

    /// Emit a `CONVERSION_BASED_UNIT` length chain. Used for both genuine
    /// non-SI units (Inch / Foot — base MILLI METRE, factor 25.4 / 304.8)
    /// and SI self-wraps (METRE / MILLIMETRE / CENTIMETRE — base same as
    /// the unit, factor 1.0). Wraps `LENGTH_MEASURE_WITH_UNIT` referencing
    /// the SI base and the shared `DIMENSIONAL_EXPONENTS(1,...)`. Returns
    /// the outer CBU id and caches it as `length_unit_id`.
    fn emit_conversion_based_length(
        &mut self,
        name: &str,
        base_prefix: Option<&'static str>,
        factor: f64,
    ) -> u64 {
        let base_si = self.emit_base_si_length(base_prefix);
        let dim_exp = self.emit_length_dim_exponents();
        let measure = self.fresh();
        self.entities.push(WriterEntity {
            id: measure,
            body: WriterBody::Simple {
                name: "LENGTH_MEASURE_WITH_UNIT".into(),
                attrs: vec![Attribute::Real(factor), Attribute::EntityRef(base_si)],
            },
        });
        let outer = self.fresh();
        self.entities.push(WriterEntity {
            id: outer,
            body: WriterBody::Complex {
                parts: vec![
                    (
                        "CONVERSION_BASED_UNIT".into(),
                        vec![
                            Attribute::String(name.into()),
                            Attribute::EntityRef(measure),
                        ],
                    ),
                    ("LENGTH_UNIT".into(), vec![]),
                    ("NAMED_UNIT".into(), vec![Attribute::EntityRef(dim_exp)]),
                ],
            },
        });
        self.length_unit_id = Some(outer);
        outer
    }

    fn emit_angle_unit(&mut self, units: &UnitContext) -> u64 {
        if let Some(n) = self.angle_unit_id {
            return n;
        }
        match units.plane_angle {
            AngleUnit::Radian if units.plane_angle_cbu_wrapped => {
                self.emit_conversion_based_angle("RADIAN", 1.0)
            }
            AngleUnit::Radian => self.emit_plain_si_radian(),
            AngleUnit::Degree => {
                self.emit_conversion_based_angle("DEGREE", std::f64::consts::PI / 180.0)
            }
        }
    }

    fn emit_plain_si_radian(&mut self) -> u64 {
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex {
                parts: vec![
                    (
                        "SI_UNIT".into(),
                        vec![Attribute::Unset, Attribute::Enum("RADIAN".into())],
                    ),
                    ("NAMED_UNIT".into(), vec![Attribute::Derived]),
                    ("PLANE_ANGLE_UNIT".into(), vec![]),
                ],
            },
        });
        self.angle_unit_id = Some(n);
        n
    }

    /// Emit a bare SI radian entity (not cached in `angle_unit_id`) — used
    /// as the base inside a Degree `CONVERSION_BASED_UNIT` chain.
    fn emit_base_si_radian(&mut self) -> u64 {
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex {
                parts: vec![
                    ("NAMED_UNIT".into(), vec![Attribute::Derived]),
                    ("PLANE_ANGLE_UNIT".into(), vec![]),
                    (
                        "SI_UNIT".into(),
                        vec![Attribute::Unset, Attribute::Enum("RADIAN".into())],
                    ),
                ],
            },
        });
        n
    }

    /// Emit a `CONVERSION_BASED_UNIT` plane-angle chain. Used for genuine
    /// non-SI angles (Degree — factor π/180) and for SI self-wrap (Radian
    /// — factor 1.0). Base is always plain SI RADIAN.
    fn emit_conversion_based_angle(&mut self, name: &str, factor: f64) -> u64 {
        let base_si = self.emit_base_si_radian();
        let dim_exp = self.emit_dimensionless_exponents();
        let measure = self.fresh();
        self.entities.push(WriterEntity {
            id: measure,
            body: WriterBody::Simple {
                name: "PLANE_ANGLE_MEASURE_WITH_UNIT".into(),
                attrs: vec![Attribute::Real(factor), Attribute::EntityRef(base_si)],
            },
        });
        let outer = self.fresh();
        self.entities.push(WriterEntity {
            id: outer,
            body: WriterBody::Complex {
                parts: vec![
                    (
                        "CONVERSION_BASED_UNIT".into(),
                        vec![
                            Attribute::String(name.into()),
                            Attribute::EntityRef(measure),
                        ],
                    ),
                    ("NAMED_UNIT".into(), vec![Attribute::EntityRef(dim_exp)]),
                    ("PLANE_ANGLE_UNIT".into(), vec![]),
                ],
            },
        });
        self.angle_unit_id = Some(outer);
        outer
    }

    fn emit_solid_angle_unit(&mut self, _unit: SolidAngleUnit) -> u64 {
        if let Some(n) = self.solid_angle_unit_id {
            return n;
        }
        let parts = vec![
            (
                "SI_UNIT".into(),
                vec![Attribute::Unset, Attribute::Enum("STERADIAN".into())],
            ),
            ("NAMED_UNIT".into(), vec![Attribute::Derived]),
            ("SOLID_ANGLE_UNIT".into(), vec![]),
        ];
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex { parts },
        });
        self.solid_angle_unit_id = Some(n);
        n
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{AngleUnit, LengthUnit, SolidAngleUnit, StepModel, UnitContext};
    use crate::parse;
    use crate::reader::ReaderContext;

    fn model_with_units(units: UnitContext) -> StepModel {
        StepModel {
            units: Some(units),
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
            },
            UnitContext {
                length: LengthUnit::Foot,
                plane_angle: AngleUnit::Radian,
                solid_angle: SolidAngleUnit::Steradian,
                length_uncertainty: None,
                length_cbu_wrapped: false,
                plane_angle_cbu_wrapped: false,
            },
            UnitContext {
                length: LengthUnit::Metre,
                plane_angle: AngleUnit::Degree,
                solid_angle: SolidAngleUnit::Steradian,
                length_uncertainty: None,
                length_cbu_wrapped: false,
                plane_angle_cbu_wrapped: false,
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
                back.model.units,
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
        assert_eq!(back.model.units, Some(unit), "CBU wrap flag preserved");
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
        assert_eq!(back.model.units, Some(unit));
    }
}
