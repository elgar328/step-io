//! Helpers shared by the four unit-leaf handlers
//! (`LengthUnitHandler` / `PlaneAngleUnitHandler` / `SolidAngleUnitHandler` /
//! `MassUnitHandler`).
//!
//! Reader side: SI / `CONVERSION_BASED_UNIT` matchers and the shared
//! `read_conversion_based_unit_body` covering the CBU branch (length /
//! plane-angle / mass — solid-angle CBU forms are unobserved).
//!
//! Writer side: cached `DIMENSIONAL_EXPONENTS` emitters used by every
//! leaf when it produces an explicit `NAMED_UNIT.dimensions`
//! (ABC-tier loyalty) — the cache fields live on `WriteBuffer`, so the
//! helpers take `&mut WriteBuffer`.

use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::id::{MeasureWithUnitId, NamedUnitId};
use crate::ir::shape_rep::{AngleUnit, LengthUnit};
use crate::ir::units::MassUnit;
use crate::parser::entity::{Attribute, EntityGraph, RawEntity, RawEntityPart};
use crate::reader::{ReaderContext, find_part_attrs, has_all_parts, require_part_attrs};
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

/// Resolved CBU outer back-references (units-CBU-① preservation): the base SI
/// `NamedUnit` and the preserved `conversion_factor` `MEASURE_WITH_UNIT`. Both
/// are already lowered when `read_conversion_based_unit_body` runs (topo: the
/// embedded MWU and its base SI are dependencies of the CBU outer), so they
/// resolve through `id_cache` inline — no `backfill_cbu_base` post-pass.
pub(super) struct CbuFactorRefs {
    pub(super) cbu_base: Option<NamedUnitId>,
    pub(super) cbu_factor_mwu_id: Option<MeasureWithUnitId>,
}

pub(super) fn has_part(parts: &[RawEntityPart], name: &str) -> bool {
    has_all_parts(parts, &[name])
}

/// Normalize a bare (untyped) numeric `value_component` to the typed measure
/// form the generated `bind_measure_value` (a `measure_value` SELECT bind)
/// expects. The typed `*_MEASURE_WITH_UNIT` subtypes redeclare `value_component`
/// to a concrete measure REAL, so some exporters write the standard plain
/// `25.4` rather than the supertype-style `LENGTH_MEASURE(25.4)` (e.g. a
/// `CONVERSION_BASED_UNIT` conversion factor, NIST `fillet_box`). The SELECT
/// bind only matches the typed form, so the handler rewrites the plain
/// real/integer to the subtype's measure type before binding (reader-side
/// leniency; the generated bind stays strict). Typed inputs pass through
/// unchanged.
pub(super) fn normalize_bare_measure_attrs(
    attrs: &[Attribute],
    measure_type: &str,
) -> Vec<Attribute> {
    let mut out = attrs.to_vec();
    if let Some(first @ (Attribute::Real(_) | Attribute::Integer(_))) = out.first().cloned() {
        out[0] = Attribute::Typed {
            type_name: measure_type.into(),
            value: Box::new(first),
        };
    }
    out
}

pub(super) fn match_length_conversion(upper_name: &str) -> Option<LengthUnit> {
    match upper_name {
        "INCH" => Some(LengthUnit::Inch),
        "FOOT" => Some(LengthUnit::Foot),
        // Some AP242 / ABC exports wrap SI units in a CONVERSION_BASED_UNIT.
        // Self-wrap is represented structurally via `cbu_base = Some(<base_id>)`
        // with `outer.unit == base.unit`; the writer reproduces the wrapper
        // by virtue of `cbu_base` being `Some`.
        "MILLIMETRE" => Some(LengthUnit::Millimetre),
        "CENTIMETRE" => Some(LengthUnit::Centimetre),
        "METRE" => Some(LengthUnit::Metre),
        _ => None,
    }
}

pub(super) fn match_angle_conversion(upper_name: &str) -> Option<AngleUnit> {
    match upper_name {
        "DEGREE" | "DEGREES" => Some(AngleUnit::Degree),
        "RADIAN" | "RADIANS" => Some(AngleUnit::Radian),
        _ => None,
    }
}

pub(super) fn match_mass_conversion(upper_name: &str) -> Option<MassUnit> {
    match upper_name {
        "POUND" => Some(MassUnit::Pound),
        // gram defined as a CONVERSION_BASED_UNIT (0.001 of the SI kg) —
        // a genuine conversion, like INCH for length.
        "GRAM" => Some(MassUnit::Gram),
        // metric tonne = 1000 × SI kg, in CBU form.
        "TON" => Some(MassUnit::Ton),
        _ => None,
    }
}

/// Which flavour's CBU branch is being read. Replaces the prior asymmetric
/// `(is_length, is_plane_angle)` bool pair on
/// [`read_conversion_based_unit_body`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CbuFlavor {
    Length,
    PlaneAngle,
    Mass,
}

