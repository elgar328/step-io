//! The model's units — the length and angle units and precision a kernel
//! needs to interpret coordinates.
//!
//! [`Scene::units`] reports them together. Each [`Unit`] carries a
//! [`to_si`](Unit::to_si) factor — to metres or radians — so a consumer
//! multiplies the file's raw coordinates by it; a field is `None` where the
//! model gives no such unit.

use crate::StepModel;
use crate::generated::model as m;
use crate::scene::{Ctx, Scene};

/// The model's unit context. Fields are `None` when the model carries no such
/// unit (or no unit context at all).
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Units {
    pub length: Option<Unit>,
    pub angle: Option<Unit>,
    /// Modelling precision (uncertainty), in the length unit.
    pub precision: Option<f64>,
}

/// A single unit: a display name and the factor to multiply a value by to reach
/// the SI base unit (metre for length, radian for angle).
#[derive(Clone, Debug)]
#[non_exhaustive]
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
    /// `None` fields rather than an error; a length or angle unit it cannot
    /// read is reported in [`Scene::warnings`], naming the unit context.
    #[must_use]
    pub fn units(&self) -> Units {
        self.ctx().units().clone()
    }
}

/// Resolve a model's units (delegated to by [`Scene::units`]; also reused by the
/// NURBS layer to read the plane-angle unit). Runs once per scene, so its
/// warnings name the unit context, not the handle that first asked.
pub(crate) fn units_of(cx: Ctx<'_>) -> Units {
    let model = cx.model;
    let found = find_unit_context(model);
    let ctx = found.map(|(_, cu)| cu);
    let subject = match found {
        Some((id, _)) => Some(m::EntityKey::ComplexEntity(id)),
        None => (!model.global_unit_assigned_context_arena.items.is_empty()).then_some(
            m::EntityKey::GlobalUnitAssignedContext(m::GlobalUnitAssignedContextId(0)),
        ),
    };
    let cx = Ctx { subject, ..cx };

    let unit_refs: &[m::UnitRef] = ctx
        .and_then(|cu| {
            cu.parts.iter().find_map(|p| match p {
                m::PartialEntity::GlobalUnitAssignedContext { units } => Some(units.as_slice()),
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
        if let Some((kind, unit)) = classify_unit(cx, ur) {
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
                m::PartialEntity::GlobalUncertaintyAssignedContext { uncertainty } => {
                    Some(uncertainty.as_slice())
                }
                _ => None,
            })
        })
        .and_then(|u| u.first())
        .and_then(|u| match u {
            m::UncertaintyMeasureWithUnitRef::UncertaintyMeasureWithUnit(id) => Some(
                model
                    .uncertainty_measure_with_unit_arena
                    .get(id.0)
                    .value_component
                    .value
                    .as_f64(),
            ),
            m::UncertaintyMeasureWithUnitRef::Complex(cuid) => {
                let value = complex_measure(model, *cuid).map(|(v, _)| v.value.as_f64());
                if value.is_none() {
                    cx.warn(
                        "uncertainty is a complex instance without a MEASURE_WITH_UNIT part"
                            .to_owned(),
                    );
                }
                value
            }
        });
    out
}

/// The first complex instance carrying a `GLOBAL_UNIT_ASSIGNED_CONTEXT` part —
/// the model's unit context (it also carries the uncertainty part).
fn find_unit_context(model: &StepModel) -> Option<(m::ComplexEntityId, &m::ComplexEntity)> {
    let arena = &model.complex_entity_arena;
    arena.indices().find_map(|i| {
        let cu = arena.get(i);
        cu.parts
            .iter()
            .any(|p| matches!(p, m::PartialEntity::GlobalUnitAssignedContext { .. }))
            .then_some((m::ComplexEntityId(i), cu))
    })
}

/// Classify a unit reference (SI, conversion-based, or complex-wrapped) into its
/// kind and SI factor.
fn classify_unit(cx: Ctx<'_>, ur: &m::UnitRef) -> Option<(UnitKind, Unit)> {
    let model = cx.model;
    match ur {
        m::UnitRef::SiUnit(id) => {
            let si = model.si_unit_arena.get(id.0);
            Some(classify_si(si.prefix, si.name))
        }
        m::UnitRef::ConversionBasedUnit(id) => {
            let cbu = model.conversion_based_unit_arena.get(id.0);
            classify_cbu(cx, model.text(cbu.name), &cbu.conversion_factor)
        }
        m::UnitRef::Complex(cuid) => {
            let cu = model.complex_entity_arena.get(cuid.0);
            let found = cu.parts.iter().find_map(|p| match p {
                m::PartialEntity::SiUnit { prefix, name } => Some(classify_si(*prefix, *name)),
                m::PartialEntity::ConversionBasedUnit {
                    name,
                    conversion_factor,
                } => classify_cbu(cx, model.text(*name), conversion_factor),
                _ => None,
            });
            let length_or_angle = cu.parts.iter().any(|p| {
                matches!(
                    p,
                    m::PartialEntity::LengthUnit | m::PartialEntity::PlaneAngleUnit
                )
            });
            if found.is_none() && length_or_angle {
                cx.warn(
                    "length or angle unit is a complex instance without an SI_UNIT or \
                     CONVERSION_BASED_UNIT part; not read"
                        .to_owned(),
                );
            }
            found
        }
        // Units that could be a length or angle but carry no factor to read.
        m::UnitRef::LengthUnit(_)
        | m::UnitRef::PlaneAngleUnit(_)
        | m::UnitRef::NamedUnit(_)
        | m::UnitRef::ContextDependentUnit(_) => {
            cx.warn(format!(
                "unit {} is not an SI or conversion-based unit; not read",
                Ctx::keyword(ur.entity_key())
            ));
            None
        }
        // Other kinds (area, mass, time, …) are not units `Units` holds.
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
    cx: Ctx<'_>,
    name: &str,
    factor: &m::MeasureWithUnitRef,
) -> Option<(UnitKind, Unit)> {
    let Some((value, base)) = measure_with_unit(cx.model, factor) else {
        cx.warn(format!(
            "CONVERSION_BASED_UNIT {name}: conversion factor is a complex instance without a \
             MEASURE_WITH_UNIT part; not read"
        ));
        return None;
    };
    let (kind, base_unit) = classify_unit(cx, base)?;
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
    let (value, unit) = measure_parts(model, r)?;
    Some((value.value.as_f64(), unit))
}

/// The value and unit of any `MEASURE_WITH_UNIT`: every subtype holds both,
/// and a complex instance holds them in its `MEASURE_WITH_UNIT` part. `None`
/// only for a complex instance without that part.
pub(crate) fn measure_parts<'m>(
    model: &'m StepModel,
    r: &m::MeasureWithUnitRef,
) -> Option<(&'m m::MeasureValue, &'m m::UnitRef)> {
    macro_rules! of {
        ($arena:ident, $i:expr) => {{
            let e = model.$arena.get($i.0);
            (&e.value_component, &e.unit_component)
        }};
    }
    Some(match r {
        m::MeasureWithUnitRef::MeasureWithUnit(i) => of!(measure_with_unit_arena, i),
        m::MeasureWithUnitRef::LengthMeasureWithUnit(i) => of!(length_measure_with_unit_arena, i),
        m::MeasureWithUnitRef::PositiveLengthMeasureWithUnit(i) => {
            of!(positive_length_measure_with_unit_arena, i)
        }
        m::MeasureWithUnitRef::PlaneAngleMeasureWithUnit(i) => {
            of!(plane_angle_measure_with_unit_arena, i)
        }
        m::MeasureWithUnitRef::PositivePlaneAngleMeasureWithUnit(i) => {
            of!(positive_plane_angle_measure_with_unit_arena, i)
        }
        m::MeasureWithUnitRef::MassMeasureWithUnit(i) => of!(mass_measure_with_unit_arena, i),
        m::MeasureWithUnitRef::RatioMeasureWithUnit(i) => of!(ratio_measure_with_unit_arena, i),
        m::MeasureWithUnitRef::MeasureRepresentationItem(i) => {
            of!(measure_representation_item_arena, i)
        }
        m::MeasureWithUnitRef::UncertaintyMeasureWithUnit(i) => {
            of!(uncertainty_measure_with_unit_arena, i)
        }
        m::MeasureWithUnitRef::Complex(cuid) => return complex_measure(model, *cuid),
    })
}

/// The value and unit of a complex instance's `MEASURE_WITH_UNIT` part, such as
/// `(LENGTH_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM()
/// MEASURE_WITH_UNIT(LENGTH_MEASURE(5.), #u) REPRESENTATION_ITEM(''))`.
pub(crate) fn complex_measure(
    model: &StepModel,
    cuid: m::ComplexEntityId,
) -> Option<(&m::MeasureValue, &m::UnitRef)> {
    model
        .complex_entity_arena
        .get(cuid.0)
        .parts
        .iter()
        .find_map(|p| match p {
            m::PartialEntity::MeasureWithUnit {
                value_component,
                unit_component,
            } => Some((value_component, unit_component)),
            _ => None,
        })
}

/// Whether a complex instance has a part that makes it a length measure.
pub(crate) fn complex_is_length_measure(model: &StepModel, cuid: m::ComplexEntityId) -> bool {
    model
        .complex_entity_arena
        .get(cuid.0)
        .parts
        .iter()
        .any(|p| {
            matches!(
                p,
                m::PartialEntity::LengthMeasureWithUnit
                    | m::PartialEntity::PositiveLengthMeasureWithUnit
            )
        })
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
