//! The model's units — the length and angle units and precision a kernel
//! needs to interpret coordinates.
//!
//! [`Scene::units`] reports them together. Each [`Unit`] carries a
//! [`to_si`](Unit::to_si) factor — to metres or radians — so a consumer
//! multiplies the file's raw coordinates by it; a field is `None` where the
//! model gives no such unit.

use crate::StepModel;
use crate::generated::model as m;
use crate::scene::Scene;

/// The model's unit context. Fields are `None` when the model carries no such
/// unit (or no unit context at all).
#[derive(Clone, Debug, Default)]
pub struct Units {
    pub length: Option<Unit>,
    pub angle: Option<Unit>,
    /// Modelling precision (uncertainty), in the length unit.
    pub precision: Option<f64>,
}

/// A single unit: a display name and the factor to multiply a value by to reach
/// the SI base unit (metre for length, radian for angle).
#[derive(Clone, Debug)]
pub struct Unit {
    pub name: String,
    pub to_si: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UnitKind {
    Length,
    PlaneAngle,
    Other,
}

impl Scene<'_> {
    /// The model's length / angle units and precision.
    ///
    /// Picks the first unit context found; a model normally has one global
    /// context. Units it cannot classify (or a missing context) come back as
    /// `None` fields rather than an error.
    #[must_use]
    pub fn units(&self) -> Units {
        units_of(self.model())
    }
}

/// Resolve a model's units (delegated to by [`Scene::units`]; also reused by the
/// NURBS layer to read the plane-angle unit).
pub(crate) fn units_of(model: &StepModel) -> Units {
    let ctx = find_unit_context(model);

    let unit_refs: &[m::UnitRef] = ctx
        .and_then(|cu| {
            cu.parts.iter().find_map(|p| match p {
                m::UnitPart::GlobalUnitAssignedContext { units } => Some(units.as_slice()),
                _ => None,
            })
        })
        .or_else(|| {
            model
                .global_unit_assigned_context_arena
                .items
                .first()
                .map(|g| g.units.as_slice())
        })
        .unwrap_or(&[]);

    let mut out = Units::default();
    for ur in unit_refs {
        if let Some((kind, unit)) = classify_unit(model, ur) {
            match kind {
                UnitKind::Length if out.length.is_none() => out.length = Some(unit),
                UnitKind::PlaneAngle if out.angle.is_none() => out.angle = Some(unit),
                _ => {}
            }
        }
    }
    out.precision = ctx
        .and_then(|cu| {
            cu.parts.iter().find_map(|p| match p {
                m::UnitPart::GlobalUncertaintyAssignedContext { uncertainty } => {
                    Some(uncertainty.as_slice())
                }
                _ => None,
            })
        })
        .and_then(|u| u.first())
        .map(
            |m::UncertaintyMeasureWithUnitRef::UncertaintyMeasureWithUnit(id)| {
                model
                    .uncertainty_measure_with_unit_arena
                    .get(id.0)
                    .value_component
                    .value
                    .as_f64()
            },
        );
    out
}

/// The first complex instance carrying a `GLOBAL_UNIT_ASSIGNED_CONTEXT` facet —
/// the model's unit context (it also carries the uncertainty facet).
fn find_unit_context(model: &StepModel) -> Option<&m::ComplexUnit> {
    model.complex_unit_arena.items.iter().find(|cu| {
        cu.parts
            .iter()
            .any(|p| matches!(p, m::UnitPart::GlobalUnitAssignedContext { .. }))
    })
}