/// Read a `CONVERSION_BASED_UNIT.conversion_factor` MWU's scalar factor from
/// the graph. The MWU is a `*_MEASURE_WITH_UNIT` whose attr[0] is a typed real
/// (`PLANE_ANGLE_MEASURE(0.01745)` / `MASS_MEASURE(0.4536)` / …). Mirrors the
/// typed-real shape the typed `*_MEASURE_WITH_UNIT` binds use / `backfill_cbu_base`.
fn cbu_factor(graph: &EntityGraph, mwu_ref: u64) -> Option<f64> {
    let RawEntity::Simple { attributes, .. } = graph.entities.get(&mwu_ref)? else {
        return None;
    };
    if let Some(Attribute::Typed { value, .. }) = attributes.first()
        && let Attribute::Real(v) = value.as_ref()
    {
        Some(*v)
    } else {
        None
    }
}

/// Relative-tolerance compare for CBU conversion factors (source files vary in
/// printed precision; the canonical constants are exact).
fn factor_eq(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * b.abs()
}

/// Identify a plane-angle unit by its conversion factor to the SI base
/// (radian, the only SI plane-angle unit — so the factor is an unambiguous
/// identity). π/180 → Degree, 1.0 → Radian.
fn match_angle_by_factor(factor: f64) -> Option<AngleUnit> {
    if factor_eq(factor, std::f64::consts::PI / 180.0) {
        Some(AngleUnit::Degree)
    } else if factor_eq(factor, 1.0) {
        Some(AngleUnit::Radian)
    } else {
        None
    }
}

/// Identify a mass unit by its conversion factor to the SI base (kilogram).
/// `0.453_592_37` → Pound, `0.001` → Gram, `1000.0` → Ton. (Length is excluded
/// from factor matching: its CBU base varies — millimetre vs metre — so the
/// factor is not a base-free identity.)
fn match_mass_by_factor(factor: f64) -> Option<MassUnit> {
    if factor_eq(factor, 0.453_592_37) {
        Some(MassUnit::Pound)
    } else if factor_eq(factor, 0.001) {
        Some(MassUnit::Gram)
    } else if factor_eq(factor, 1000.0) {
        Some(MassUnit::Ton)
    } else {
        None
    }
}

/// Reader body shared by `LengthUnitHandler` / `PlaneAngleUnitHandler` /
/// `MassUnitHandler` for the `CONVERSION_BASED_UNIT` branch. The flavour
/// selector picks the right matcher and per-flavour bookkeeping. For the
/// fixed-SI-base flavours (plane-angle → radian, mass → kilogram) the unit is
/// identified **by conversion factor first, name second** — a non-standard
/// name (e.g. a degree unit named `'MIAU'`) is normalized to the standard
/// unit when its factor matches. Length keeps name-matching (its base varies).
/// Unrecognised CBUs are dropped with a warning and return `None` (the outer
/// never reaches `NamedUnit` registration). A recognised CBU returns the
/// resolved [`CbuFactorRefs`] (base SI + preserved factor MWU), which the
/// caller threads into `register_named_*`.
pub(super) fn read_conversion_based_unit_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    parts: &[RawEntityPart],
    flavor: CbuFlavor,
    graph: &EntityGraph,
) -> Result<Option<CbuFactorRefs>, ConvertError> {
    let cbu_attrs = require_part_attrs(parts, "CONVERSION_BASED_UNIT", entity_id)?;
    check_count(cbu_attrs, 2, entity_id, "CONVERSION_BASED_UNIT")?;
    let name = read_string_or_unset(cbu_attrs, 0, entity_id, "name")?;
    let upper = name.to_uppercase();
    let mwu_ref = match cbu_attrs.get(1) {
        Some(Attribute::EntityRef(r)) => Some(*r),
        _ => None,
    };
    let factor = mwu_ref.and_then(|r| cbu_factor(graph, r));

    let recognised = match flavor {
        CbuFlavor::Length => {
            if let Some(unit) = match_length_conversion(&upper) {
                ctx.length_unit_map.insert(entity_id, unit);
                true
            } else {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("unsupported CONVERSION_BASED_UNIT length name: {name:?}"),
                });
                false
            }
        }
        CbuFlavor::PlaneAngle => {
            // NsCase::CbuAngleFactor anonymizers: a non-standard CBU name no
            // longer identifies the unit → identify by conversion factor, name
            // fallback; warn on disagreement. See reader::nonstandard.
            let by_name = match_angle_conversion(&upper);
            if let Some(unit) = factor.and_then(match_angle_by_factor).or(by_name) {
                ctx.angle_unit_map.insert(entity_id, unit);
                if by_name != Some(unit) {
                    let normalized_to = match unit {
                        AngleUnit::Degree => "DEGREE",
                        AngleUnit::Radian => "RADIAN",
                    };
                    ctx.ns_push(
                        crate::reader::NsCase::CbuAngleFactor,
                        format!("CONVERSION_BASED_UNIT.name ({name:?})"),
                        1,
                        normalized_to.into(),
                    );
                }
                true
            } else {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("unsupported CONVERSION_BASED_UNIT angle name: {name:?}"),
                });
                false
            }
        }
        CbuFlavor::Mass => {
            // NsCase::CbuMassFactor anonymizers: as NsCase::CbuAngleFactor, for
            // mass (fixed kg base). See reader::nonstandard.
            let by_name = match_mass_conversion(&upper);
            if let Some(unit) = factor.and_then(match_mass_by_factor).or(by_name) {
                ctx.mass_unit_map.insert(entity_id, unit);
                if by_name != Some(unit) {
                    let normalized_to = match unit {
                        MassUnit::Pound => "POUND",
                        MassUnit::Gram => "GRAM",
                        MassUnit::Ton => "TON",
                        MassUnit::Kilogram => "KILOGRAM",
                        // Megagram is plain SI only — never a CBU conversion
                        // result; unreachable on this path.
                        MassUnit::Megagram => "MEGAGRAM",
                    };
                    ctx.ns_push(
                        crate::reader::NsCase::CbuMassFactor,
                        format!("CONVERSION_BASED_UNIT.name ({name:?})"),
                        1,
                        normalized_to.into(),
                    );
                }
                true
            } else {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!("unsupported CONVERSION_BASED_UNIT mass name: {name:?}"),
                });
                false
            }
        }
    };
    if recognised && let Some(r) = mwu_ref {
        Ok(Some(resolve_cbu_factor_refs(ctx, r, graph)))
    } else {
        Ok(None)
    }
}

