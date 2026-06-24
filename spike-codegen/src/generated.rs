//! Schema-faithful, GENERATED-STYLE model of the STEP `units` subset.
//!
//! Everything in this file is hand-written in the *shape a generator would
//! emit* from `schema/early.toml` (the spike validates the design before
//! building the generator). Each item is annotated `[GEN]` (mechanically
//! derivable from the schema) or `[HAND]` (needed human judgment) so the
//! report can estimate codegen coverage.
//!
//! Design crux being validated:
//!   * complex (multi-inheritance) instance = GENERIC part-bag (`Vec<UnitPart>`),
//!     NOT per-combination structs. Adding a unit-type marker = one more variant.
//!   * abstract-supertype ref = discriminated ref enum over the concrete arenas.
//!   * select-of-scalars = generic `{ type_name, value }`.

// ---------------------------------------------------------------------------
// ENUMs  [GEN] (one Rust enum per EXPRESS ENUM type; variants from the schema)
// ---------------------------------------------------------------------------

/// `si_prefix` ENUM. [GEN]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiPrefix {
    Exa,
    Peta,
    Tera,
    Giga,
    Mega,
    Kilo,
    Hecto,
    Deca,
    Deci,
    Centi,
    Milli,
    Micro,
    Nano,
    Pico,
    Femto,
    Atto,
}

impl SiPrefix {
    /// [GEN] parse from the un-dotted enum token.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "EXA" => Self::Exa,
            "PETA" => Self::Peta,
            "TERA" => Self::Tera,
            "GIGA" => Self::Giga,
            "MEGA" => Self::Mega,
            "KILO" => Self::Kilo,
            "HECTO" => Self::Hecto,
            "DECA" => Self::Deca,
            "DECI" => Self::Deci,
            "CENTI" => Self::Centi,
            "MILLI" => Self::Milli,
            "MICRO" => Self::Micro,
            "NANO" => Self::Nano,
            "PICO" => Self::Pico,
            "FEMTO" => Self::Femto,
            "ATTO" => Self::Atto,
            _ => return None,
        })
    }
    /// [GEN] emit as the dotted enum token.
    pub fn token(self) -> &'static str {
        match self {
            Self::Exa => ".EXA.",
            Self::Peta => ".PETA.",
            Self::Tera => ".TERA.",
            Self::Giga => ".GIGA.",
            Self::Mega => ".MEGA.",
            Self::Kilo => ".KILO.",
            Self::Hecto => ".HECTO.",
            Self::Deca => ".DECA.",
            Self::Deci => ".DECI.",
            Self::Centi => ".CENTI.",
            Self::Milli => ".MILLI.",
            Self::Micro => ".MICRO.",
            Self::Nano => ".NANO.",
            Self::Pico => ".PICO.",
            Self::Femto => ".FEMTO.",
            Self::Atto => ".ATTO.",
        }
    }
}

/// `si_unit_name` ENUM (subset present in corpus; the full schema has ~30). [GEN]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiUnitName {
    Metre,
    Gram,
    Second,
    Ampere,
    Kelvin,
    Mole,
    Candela,
    Radian,
    Steradian,
    Hertz,
    Newton,
    Pascal,
    Joule,
    Watt,
}

impl SiUnitName {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "METRE" => Self::Metre,
            "GRAM" => Self::Gram,
            "SECOND" => Self::Second,
            "AMPERE" => Self::Ampere,
            "KELVIN" => Self::Kelvin,
            "MOLE" => Self::Mole,
            "CANDELA" => Self::Candela,
            "RADIAN" => Self::Radian,
            "STERADIAN" => Self::Steradian,
            "HERTZ" => Self::Hertz,
            "NEWTON" => Self::Newton,
            "PASCAL" => Self::Pascal,
            "JOULE" => Self::Joule,
            "WATT" => Self::Watt,
            _ => return None,
        })
    }
    pub fn token(self) -> &'static str {
        match self {
            Self::Metre => ".METRE.",
            Self::Gram => ".GRAM.",
            Self::Second => ".SECOND.",
            Self::Ampere => ".AMPERE.",
            Self::Kelvin => ".KELVIN.",
            Self::Mole => ".MOLE.",
            Self::Candela => ".CANDELA.",
            Self::Radian => ".RADIAN.",
            Self::Steradian => ".STERADIAN.",
            Self::Hertz => ".HERTZ.",
            Self::Newton => ".NEWTON.",
            Self::Pascal => ".PASCAL.",
            Self::Joule => ".JOULE.",
            Self::Watt => ".WATT.",
        }
    }
}

