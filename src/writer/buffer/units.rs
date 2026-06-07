//! Unit context emission. The entire emit chain lives
//! in `entities/units/global_unit_assigned_context.rs` (the
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
        // Dispatch through EntityHandler trait. Body
        // lives in
        // `src/entities/shape_rep/global_unit_assigned_context.rs`.
        use crate::entities::ComplexEntityHandler;
        crate::entities::shape_rep::global_unit_assigned_context::GlobalUnitAssignedContextHandler::write(self, units)
    }

    /// Emit the [`UnitsPool`] arenas (`NamedUnit`, `MeasureWithUnit`,
    /// `DerivedUnitElement`, `DerivedUnit`) and populate the step-id caches
    /// that GUAC emit + property emit later consult.
    ///
    /// `NamedUnit` step ids are pre-reserved in arena order before any
    /// entity is pushed, so a CBU outer can forward-reference its base
    /// even when the base comes later in arena order. Sub-entities (DE,
    /// MWU inside a CBU chain) use `fresh()` and land after the
    /// reservation block.
    ///
    /// [`UnitsPool`]: crate::ir::UnitsPool
    pub(crate) fn emit_units_pool_if_set(&mut self) -> Result<(), WriteError> {
        let Some(pool) = self.model.units_pool.as_ref() else {
            return Ok(());
        };
        // DIMENSIONAL_EXPONENTS arena (phase dim-exp-arena-c) emits first
        // so NAMED_UNIT subtype writers below can resolve flavor.dim_exp
        // through `dimensional_exponents_step_ids`.
        self.dimensional_exponents_step_ids
            .resize(pool.dimensional_exponents.len(), 0);
        for (id, de) in pool.dimensional_exponents.iter_with_ids() {
            use crate::entities::SimpleEntityHandler;
            use crate::entities::units::dimensional_exponents::DimensionalExponentsHandler;
            let step = DimensionalExponentsHandler::write(self, *de)?;
            self.dimensional_exponents_step_ids[id.0 as usize] = step;
        }
        // units-2: pre-reserve step ids for all NamedUnit entries in arena
        // order. The emit then writes each entry at its reserved id. This
        // keeps NAMED_UNIT entity-id ordering matching the IR arena order
        // (so re-read produces an identical arena), and CBU outers can
        // reference their base's pre-reserved id even if the base appears
        // later in arena order (forward-ref).
        self.named_unit_step_ids.resize(pool.named_units.len(), 0);
        for (id, _) in pool.named_units.iter_with_ids() {
            self.named_unit_step_ids[id.0 as usize] = self.fresh();
        }
        // Now emit each entry at its reserved id. Sub-entities (DE, MWU)
        // use `fresh()` and get ids after the reservation block.
        let entries: Vec<(crate::ir::id::NamedUnitId, NamedUnit)> = pool
            .named_units
            .iter_with_ids()
            .map(|(id, n)| (id, *n))
            .collect();
        for (id, named) in entries {
            let target = self.named_unit_step_ids[id.0 as usize];
            if let Some(base_id) = cbu_base_of(&named) {
                let base_step = self.named_unit_step_ids[base_id.0 as usize];
                emit_named_unit_cbu(self, named, base_step, target)?;
            } else {
                emit_named_unit_plain(self, named, target)?;
            }
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
        // units-1b / units-3a: DERIVED_UNIT and its dimension-constrained
        // subtypes (AREA_UNIT / VOLUME_UNIT) wrap DUE refs — emit after the
        // DUE loop so `due_step_ids` is fully populated.
        self.derived_unit_step_ids
            .resize(pool.derived_units.len(), 0);
        let du_entries: Vec<(crate::ir::id::DerivedUnitId, crate::ir::units::DerivedUnit)> = pool
            .derived_units
            .iter_with_ids()
            .map(|(id, du)| (id, du.clone()))
            .collect();
        for (id, du) in du_entries {
            let element_steps: Vec<u64> = du
                .elements
                .iter()
                .map(|e| self.due_step_ids[e.0 as usize])
                .collect();
            let step = emit_derived_unit_by_kind(self, du.kind, element_steps)?;
            self.derived_unit_step_ids[id.0 as usize] = step;
        }
        Ok(())
    }
}