/// Resolve a recognised CBU outer's [`CbuFactorRefs`] from its
/// `conversion_factor` MWU ref. The MWU is preserved in `mwu_arena` (its
/// `MeasureWithUnitId`) and its `unit_component` is the base SI's
/// `NamedUnitId` — both already in `id_cache` (topo dependencies).
fn resolve_cbu_factor_refs(
    ctx: &ReaderContext,
    mwu_ref: u64,
    graph: &EntityGraph,
) -> CbuFactorRefs {
    let cbu_factor_mwu_id = ctx.id_cache.get::<MeasureWithUnitId>(mwu_ref);
    let cbu_base = graph
        .entities
        .get(&mwu_ref)
        .and_then(|e| match e {
            RawEntity::Simple { attributes, .. } => attributes.iter().find_map(|a| match a {
                Attribute::EntityRef(b) => Some(*b),
                _ => None,
            }),
            RawEntity::Complex { .. } => None,
        })
        .and_then(|base_eid| ctx.id_cache.get::<NamedUnitId>(base_eid));
    CbuFactorRefs {
        cbu_base,
        cbu_factor_mwu_id,
    }
}

/// Emit the length-flavour `DIMENSIONAL_EXPONENTS` (1, 0, 0, 0, 0, 0, 0)
/// once per `WriteBuffer` and cache the step id (units-3c dedup); later
/// callers receive the cached id.
pub(super) fn emit_length_dim_exponents(buf: &mut WriteBuffer) -> u64 {
    if let Some(id) = buf.length_dim_exp_step {
        return id;
    }
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
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
    buf.length_dim_exp_step = Some(n);
    n
}

/// Emit the dimensionless `DIMENSIONAL_EXPONENTS` (0, 0, 0, 0, 0, 0, 0)
/// once per `WriteBuffer` and cache the step id.
pub(super) fn emit_dimensionless_exponents(buf: &mut WriteBuffer) -> u64 {
    if let Some(id) = buf.dimensionless_dim_exp_step {
        return id;
    }
    let n = buf.fresh();
    buf.entities.push(WriterEntity {
        id: n,
        body: WriterBody::Simple {
            name: "DIMENSIONAL_EXPONENTS".into(),
            attrs: vec![Attribute::Real(0.0); 7],
        },
    });
    buf.dimensionless_dim_exp_step = Some(n);
    n
}

/// Read `NAMED_UNIT.dimensions` field of a unit complex (phase
/// dim-exp-arena-c). Returns `Some(id)` when the source emitted an
/// explicit `DIMENSIONAL_EXPONENTS` ref, `None` for the `*` (Derived)
/// form. Unknown refs (cross-cascade) silently degrade to `None`.
pub(crate) fn read_named_unit_dim_exp(
    ctx: &ReaderContext,
    parts: &[crate::parser::entity::RawEntityPart],
) -> Option<crate::ir::DimensionalExponentsId> {
    let attrs = find_part_attrs(parts, "NAMED_UNIT")?;
    match attrs.first()? {
        Attribute::EntityRef(n) => ctx
            .id_cache
            .get::<crate::ir::id::DimensionalExponentsId>(*n),
        _ => None,
    }
}