// ---------------------------------------------------------------------------
// Arena + typed ids  [GEN]
// ---------------------------------------------------------------------------

/// A flat arena, one per concrete entity type. [GEN]
#[derive(Debug)]
pub struct Arena<T> {
    pub items: Vec<T>,
}

// Manual `Default` (derive would require `T: Default`, which entities don't need).
impl<T> Default for Arena<T> {
    fn default() -> Self {
        Arena { items: Vec::new() }
    }
}

impl<T> Arena<T> {
    pub fn push(&mut self, v: T) -> usize {
        let i = self.items.len();
        self.items.push(v);
        i
    }
    pub fn get(&self, i: usize) -> &T {
        &self.items[i]
    }
}

/// Typed id newtypes (one per arena). [GEN]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComplexUnitId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DimExpId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasureWithUnitId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivedUnitId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivedUnitElementId(pub usize);

// ---------------------------------------------------------------------------
// References to *concrete* attribute types  [GEN]
// ---------------------------------------------------------------------------

/// `named_unit.dimensions` is `dimensional_exponents` — but in the corpus
/// complex SI units write `NAMED_UNIT(*)` (DERIVED). So the ref slot must be
/// able to carry either a concrete ref or the DERIVED marker. [GEN] (the
/// `Derived` arm is implied by any attribute being able to appear as `*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimRef {
    Ref(DimExpId),
    /// `*` — derived/redeclared in the complex instance.
    Derived,
}

/// `unit` = SELECT(named_unit, derived_unit). `named_unit` is an ABSTRACT
/// supertype; its concrete instantiations in this subset are the complex SI /
/// CBU / length etc. units (all stored in `ComplexUnit`). So the discriminated
/// ref is over the concrete arenas the select can resolve to. [GEN] (the arms
/// are the schema's concrete leaf entities under the select members).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitRef {
    /// Any concrete `named_unit` instance (here all are complex instances).
    NamedUnit(ComplexUnitId),
    DerivedUnit(DerivedUnitId),
}

/// `derived_unit_element.unit` / `named_unit`-typed ref. Abstract supertype
/// ref → discriminated. [GEN]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedUnitRef {
    Complex(ComplexUnitId),
}

/// `measure_value` = SELECT of many named scalar measure types, all numeric.
/// Modeled generically as `{ type_name, value }` (the L1 select-of-scalars
/// precedent). [HAND] — the decision to collapse the whole select into a
/// generic pair (rather than ~40 variants) is a design judgment, though the
/// *member list* is mechanical.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureValue {
    /// `Some` = typed select member (e.g. `LENGTH_MEASURE(1.0)`); `None` = the
    /// value is written bare because a subtype redeclares `value_component` to
    /// a concrete REAL-based measure type (e.g. `LENGTH_MEASURE_WITH_UNIT`
    /// writes `25.4`, not `LENGTH_MEASURE(25.4)`). Preserving this distinction
    /// is required for byte-faithful write.
    pub type_name: Option<String>,
    pub value: f64,
}

// ---------------------------------------------------------------------------
// COMPLEX instance = generic part-bag  [GEN structure; the crux]
// ---------------------------------------------------------------------------

