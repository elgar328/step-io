//! `pmi` pool — Product Manufacturing Information (GD&T) entities.
//!
//! Phase pmi-primitives seeds the pool with three dependency-free leaf
//! `single_struct` entities (tolerance / qualifier primitives). Future
//! phases add the tolerance / datum / geometric-tolerance entities that
//! consume them. `SHAPE_ASPECT` and its subtypes are *not* here — the
//! ir.toml blueprint puts them in the `shape_rep` pool.

use super::arena::Arena;

/// Top-level container for `pmi`-pool entities. `None` when the source
/// file carried no PMI content.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PmiPool {
    /// `TOLERANCE_ZONE_FORM` arena.
    pub tolerance_zone_forms: Arena<ToleranceZoneForm>,
    /// `TYPE_QUALIFIER` arena.
    pub type_qualifiers: Arena<TypeQualifier>,
    /// `VALUE_FORMAT_TYPE_QUALIFIER` arena.
    pub value_format_type_qualifiers: Arena<ValueFormatTypeQualifier>,
}

/// `TOLERANCE_ZONE_FORM(name)` — names a tolerance zone's geometric form
/// (e.g. `'cylindrical or circular'`).
#[derive(Debug, Clone, PartialEq)]
pub struct ToleranceZoneForm {
    pub name: String,
}

/// `TYPE_QUALIFIER(name)` — a qualifier naming a measure's type
/// (e.g. `'maximum'`).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeQualifier {
    pub name: String,
}

/// `VALUE_FORMAT_TYPE_QUALIFIER(format_type)` — a qualifier giving a
/// value's display format (e.g. `'NR2 1.3'`).
#[derive(Debug, Clone, PartialEq)]
pub struct ValueFormatTypeQualifier {
    pub format_type: String,
}