/// Classify a unit reference (SI, conversion-based, or complex-wrapped) into its
/// kind and SI factor.
fn classify_unit(model: &StepModel, ur: &m::UnitRef) -> Option<(UnitKind, Unit)> {
    match ur {
        m::UnitRef::SiUnit(id) => {
            let si = model.si_unit_arena.get(id.0);
            Some(classify_si(si.prefix, si.name))
        }
        m::UnitRef::ConversionBasedUnit(id) => {
            let cbu = model.conversion_based_unit_arena.get(id.0);
            classify_cbu(model, &cbu.name, &cbu.conversion_factor)
        }
        m::UnitRef::Complex(cuid) => {
            let cu = model.complex_unit_arena.get(cuid.0);
            cu.parts.iter().find_map(|p| match p {
                m::UnitPart::SiUnit { prefix, name } => Some(classify_si(*prefix, *name)),
                m::UnitPart::ConversionBasedUnit {
                    name,
                    conversion_factor,
                } => classify_cbu(model, name, conversion_factor),
                _ => None,
            })
        }
        _ => None,
    }
}

/// An SI unit's kind and factor: the kind from the base name, the factor from
/// the decimal prefix (base SI is 1.0).
fn classify_si(prefix: Option<m::SiPrefix>, name: m::SiUnitName) -> (UnitKind, Unit) {
    let kind = match name {
        m::SiUnitName::Metre => UnitKind::Length,
        m::SiUnitName::Radian => UnitKind::PlaneAngle,
        _ => UnitKind::Other,
    };
    let display = format!(
        "{}{:?}",
        prefix.map(|p| format!("{p:?}")).unwrap_or_default(),
        name
    )
    .to_uppercase();
    (
        kind,
        Unit {
            name: display,
            to_si: si_prefix_factor(prefix),
        },
    )
}

/// A conversion-based unit (e.g. inch, degree): its factor is the conversion
/// value times the base unit's own factor to SI; its kind comes from the base.
fn classify_cbu(
    model: &StepModel,
    name: &str,
    factor: &m::MeasureWithUnitRef,
) -> Option<(UnitKind, Unit)> {
    let (value, base) = measure_with_unit(model, factor)?;
    let (kind, base_unit) = classify_unit(model, base)?;
    Some((
        kind,
        Unit {
            name: name.to_uppercase(),
            to_si: value * base_unit.to_si,
        },
    ))
}

/// Resolve a `MEASURE_WITH_UNIT` (or a length/angle subtype) to its value and
/// the unit it is expressed in.
fn measure_with_unit<'m>(
    model: &'m StepModel,
    r: &'m m::MeasureWithUnitRef,
) -> Option<(f64, &'m m::UnitRef)> {
    let (value, unit) = match r {
        m::MeasureWithUnitRef::MeasureWithUnit(i) => {
            let e = model.measure_with_unit_arena.get(i.0);
            (&e.value_component, &e.unit_component)
        }
        m::MeasureWithUnitRef::LengthMeasureWithUnit(i) => {
            let e = model.length_measure_with_unit_arena.get(i.0);
            (&e.value_component, &e.unit_component)
        }
        m::MeasureWithUnitRef::PlaneAngleMeasureWithUnit(i) => {
            let e = model.plane_angle_measure_with_unit_arena.get(i.0);
            (&e.value_component, &e.unit_component)
        }
        _ => return None,
    };
    Some((value.value.as_f64(), unit))
}

/// Decimal SI prefix as a multiplier (no prefix = 1.0).
fn si_prefix_factor(prefix: Option<m::SiPrefix>) -> f64 {
    use m::SiPrefix::{
        Atto, Centi, Deca, Deci, Exa, Femto, Giga, Hecto, Kilo, Mega, Micro, Milli, Nano, Peta,
        Pico, Tera,
    };
    match prefix {
        None => 1.0,
        Some(Exa) => 1e18,
        Some(Peta) => 1e15,
        Some(Tera) => 1e12,
        Some(Giga) => 1e9,
        Some(Mega) => 1e6,
        Some(Kilo) => 1e3,
        Some(Hecto) => 1e2,
        Some(Deca) => 1e1,
        Some(Deci) => 1e-1,
        Some(Centi) => 1e-2,
        Some(Milli) => 1e-3,
        Some(Micro) => 1e-6,
        Some(Nano) => 1e-9,
        Some(Pico) => 1e-12,
        Some(Femto) => 1e-15,
        Some(Atto) => 1e-18,
    }
}