/// One sub-entity (part) of a complex unit instance. Each variant carries that
/// schema entity's OWN attributes only (inherited attrs live on the part that
/// declares them). [GEN] — variants are the concrete entities that can appear
/// as parts of a complex `named_unit`; adding a new unit-type marker = one more
/// variant, with NO combination enumeration.
#[derive(Debug, Clone, PartialEq)]
pub enum UnitPart {
    /// `NAMED_UNIT(dimensions)` — own attr `dimensions`.
    NamedUnit {
        dimensions: DimRef,
    },
    /// `SI_UNIT(prefix, name)` — own attrs.
    SiUnit {
        prefix: Option<SiPrefix>,
        name: SiUnitName,
    },
    /// `CONVERSION_BASED_UNIT(name, conversion_factor)` — own attrs.
    ConversionBasedUnit {
        name: String,
        conversion_factor: MeasureWithUnitId,
    },
    /// `CONTEXT_DEPENDENT_UNIT(name)` — own attr.
    ContextDependentUnit {
        name: String,
    },
    // --- unit-type markers (own_attrs = []) ---
    LengthUnit,
    MassUnit,
    PlaneAngleUnit,
    SolidAngleUnit,
    TimeUnit,
}

/// A complex (multi-inheritance) unit instance: an order-free bag of parts. [GEN]
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexUnit {
    pub parts: Vec<UnitPart>,
}

// ---------------------------------------------------------------------------
// SIMPLE entities = flattened structs (own + inherited in P21 order)  [GEN]
// ---------------------------------------------------------------------------

/// `dimensional_exponents` (no parent): 7 reals in declared order. [GEN]
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionalExponents {
    pub length_exponent: f64,
    pub mass_exponent: f64,
    pub time_exponent: f64,
    pub electric_current_exponent: f64,
    pub thermodynamic_temperature_exponent: f64,
    pub amount_of_substance_exponent: f64,
    pub luminous_intensity_exponent: f64,
}

/// `measure_with_unit` (and subtypes like `length_measure_with_unit` /
/// `uncertainty_measure_with_unit`). The subtype name is preserved verbatim so
/// write re-emits the exact keyword; own attrs are flattened in P21 order.
/// [GEN] for the struct; [HAND] for keeping `subtype_name` (a faithfulness
/// decision — the schema models these as distinct subtypes, and the spike
/// treats UMWU's extra attrs generically below).
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureWithUnit {
    /// Exact P21 keyword (e.g. `LENGTH_MEASURE_WITH_UNIT`).
    pub subtype_name: String,
    pub value_component: MeasureValue,
    pub unit_component: UnitRef,
    /// `uncertainty_measure_with_unit` adds `name`, `description`. Held
    /// generically so this one struct covers the whole MWU subtype tree
    /// faithfully. [HAND] — modeling subtype-extra-attrs as a tail.
    pub extra: Vec<String>,
}

/// `derived_unit` (no parent): `SET OF derived_unit_element`. The element is a
/// standalone entity (`#N=DERIVED_UNIT_ELEMENT(...)`) referenced by id, so the
/// set is a `Vec` of refs into the element arena (NOT inlined — inlining was the
/// initial wrong guess; the corpus writes both forms and the schema says it is a
/// distinct entity). [GEN]
#[derive(Debug, Clone, PartialEq)]
pub struct DerivedUnit {
    pub elements: Vec<DerivedUnitElementId>,
}

/// `derived_unit_element` (no parent), its own arena. [GEN]
#[derive(Debug, Clone, PartialEq)]
pub struct DerivedUnitElement {
    pub unit: NamedUnitRef,
    pub exponent: f64,
}

// ---------------------------------------------------------------------------
// Top-level model = one arena per entity type  [GEN]
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct SpikeUnits {
    pub complex_units: Arena<ComplexUnit>,
    pub dim_exps: Arena<DimensionalExponents>,
    pub measures: Arena<MeasureWithUnit>,
    pub derived_units: Arena<DerivedUnit>,
    pub derived_unit_elements: Arena<DerivedUnitElement>,
}