fn emit_derived_unit_by_kind(
    buf: &mut WriteBuffer<'_>,
    kind: crate::ir::units::DerivedUnitKind,
    elements: Vec<u64>,
) -> Result<u64, WriteError> {
    use crate::entities::SimpleEntityHandler;
    use crate::ir::units::DerivedUnitKind;
    match kind {
        DerivedUnitKind::Plain => {
            crate::entities::units::derived_unit::DerivedUnitHandler::write(buf, elements)
        }
        DerivedUnitKind::AreaUnit => {
            crate::entities::units::area_unit::AreaUnitHandler::write(buf, elements)
        }
        DerivedUnitKind::VolumeUnit => {
            crate::entities::units::volume_unit::VolumeUnitHandler::write(buf, elements)
        }
    }
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

fn cbu_base_of(named: &NamedUnit) -> Option<crate::ir::id::NamedUnitId> {
    match named {
        NamedUnit::Length(f) => f.cbu_base,
        NamedUnit::PlaneAngle(f) => f.cbu_base,
        NamedUnit::SolidAngle(_) | NamedUnit::Ratio(_) | NamedUnit::Itself(_) => None,
        NamedUnit::Mass(f) => f.cbu_base,
    }
}

fn emit_named_unit_plain(
    buf: &mut WriteBuffer<'_>,
    named: NamedUnit,
    target_id: u64,
) -> Result<u64, WriteError> {
    use crate::entities::units::length_unit::LengthUnitHandler;
    use crate::entities::units::mass_unit::MassUnitHandler;
    use crate::entities::units::named_unit::NamedUnitSimpleHandler;
    use crate::entities::units::plane_angle_unit::PlaneAngleUnitHandler;
    use crate::entities::units::ratio_unit::{RatioUnitHandler, RatioUnitSimpleHandler};
    use crate::entities::units::solid_angle_unit::SolidAngleUnitHandler;
    use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
    let dim_exp_step = |de: Option<crate::ir::DimensionalExponentsId>| {
        de.map_or(0, |id| buf.dimensional_exponents_step_ids[id.0 as usize])
    };
    match named {
        NamedUnit::Length(f) => {
            LengthUnitHandler::write(buf, (f.unit, target_id, dim_exp_step(f.dim_exp)))
        }
        NamedUnit::PlaneAngle(f) => {
            PlaneAngleUnitHandler::write(buf, (f.unit, target_id, dim_exp_step(f.dim_exp)))
        }
        NamedUnit::SolidAngle(f) => {
            SolidAngleUnitHandler::write(buf, (f.unit, target_id, dim_exp_step(f.dim_exp)))
        }
        NamedUnit::Mass(f) => {
            MassUnitHandler::write(buf, (f.unit, target_id, dim_exp_step(f.dim_exp)))
        }
        // Reproduce the source form: complex `(NAMED_UNIT()RATIO_UNIT())` vs
        // the standalone simple `RATIO_UNIT(dimensions)` entity.
        NamedUnit::Ratio(f) if f.complex => {
            RatioUnitHandler::write(buf, (target_id, dim_exp_step(f.dim_exp)))
        }
        NamedUnit::Ratio(f) => {
            RatioUnitSimpleHandler::write(buf, (target_id, dim_exp_step(f.dim_exp)))
        }
        // Bare NAMED_UNIT(#dimensions) — a dimensionless/count unit.
        NamedUnit::Itself(d) => {
            NamedUnitSimpleHandler::write(buf, (target_id, dim_exp_step(d.dimensions)))
        }
    }
}

fn emit_named_unit_cbu(
    buf: &mut WriteBuffer<'_>,
    named: NamedUnit,
    base_step: u64,
    target_id: u64,
) -> Result<u64, WriteError> {
    use crate::entities::units::length_unit::emit_length_cbu_outer;
    use crate::entities::units::mass_unit::emit_mass_cbu_outer;
    use crate::entities::units::plane_angle_unit::emit_plane_angle_cbu_outer;
    let dim_exp_step = |de: Option<crate::ir::DimensionalExponentsId>| {
        de.map_or(0, |id| buf.dimensional_exponents_step_ids[id.0 as usize])
    };
    match named {
        NamedUnit::Length(f) => Ok(emit_length_cbu_outer(
            buf,
            f.unit,
            base_step,
            target_id,
            dim_exp_step(f.dim_exp),
            f.cbu_factor_bare,
        )),
        NamedUnit::PlaneAngle(f) => Ok(emit_plane_angle_cbu_outer(
            buf,
            f.unit,
            base_step,
            target_id,
            dim_exp_step(f.dim_exp),
        )),
        NamedUnit::Mass(f) => {
            emit_mass_cbu_outer(buf, f.unit, base_step, target_id, dim_exp_step(f.dim_exp))
        }
        // SolidAngle / Ratio / bare Itself have no CBU variant; fall through.
        NamedUnit::SolidAngle(_) | NamedUnit::Ratio(_) | NamedUnit::Itself(_) => {
            emit_named_unit_plain(buf, named, target_id)
        }
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
    match mwu {
        MeasureWithUnit::Itself(d) => {
            use crate::entities::units::measure_with_unit::MeasureWithUnitHandler;
            let unit_step = buf.named_unit_step_ids[d.unit.0 as usize];
            // Bare supertype: re-emit `MEASURE_WITH_UNIT(<measure_type>(value), unit)`
            // preserving the generic form (not a typed subtype).
            MeasureWithUnitHandler::write(buf, (d.measure_type.clone(), d.value, unit_step))
        }
        MeasureWithUnit::Length { value, unit } => {
            let unit_step = buf.named_unit_step_ids[unit.0 as usize];
            LengthMeasureWithUnitHandler::write(buf, (*value, unit_step))
        }
        MeasureWithUnit::Mass { value, unit } => {
            let unit_step = buf.named_unit_step_ids[unit.0 as usize];
            MassMeasureWithUnitHandler::write(buf, (*value, unit_step))
        }
        MeasureWithUnit::PlaneAngle { value, unit } => {
            let unit_step = buf.named_unit_step_ids[unit.0 as usize];
            PlaneAngleMeasureWithUnitHandler::write(buf, (*value, unit_step))
        }
        MeasureWithUnit::Ratio { value, unit } => {
            let unit_step = buf.named_unit_step_ids[unit.0 as usize];
            RatioMeasureWithUnitHandler::write(buf, (*value, unit_step))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::arena::Arena;
    use crate::ir::shape_rep::LengthUncertainty;
    use crate::ir::units::{NamedUnit, UnitsPool};
    use crate::ir::{AngleUnit, LengthUnit, SolidAngleUnit, StepModel, UnitContext};
    use crate::parse;
    use crate::reader::ReaderContext;

    /// units-2 test helper: build a `StepModel` with a single `UnitContext`
    /// constructed by pushing fresh `LengthFlavor` / `PlaneAngleFlavor` /
    /// `SolidAngleFlavor` entries into the model's units pool. CBU outers
    /// (Inch / Foot / CBU-wrapped SI / Degree) automatically get a
    /// `cbu_base` pointing at their SI base in the pool.
    struct UnitsBuilder {
        length: LengthUnit,
        plane_angle: AngleUnit,
        solid_angle: SolidAngleUnit,
        length_self_wrap: bool,
        plane_angle_self_wrap: bool,
        length_uncertainty: Option<LengthUncertainty>,
        plane_angle_uncertainty: Option<LengthUncertainty>,
        solid_angle_uncertainty: Option<LengthUncertainty>,
    }

    impl UnitsBuilder {
        fn new(length: LengthUnit, plane: AngleUnit, solid: SolidAngleUnit) -> Self {
            Self {
                length,
                plane_angle: plane,
                solid_angle: solid,
                length_self_wrap: false,
                plane_angle_self_wrap: false,
                length_uncertainty: None,
                plane_angle_uncertainty: None,
                solid_angle_uncertainty: None,
            }
        }
        /// Wrap the SI length in a `CONVERSION_BASED_UNIT('METRE', ...)` outer
        /// whose base is the same SI unit (corpus self-wrap pattern).
        fn length_self_wrap(mut self, v: bool) -> Self {
            self.length_self_wrap = v;
            self
        }
        fn plane_angle_self_wrap(mut self, v: bool) -> Self {
            self.plane_angle_self_wrap = v;
            self
        }
        fn length_uncertainty(mut self, v: LengthUncertainty) -> Self {
            self.length_uncertainty = Some(v);
            self
        }
        fn plane_angle_uncertainty(mut self, v: LengthUncertainty) -> Self {
            self.plane_angle_uncertainty = Some(v);
            self
        }
        fn solid_angle_uncertainty(mut self, v: LengthUncertainty) -> Self {
            self.solid_angle_uncertainty = Some(v);
            self
        }

        fn build(self) -> StepModel {
            let mut pool = UnitsPool::default();

            // Length: non-SI (Inch / Foot) or self-wrap → CBU outer with
            // a base SI Millimetre entry. Plain SI → no base.
            let length_id = match self.length {
                LengthUnit::Inch | LengthUnit::Foot => {
                    pool.push_cbu_length(self.length, LengthUnit::Millimetre)
                }
                _ if self.length_self_wrap => pool.push_cbu_length(self.length, self.length),
                _ => pool.push_plain_length(self.length),
            };
            let plane_id = match self.plane_angle {
                AngleUnit::Degree => pool.push_cbu_plane_angle(self.plane_angle, AngleUnit::Radian),
                AngleUnit::Radian if self.plane_angle_self_wrap => {
                    pool.push_cbu_plane_angle(self.plane_angle, self.plane_angle)
                }
                AngleUnit::Radian => pool.push_plain_plane_angle(self.plane_angle),
            };
            let solid_id = pool.push_plain_solid_angle(self.solid_angle);

            let ctx = UnitContext {
                units: vec![length_id, plane_id, solid_id],
                length_uncertainty: self.length_uncertainty,
                plane_angle_uncertainty: self.plane_angle_uncertainty,
                solid_angle_uncertainty: self.solid_angle_uncertainty,
                form: crate::ir::shape_rep::UnitContextForm::Complex,
            };

            let mut arena: Arena<UnitContext> = Arena::default();
            arena.push(ctx);
            StepModel {
                units: arena,
                units_pool: Some(pool),
                ..StepModel::default()
            }
        }
    }

    fn model_with_units(builder: UnitsBuilder) -> StepModel {
        builder.build()
    }

    /// Lookup the resolved `LengthUnit` for the first context's `length` ref.
    fn first_length(model: &StepModel) -> Option<LengthUnit> {
        let ctx = model.units.iter().next()?;
        let pool = model.units_pool.as_ref()?;
        match pool.named_units[ctx.length(pool)?] {
            NamedUnit::Length(f) => Some(f.unit),
            _ => None,
        }
    }
    fn first_plane_angle(model: &StepModel) -> Option<AngleUnit> {
        let ctx = model.units.iter().next()?;
        let pool = model.units_pool.as_ref()?;
        match pool.named_units[ctx.plane_angle(pool)?] {
            NamedUnit::PlaneAngle(f) => Some(f.unit),
            _ => None,
        }
    }
    fn first_solid_angle(model: &StepModel) -> Option<SolidAngleUnit> {
        let ctx = model.units.iter().next()?;
        let pool = model.units_pool.as_ref()?;
        match pool.named_units[ctx.solid_angle(pool)?] {
            NamedUnit::SolidAngle(f) => Some(f.unit),
            _ => None,
        }
    }
    fn first_ctx(model: &StepModel) -> Option<&UnitContext> {
        model.units.iter().next()
    }

    #[test]
    fn writes_length_unit_inch_as_conversion_based_unit() {
        let model = model_with_units(UnitsBuilder::new(
            LengthUnit::Inch,
            AngleUnit::Radian,
            SolidAngleUnit::Steradian,
        ));
        let out = model.write_to_string().expect("write");
        assert!(out.contains("CONVERSION_BASED_UNIT('INCH'"), "{out}");
        assert!(out.contains("LENGTH_MEASURE_WITH_UNIT(25.4"), "{out}");
        assert!(out.contains("DIMENSIONAL_EXPONENTS(1."), "{out}");
        assert!(out.contains("(.MILLI.,.METRE.)"), "{out}");
    }

    #[test]
    fn writes_length_unit_foot_as_conversion_based_unit() {
        let model = model_with_units(UnitsBuilder::new(
            LengthUnit::Foot,
            AngleUnit::Radian,
            SolidAngleUnit::Steradian,
        ));
        let out = model.write_to_string().expect("write");
        assert!(out.contains("CONVERSION_BASED_UNIT('FOOT'"), "{out}");
        assert!(out.contains("LENGTH_MEASURE_WITH_UNIT(304.8"), "{out}");
        assert!(out.contains("(.MILLI.,.METRE.)"), "{out}");
    }

    #[test]
    fn writes_angle_unit_degree_as_conversion_based_unit() {
        let model = model_with_units(UnitsBuilder::new(
            LengthUnit::Millimetre,
            AngleUnit::Degree,
            SolidAngleUnit::Steradian,
        ));
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
        let model = model_with_units(UnitsBuilder::new(
            LengthUnit::Millimetre,
            AngleUnit::Radian,
            SolidAngleUnit::Steradian,
        ));
        let out = model.write_to_string().expect("write");
        assert!(
            !out.contains("CONVERSION_BASED_UNIT"),
            "plain mm should not wrap in CBU: {out}"
        );
        assert!(out.contains("(.MILLI.,.METRE.)"), "{out}");
    }

    #[test]
    fn writes_centimetre_omits_conversion_based_unit() {
        let model = model_with_units(UnitsBuilder::new(
            LengthUnit::Centimetre,
            AngleUnit::Radian,
            SolidAngleUnit::Steradian,
        ));
        let out = model.write_to_string().expect("write");
        assert!(!out.contains("CONVERSION_BASED_UNIT"), "{out}");
        assert!(out.contains("(.CENTI.,.METRE.)"), "{out}");
    }

    #[test]
    fn writes_metre_omits_conversion_based_unit() {
        let model = model_with_units(UnitsBuilder::new(
            LengthUnit::Metre,
            AngleUnit::Radian,
            SolidAngleUnit::Steradian,
        ));
        let out = model.write_to_string().expect("write");
        assert!(!out.contains("CONVERSION_BASED_UNIT"), "{out}");
        assert!(out.contains("SI_UNIT($,.METRE.)"), "{out}");
    }

    /// Writer output must re-parse into a `UnitContext` whose resolved
    /// `(length, plane_angle, solid_angle)` matches the source — confirms
    /// reader/writer are paired for every supported unit flavour. Compares
    /// values via arena lookup, not raw `NamedUnitId` (which is not
    /// round-trip stable when base SI entries are present in the pool).
    #[test]
    fn non_metric_units_survive_write_then_read() {
        let cases: &[(LengthUnit, AngleUnit, SolidAngleUnit)] = &[
            (
                LengthUnit::Inch,
                AngleUnit::Radian,
                SolidAngleUnit::Steradian,
            ),
            (
                LengthUnit::Foot,
                AngleUnit::Radian,
                SolidAngleUnit::Steradian,
            ),
            (
                LengthUnit::Metre,
                AngleUnit::Degree,
                SolidAngleUnit::Steradian,
            ),
        ];
        for &(l, p, s) in cases {
            let model = model_with_units(UnitsBuilder::new(l, p, s));
            let text = model.write_to_string().expect("write");
            let graph = parse(&text).expect("re-parse");
            let back = ReaderContext::convert(&graph);
            assert!(
                back.warnings.is_empty(),
                "warnings for ({l:?}, {p:?}, {s:?}): {:#?}",
                back.warnings
            );
            assert_eq!(first_length(&back.model), Some(l), "length for ({l:?})");
            assert_eq!(first_plane_angle(&back.model), Some(p), "plane_angle");
            assert_eq!(first_solid_angle(&back.model), Some(s), "solid_angle");
        }
    }

    #[test]
    fn cbu_wrapped_metre_round_trips() {
        let model = model_with_units(
            UnitsBuilder::new(
                LengthUnit::Metre,
                AngleUnit::Radian,
                SolidAngleUnit::Steradian,
            )
            .length_self_wrap(true),
        );
        let text = model.write_to_string().expect("write");
        assert!(
            text.contains("CONVERSION_BASED_UNIT('METRE'"),
            "writer must emit CBU('METRE') for self-wrap length: {text}"
        );
        assert!(text.contains("LENGTH_MEASURE_WITH_UNIT(1."));
        assert!(text.contains("DIMENSIONAL_EXPONENTS(1."));

        let graph = parse(&text).expect("re-parse");
        let back = ReaderContext::convert(&graph);
        assert!(back.warnings.is_empty(), "{:#?}", back.warnings);
        assert_eq!(first_length(&back.model), Some(LengthUnit::Metre));
    }

    #[test]
    fn cbu_wrapped_radian_round_trips() {
        let model = model_with_units(
            UnitsBuilder::new(
                LengthUnit::Millimetre,
                AngleUnit::Radian,
                SolidAngleUnit::Steradian,
            )
            .plane_angle_self_wrap(true),
        );
        let text = model.write_to_string().expect("write");
        assert!(
            text.contains("CONVERSION_BASED_UNIT('RADIAN'"),
            "writer must emit CBU('RADIAN') for self-wrap angle: {text}"
        );

        let graph = parse(&text).expect("re-parse");
        let back = ReaderContext::convert(&graph);
        assert!(back.warnings.is_empty(), "{:#?}", back.warnings);
        assert_eq!(first_plane_angle(&back.model), Some(AngleUnit::Radian));
    }

    /// Synthetic round-trip for plane-angle / solid-angle uncertainty.
    /// No production fixture exercises these so this test pins the
    /// read/write paths against a hand-built `UnitContext`.
    #[test]
    fn angle_and_solid_angle_uncertainty_round_trip() {
        let length_unc = LengthUncertainty {
            value: 1e-7,
            name: "distance_accuracy_value".into(),
            description: "confusion accuracy".into(),
        };
        let plane_unc = LengthUncertainty {
            value: 1e-5,
            name: "angle_accuracy".into(),
            description: "angle uncertainty".into(),
        };
        let solid_unc = LengthUncertainty {
            value: 1e-3,
            name: "solid_angle_accuracy".into(),
            description: "solid angle uncertainty".into(),
        };
        let model = model_with_units(
            UnitsBuilder::new(
                LengthUnit::Millimetre,
                AngleUnit::Radian,
                SolidAngleUnit::Steradian,
            )
            .length_uncertainty(length_unc.clone())
            .plane_angle_uncertainty(plane_unc.clone())
            .solid_angle_uncertainty(solid_unc.clone()),
        );
        let text = model.write_to_string().expect("write");
        assert!(text.contains("LENGTH_MEASURE("), "{text}");
        assert!(text.contains("PLANE_ANGLE_MEASURE("), "{text}");
        assert!(text.contains("SOLID_ANGLE_MEASURE("), "{text}");
        assert!(text.contains("'angle_accuracy'"), "{text}");
        assert!(text.contains("'solid_angle_accuracy'"), "{text}");

        let graph = parse(&text).expect("re-parse");
        let back = ReaderContext::convert(&graph);
        assert!(back.warnings.is_empty(), "{:#?}", back.warnings);
        let ctx = first_ctx(&back.model).expect("ctx");
        assert_eq!(ctx.length_uncertainty.as_ref(), Some(&length_unc));
        assert_eq!(ctx.plane_angle_uncertainty.as_ref(), Some(&plane_unc));
        assert_eq!(ctx.solid_angle_uncertainty.as_ref(), Some(&solid_unc));
    }
}
