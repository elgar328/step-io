//! Attribute extraction utilities for converting raw `Vec<Attribute>` values
//! into typed Rust values. Every function is pure (no state) and independently
//! testable.

use crate::parser::entity::Attribute;

use super::error::{AttributeKindTag, ConvertError};

/// Verify the attribute list has exactly `expected` entries.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeCount`] on mismatch.
pub fn check_count(
    attrs: &[Attribute],
    expected: usize,
    entity_id: u64,
    entity_name: &str,
) -> Result<(), ConvertError> {
    if attrs.len() == expected {
        Ok(())
    } else {
        Err(ConvertError::AttributeCount {
            entity_id,
            entity_name: entity_name.to_string(),
            expected,
            actual: attrs.len(),
        })
    }
}

// ---------------------------------------------------------------------------
// Internal helper: bounds-checked attribute access
// ---------------------------------------------------------------------------

fn get_attr<'a>(
    attrs: &'a [Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<&'a Attribute, ConvertError> {
    attrs.get(index).ok_or(ConvertError::AttributeIndex {
        entity_id,
        field_name,
        index,
        len: attrs.len(),
    })
}

// ---------------------------------------------------------------------------
// Scalar extractors
// ---------------------------------------------------------------------------

/// Extract an `f64` from `attrs[index]`.
///
/// Accepts both `Attribute::Real` and `Attribute::Integer` (promoted to f64).
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`] or [`ConvertError::AttributeType`].
pub fn read_real(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<f64, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    match attr {
        Attribute::Real(v) => Ok(*v),
        // Integer promotion — common in real STEP files (0 instead of 0.0).
        #[allow(clippy::cast_precision_loss)]
        Attribute::Integer(v) => Ok(*v as f64),
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "Real",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}

/// Extract an `i64` from `attrs[index]`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`] or [`ConvertError::AttributeType`].
pub fn read_integer(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<i64, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    match attr {
        Attribute::Integer(v) => Ok(*v),
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "Integer",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}

/// Extract an optional integer from `attrs[index]`. `$` maps to `None`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`] or [`ConvertError::AttributeType`].
pub fn read_optional_integer(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Option<i64>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    match attr {
        Attribute::Integer(v) => Ok(Some(*v)),
        Attribute::Unset => Ok(None),
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "Integer or Unset",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}

/// Extract an optional string from `attrs[index]`. `$` maps to `None`.
///
/// Use this when the schema field is `OPTIONAL STRING` and the IR needs
/// to distinguish `$` from `''` (the lossy collapse of `read_string_or_unset`).
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`] or [`ConvertError::AttributeType`].
pub fn read_optional_string(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Option<String>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    match attr {
        Attribute::String(s) => Ok(Some(s.clone())),
        Attribute::Unset => Ok(None),
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "String or Unset",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}

/// Extract an optional string list from `attrs[index]`. `$` maps to `None`,
/// `(...)` maps to `Some(vec![...])`, including the empty list `()`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`] or [`ConvertError::AttributeType`].
pub fn read_optional_string_list(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Option<Vec<String>>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    match attr {
        Attribute::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Attribute::String(s) => out.push(s.clone()),
                    other => {
                        return Err(ConvertError::AttributeType {
                            entity_id,
                            field_name,
                            expected: "String",
                            actual: AttributeKindTag::from_attribute(other),
                        });
                    }
                }
            }
            Ok(Some(out))
        }
        Attribute::Unset => Ok(None),
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "List or Unset",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}

/// Extract an optional real from `attrs[index]`. `$` maps to `None`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`] or [`ConvertError::AttributeType`].
pub fn read_optional_real(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Option<f64>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    match attr {
        Attribute::Real(v) => Ok(Some(*v)),
        #[allow(clippy::cast_precision_loss)]
        Attribute::Integer(v) => Ok(Some(*v as f64)),
        Attribute::Unset => Ok(None),
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "Real or Unset",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}

/// Extract a string reference from `attrs[index]`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`] or [`ConvertError::AttributeType`].
pub fn read_string<'a>(
    attrs: &'a [Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<&'a str, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    match attr {
        Attribute::String(s) => Ok(s.as_str()),
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "String",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}

/// Extract the hex-encoded `&str` of a `binary` attribute from `attrs[index]`.
///
/// The lexer already strips the surrounding quotes and keeps the hex digits
/// verbatim ([`Attribute::Binary`]); this returns them unchanged for a faithful
/// round-trip (decoding to bytes is a later, semantic concern).
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`] or [`ConvertError::AttributeType`].
pub fn read_binary<'a>(
    attrs: &'a [Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<&'a str, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    match attr {
        Attribute::Binary(s) => Ok(s.as_str()),
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "Binary",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}

/// Extract a `&str` from `attrs[index]`, treating `$` (Unset) as `""`.
///
/// STEP spec marks some informal string fields (descriptions, labels,
/// user-facing identifiers) as non-optional, but many CAD producers —
/// Fusion 360 notably, `STEPcode`, `ST-Developer` — emit `$` for "no value".
/// This helper accepts both `$` and empty strings, returning `""` so the
/// caller's existing `is_empty()` / `Option` normalization works unchanged.
/// Wrong types (`Enum`, `Real`, …) still error.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`] or [`ConvertError::AttributeType`].
pub fn read_string_or_unset<'a>(
    attrs: &'a [Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<&'a str, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    match attr {
        Attribute::String(s) => Ok(s.as_str()),
        Attribute::Unset => Ok(""),
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "String",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}

/// Extract an entity reference `#N` from `attrs[index]`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`] or [`ConvertError::AttributeType`].
pub fn read_entity_ref(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<u64, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    match attr {
        Attribute::EntityRef(id) => Ok(*id),
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "EntityRef",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}

/// Extract an optional entity reference from `attrs[index]`.
///
/// Returns `Ok(None)` for `Attribute::Unset` (`$`) or `Attribute::Derived`
/// (`*`). This handles STEP optional attributes such as
/// `AXIS2_PLACEMENT_3D`'s axis and `ref_direction`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`] or [`ConvertError::AttributeType`].
pub fn read_optional_entity_ref(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Option<u64>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    match attr {
        Attribute::EntityRef(id) => Ok(Some(*id)),
        Attribute::Unset | Attribute::Derived => Ok(None),
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "EntityRef or $/*",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}

/// Extract an enum value (the inner string) from `attrs[index]`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`] or [`ConvertError::AttributeType`].
pub fn read_enum<'a>(
    attrs: &'a [Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<&'a str, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    match attr {
        Attribute::Enum(s) => Ok(s.as_str()),
        other => Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "Enum",
            actual: AttributeKindTag::from_attribute(other),
        }),
    }
}

// ---------------------------------------------------------------------------
// List extractors
// ---------------------------------------------------------------------------

/// Extract a list of `f64` from `attrs[index]`.
///
/// Expects `Attribute::List` containing only `Real` or `Integer` items.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeType`] if the attribute is not a list or
/// contains non-numeric items.
#[allow(clippy::cast_precision_loss)]
pub fn read_real_list(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Vec<f64>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    let items = match attr {
        Attribute::List(items) => items,
        other => {
            return Err(ConvertError::AttributeType {
                entity_id,
                field_name,
                expected: "List",
                actual: AttributeKindTag::from_attribute(other),
            });
        }
    };
    let mut result = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Attribute::Real(v) => result.push(*v),
            Attribute::Integer(v) => result.push(*v as f64),
            other => {
                return Err(ConvertError::AttributeType {
                    entity_id,
                    field_name,
                    expected: "Real (inside list)",
                    actual: AttributeKindTag::from_attribute(other),
                });
            }
        }
    }
    Ok(result)
}

/// Extract a list of entity references from `attrs[index]`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeType`] if the attribute is not a list or
/// contains non-`EntityRef` items.
pub fn read_entity_ref_list(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Vec<u64>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    let items = match attr {
        Attribute::List(items) => items,
        other => {
            return Err(ConvertError::AttributeType {
                entity_id,
                field_name,
                expected: "List",
                actual: AttributeKindTag::from_attribute(other),
            });
        }
    };
    let mut result = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Attribute::EntityRef(id) => result.push(*id),
            other => {
                return Err(ConvertError::AttributeType {
                    entity_id,
                    field_name,
                    expected: "EntityRef (inside list)",
                    actual: AttributeKindTag::from_attribute(other),
                });
            }
        }
    }
    Ok(result)
}

/// Extract a list of strings from `attrs[index]`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeType`] if the attribute is not a list or
/// contains non-`String` items.
pub fn read_string_list(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Vec<String>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    let items = match attr {
        Attribute::List(items) => items,
        other => {
            return Err(ConvertError::AttributeType {
                entity_id,
                field_name,
                expected: "List",
                actual: AttributeKindTag::from_attribute(other),
            });
        }
    };
    let mut result = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Attribute::String(s) => result.push(s.clone()),
            other => {
                return Err(ConvertError::AttributeType {
                    entity_id,
                    field_name,
                    expected: "String (inside list)",
                    actual: AttributeKindTag::from_attribute(other),
                });
            }
        }
    }
    Ok(result)
}

/// Extract a list of `i64` from `attrs[index]`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeType`] if the attribute is not a list or
/// contains non-integer items.
pub fn read_integer_list(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Vec<i64>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    let items = match attr {
        Attribute::List(items) => items,
        other => {
            return Err(ConvertError::AttributeType {
                entity_id,
                field_name,
                expected: "List",
                actual: AttributeKindTag::from_attribute(other),
            });
        }
    };
    let mut result = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Attribute::Integer(v) => result.push(*v),
            other => {
                return Err(ConvertError::AttributeType {
                    entity_id,
                    field_name,
                    expected: "Integer (inside list)",
                    actual: AttributeKindTag::from_attribute(other),
                });
            }
        }
    }
    Ok(result)
}

/// Extract a 2D grid of entity references from `attrs[index]`.
///
/// Expects `Attribute::List` of `Attribute::List`s of `Attribute::EntityRef`.
/// All inner lists must have the same length (rectangular grid).
///
/// # Errors
///
/// Returns [`ConvertError::AttributeType`] if the structure is not a
/// rectangular grid of entity references.
pub fn read_entity_ref_grid(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Vec<Vec<u64>>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    let rows = match attr {
        Attribute::List(rows) => rows,
        other => {
            return Err(ConvertError::AttributeType {
                entity_id,
                field_name,
                expected: "List (2D grid)",
                actual: AttributeKindTag::from_attribute(other),
            });
        }
    };
    let mut grid = Vec::with_capacity(rows.len());
    let mut expected_cols = None;
    for row in rows {
        let cols = match row {
            Attribute::List(cols) => cols,
            other => {
                return Err(ConvertError::AttributeType {
                    entity_id,
                    field_name,
                    expected: "List (inner row of 2D grid)",
                    actual: AttributeKindTag::from_attribute(other),
                });
            }
        };
        // Verify rectangular grid.
        if let Some(expected) = expected_cols {
            if cols.len() != expected {
                return Err(ConvertError::DimensionMismatch {
                    entity_id,
                    field_name,
                    expected,
                    actual: cols.len(),
                });
            }
        } else {
            expected_cols = Some(cols.len());
        }
        let mut row_ids = Vec::with_capacity(cols.len());
        for col in cols {
            match col {
                Attribute::EntityRef(id) => row_ids.push(*id),
                other => {
                    return Err(ConvertError::AttributeType {
                        entity_id,
                        field_name,
                        expected: "EntityRef (inside 2D grid)",
                        actual: AttributeKindTag::from_attribute(other),
                    });
                }
            }
        }
        grid.push(row_ids);
    }
    Ok(grid)
}

/// Extract a 2D grid of `f64` from `attrs[index]`.
///
/// Expects `Attribute::List` of `Attribute::List`s of `Attribute::Real`
/// (with integer promotion). All inner lists must have the same length.
///
/// Used for `RATIONAL_B_SPLINE_SURFACE` weights.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeType`] or [`ConvertError::DimensionMismatch`].
#[allow(clippy::cast_precision_loss)]
pub fn read_real_grid(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Vec<Vec<f64>>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    let rows = match attr {
        Attribute::List(rows) => rows,
        other => {
            return Err(ConvertError::AttributeType {
                entity_id,
                field_name,
                expected: "List (2D grid)",
                actual: AttributeKindTag::from_attribute(other),
            });
        }
    };
    let mut grid = Vec::with_capacity(rows.len());
    let mut expected_cols = None;
    for row in rows {
        let cols = match row {
            Attribute::List(cols) => cols,
            other => {
                return Err(ConvertError::AttributeType {
                    entity_id,
                    field_name,
                    expected: "List (inner row of 2D grid)",
                    actual: AttributeKindTag::from_attribute(other),
                });
            }
        };
        if let Some(expected) = expected_cols {
            if cols.len() != expected {
                return Err(ConvertError::DimensionMismatch {
                    entity_id,
                    field_name,
                    expected,
                    actual: cols.len(),
                });
            }
        } else {
            expected_cols = Some(cols.len());
        }
        let mut row_values = Vec::with_capacity(cols.len());
        for col in cols {
            match col {
                Attribute::Real(v) => row_values.push(*v),
                Attribute::Integer(v) => row_values.push(*v as f64),
                other => {
                    return Err(ConvertError::AttributeType {
                        entity_id,
                        field_name,
                        expected: "Real (inside 2D grid)",
                        actual: AttributeKindTag::from_attribute(other),
                    });
                }
            }
        }
        grid.push(row_values);
    }
    Ok(grid)
}

/// Extract a ragged list-of-integer-lists from `attrs[index]` —
/// `Vec<Vec<i64>>`. Unlike [`read_real_grid`] the inner rows may differ in
/// length (e.g. triangle strips / fans of varying size).
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`] or [`ConvertError::AttributeType`].
pub fn read_integer_grid(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Vec<Vec<i64>>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    let rows = match attr {
        Attribute::List(rows) => rows,
        other => {
            return Err(ConvertError::AttributeType {
                entity_id,
                field_name,
                expected: "List (list of integer lists)",
                actual: AttributeKindTag::from_attribute(other),
            });
        }
    };
    let mut grid = Vec::with_capacity(rows.len());
    for row in rows {
        let cols = match row {
            Attribute::List(cols) => cols,
            other => {
                return Err(ConvertError::AttributeType {
                    entity_id,
                    field_name,
                    expected: "List (inner integer row)",
                    actual: AttributeKindTag::from_attribute(other),
                });
            }
        };
        let mut row_values = Vec::with_capacity(cols.len());
        for col in cols {
            match col {
                Attribute::Integer(v) => row_values.push(*v),
                other => {
                    return Err(ConvertError::AttributeType {
                        entity_id,
                        field_name,
                        expected: "Integer (inside integer grid)",
                        actual: AttributeKindTag::from_attribute(other),
                    });
                }
            }
        }
        grid.push(row_values);
    }
    Ok(grid)
}

/// Extract a 3D entity-ref grid (`LIST OF LIST OF LIST OF <entity>`) from
/// `attrs[index]` — one extra nesting level over [`read_entity_ref_grid`], for
/// B-spline *volume* control points. Each level must be a [`Attribute::List`]
/// and rectangular (uniform inner lengths); the innermost element is an
/// [`Attribute::EntityRef`].
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`], [`ConvertError::AttributeType`], or
/// [`ConvertError::DimensionMismatch`].
pub fn read_entity_ref_grid3(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Vec<Vec<Vec<u64>>>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    let planes = match attr {
        Attribute::List(planes) => planes,
        other => {
            return Err(ConvertError::AttributeType {
                entity_id,
                field_name,
                expected: "List (3D grid)",
                actual: AttributeKindTag::from_attribute(other),
            });
        }
    };
    let mut out = Vec::with_capacity(planes.len());
    let mut expected_rows = None;
    for plane in planes {
        let rows = match plane {
            Attribute::List(rows) => rows,
            other => {
                return Err(ConvertError::AttributeType {
                    entity_id,
                    field_name,
                    expected: "List (plane of 3D grid)",
                    actual: AttributeKindTag::from_attribute(other),
                });
            }
        };
        if let Some(expected) = expected_rows {
            if rows.len() != expected {
                return Err(ConvertError::DimensionMismatch {
                    entity_id,
                    field_name,
                    expected,
                    actual: rows.len(),
                });
            }
        } else {
            expected_rows = Some(rows.len());
        }
        let mut grid = Vec::with_capacity(rows.len());
        let mut expected_cols = None;
        for row in rows {
            let cols = match row {
                Attribute::List(cols) => cols,
                other => {
                    return Err(ConvertError::AttributeType {
                        entity_id,
                        field_name,
                        expected: "List (row of 3D grid)",
                        actual: AttributeKindTag::from_attribute(other),
                    });
                }
            };
            if let Some(expected) = expected_cols {
                if cols.len() != expected {
                    return Err(ConvertError::DimensionMismatch {
                        entity_id,
                        field_name,
                        expected,
                        actual: cols.len(),
                    });
                }
            } else {
                expected_cols = Some(cols.len());
            }
            let mut row_ids = Vec::with_capacity(cols.len());
            for col in cols {
                match col {
                    Attribute::EntityRef(id) => row_ids.push(*id),
                    other => {
                        return Err(ConvertError::AttributeType {
                            entity_id,
                            field_name,
                            expected: "EntityRef (inside 3D grid)",
                            actual: AttributeKindTag::from_attribute(other),
                        });
                    }
                }
            }
            grid.push(row_ids);
        }
        out.push(grid);
    }
    Ok(out)
}

/// Extract a 3D real grid (`LIST OF LIST OF LIST OF real`) from `attrs[index]` —
/// the [`read_real_grid`] analogue with one extra nesting level (B-spline volume
/// weights). Integers promote to `f64`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeIndex`], [`ConvertError::AttributeType`], or
/// [`ConvertError::DimensionMismatch`].
#[allow(clippy::cast_precision_loss)]
pub fn read_real_grid3(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Vec<Vec<Vec<f64>>>, ConvertError> {
    let attr = get_attr(attrs, index, entity_id, field_name)?;
    let planes = match attr {
        Attribute::List(planes) => planes,
        other => {
            return Err(ConvertError::AttributeType {
                entity_id,
                field_name,
                expected: "List (3D grid)",
                actual: AttributeKindTag::from_attribute(other),
            });
        }
    };
    let mut out = Vec::with_capacity(planes.len());
    let mut expected_rows = None;
    for plane in planes {
        let rows = match plane {
            Attribute::List(rows) => rows,
            other => {
                return Err(ConvertError::AttributeType {
                    entity_id,
                    field_name,
                    expected: "List (plane of 3D grid)",
                    actual: AttributeKindTag::from_attribute(other),
                });
            }
        };
        if let Some(expected) = expected_rows {
            if rows.len() != expected {
                return Err(ConvertError::DimensionMismatch {
                    entity_id,
                    field_name,
                    expected,
                    actual: rows.len(),
                });
            }
        } else {
            expected_rows = Some(rows.len());
        }
        let mut grid = Vec::with_capacity(rows.len());
        let mut expected_cols = None;
        for row in rows {
            let cols = match row {
                Attribute::List(cols) => cols,
                other => {
                    return Err(ConvertError::AttributeType {
                        entity_id,
                        field_name,
                        expected: "List (row of 3D grid)",
                        actual: AttributeKindTag::from_attribute(other),
                    });
                }
            };
            if let Some(expected) = expected_cols {
                if cols.len() != expected {
                    return Err(ConvertError::DimensionMismatch {
                        entity_id,
                        field_name,
                        expected,
                        actual: cols.len(),
                    });
                }
            } else {
                expected_cols = Some(cols.len());
            }
            let mut row_values = Vec::with_capacity(cols.len());
            for col in cols {
                match col {
                    Attribute::Real(v) => row_values.push(*v),
                    Attribute::Integer(v) => row_values.push(*v as f64),
                    other => {
                        return Err(ConvertError::AttributeType {
                            entity_id,
                            field_name,
                            expected: "Real (inside 3D grid)",
                            actual: AttributeKindTag::from_attribute(other),
                        });
                    }
                }
            }
            grid.push(row_values);
        }
        out.push(grid);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Boolean extractor
// ---------------------------------------------------------------------------

/// Extract a STEP boolean (`.T.` / `.F.`) from `attrs[index]`.
///
/// The parser stores `.T.` as `Attribute::Enum("T")` and `.F.` as
/// `Attribute::Enum("F")`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeType`] if the attribute is not an
/// `Enum` or not a recognised boolean value.
pub fn read_bool(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<bool, ConvertError> {
    let val = read_enum(attrs, index, entity_id, field_name)?;
    match val {
        "T" => Ok(true),
        "F" => Ok(false),
        _ => Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "Enum(.T. or .F.)",
            actual: AttributeKindTag::Enum,
        }),
    }
}

/// Read a STEP `LOGICAL` attribute as `Option<bool>`.
///
/// `.T.` → `Some(true)`, `.F.` → `Some(false)`, `.U.` / `.UNKNOWN.` → `None`.
/// Use for entity fields whose EXPRESS type is `LOGICAL` (as opposed to
/// `BOOLEAN`); reach for [`read_bool`] when the source only allows
/// `.T.` / `.F.`.
///
/// # Errors
///
/// Returns [`ConvertError::AttributeType`] if the attribute is not an
/// `Enum`.
pub fn read_logical(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<crate::ir::geometry::Logical, ConvertError> {
    use crate::ir::geometry::Logical;
    let val = read_enum(attrs, index, entity_id, field_name)?;
    Ok(match val {
        "T" => Logical::True,
        "F" => Logical::False,
        _ => Logical::Unknown,
    })
}

/// Render [`Logical`](crate::ir::geometry::Logical) as a STEP `LOGICAL` enum
/// string for writer `Attribute::Enum`: `True` → `"T"`, `False` → `"F"`,
/// `Unknown` → `"U"`.
#[must_use]
pub fn logical_to_step(value: crate::ir::geometry::Logical) -> &'static str {
    use crate::ir::geometry::Logical;
    match value {
        Logical::True => "T",
        Logical::False => "F",
        Logical::Unknown => "U",
    }
}

// ---------------------------------------------------------------------------
// Predicate
// ---------------------------------------------------------------------------

/// Check whether `attrs[index]` is `Unset` or `Derived`.
///
/// Returns `false` if the index is out of bounds.
#[must_use]
pub fn is_unset_or_derived(attrs: &[Attribute], index: usize) -> bool {
    matches!(
        attrs.get(index),
        Some(Attribute::Unset | Attribute::Derived)
    )
}

/// Normalize TAG-less bare scalar elements in the given SET/LIST attribute
/// `slots` to the standard `Typed { type_name: tag, value }` form, returning the
/// (possibly-rewritten) attrs and the count of rewritten elements.
///
/// Some exporters write a select's defined-type member (e.g. `parameter_value`)
/// as a bare `0.0` instead of `PARAMETER_VALUE(0.0)`. The strict generated
/// `bind` only accepts the tagged form, so the entity handler normalizes the
/// input *before* binding (and surfaces a `NonStandardInput`). When no bare
/// scalar is present the original slice is borrowed (no allocation). See
/// `reader::nonstandard` `### NS-tagless-parameter-value`.
pub(crate) fn normalize_tagless_select<'a>(
    attrs: &'a [Attribute],
    slots: &[usize],
    tag: &str,
) -> (std::borrow::Cow<'a, [Attribute]>, usize) {
    let is_bare = |a: &Attribute| matches!(a, Attribute::Real(_) | Attribute::Integer(_));
    let count: usize = slots
        .iter()
        .filter_map(|&i| match attrs.get(i) {
            Some(Attribute::List(elems)) => Some(elems.iter().filter(|e| is_bare(e)).count()),
            _ => None,
        })
        .sum();
    if count == 0 {
        return (std::borrow::Cow::Borrowed(attrs), 0);
    }
    let mut out = attrs.to_vec();
    for &i in slots {
        if let Some(Attribute::List(elems)) = out.get_mut(i) {
            for e in elems.iter_mut() {
                if is_bare(e) {
                    let value = Box::new(std::mem::replace(e, Attribute::Unset));
                    *e = Attribute::Typed {
                        type_name: tag.to_string(),
                        value,
                    };
                }
            }
        }
    }
    (std::borrow::Cow::Owned(out), count)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- check_count ---

    #[test]
    fn check_count_ok() {
        let attrs = vec![Attribute::Integer(1), Attribute::Integer(2)];
        assert!(check_count(&attrs, 2, 1, "TEST").is_ok());
    }

    #[test]
    fn check_count_mismatch() {
        let attrs = vec![Attribute::Integer(1)];
        let err = check_count(&attrs, 2, 1, "TEST").unwrap_err();
        assert!(matches!(
            err,
            ConvertError::AttributeCount {
                expected: 2,
                actual: 1,
                ..
            }
        ));
    }

    // --- read_real ---

    #[test]
    fn read_real_from_real() {
        let attrs = vec![Attribute::Real(1.5)];
        let v = read_real(&attrs, 0, 1, "x").unwrap();
        assert!((v - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn read_real_from_integer_promotion() {
        let attrs = vec![Attribute::Integer(42)];
        let v = read_real(&attrs, 0, 1, "x").unwrap();
        assert!((v - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn read_real_wrong_type() {
        let attrs = vec![Attribute::String("oops".into())];
        let err = read_real(&attrs, 0, 1, "x").unwrap_err();
        assert!(matches!(
            err,
            ConvertError::AttributeType {
                expected: "Real",
                actual: AttributeKindTag::String,
                ..
            }
        ));
    }

    #[test]
    fn read_real_index_out_of_bounds() {
        let attrs: Vec<Attribute> = vec![];
        let err = read_real(&attrs, 0, 1, "x").unwrap_err();
        assert!(matches!(
            err,
            ConvertError::AttributeIndex {
                index: 0,
                len: 0,
                ..
            }
        ));
    }

    // --- read_binary ---

    #[test]
    fn read_binary_ok() {
        let attrs = vec![Attribute::Binary("0F3A".into())];
        assert_eq!(read_binary(&attrs, 0, 1, "lit_value").unwrap(), "0F3A");
    }

    #[test]
    fn read_binary_wrong_type() {
        let attrs = vec![Attribute::String("0F3A".into())];
        let err = read_binary(&attrs, 0, 1, "lit_value").unwrap_err();
        assert!(matches!(
            err,
            ConvertError::AttributeType {
                expected: "Binary",
                actual: AttributeKindTag::String,
                ..
            }
        ));
    }

    // --- read_string ---

    #[test]
    fn read_string_ok() {
        let attrs = vec![Attribute::String("hello".into())];
        assert_eq!(read_string(&attrs, 0, 1, "name").unwrap(), "hello");
    }

    #[test]
    fn read_string_empty() {
        let attrs = vec![Attribute::String(String::new())];
        assert_eq!(read_string(&attrs, 0, 1, "name").unwrap(), "");
    }

    #[test]
    fn read_string_wrong_type() {
        let attrs = vec![Attribute::Integer(1)];
        let err = read_string(&attrs, 0, 1, "name").unwrap_err();
        assert!(matches!(err, ConvertError::AttributeType { .. }));
    }

    // --- read_string_or_unset ---

    #[test]
    fn read_string_or_unset_string() {
        let attrs = vec![Attribute::String("hello".into())];
        assert_eq!(
            read_string_or_unset(&attrs, 0, 1, "description").unwrap(),
            "hello",
        );
    }

    #[test]
    fn read_string_or_unset_unset_returns_empty() {
        let attrs = vec![Attribute::Unset];
        assert_eq!(
            read_string_or_unset(&attrs, 0, 1, "description").unwrap(),
            "",
        );
    }

    #[test]
    fn read_string_or_unset_wrong_type_errors() {
        let attrs = vec![Attribute::Integer(1)];
        let err = read_string_or_unset(&attrs, 0, 1, "description").unwrap_err();
        assert!(matches!(err, ConvertError::AttributeType { .. }));
    }

    #[test]
    fn read_string_or_unset_out_of_range_errors() {
        let attrs: Vec<Attribute> = vec![];
        let err = read_string_or_unset(&attrs, 0, 1, "description").unwrap_err();
        assert!(matches!(err, ConvertError::AttributeIndex { .. }));
    }

    // --- read_entity_ref ---

    #[test]
    fn read_entity_ref_ok() {
        let attrs = vec![Attribute::EntityRef(42)];
        assert_eq!(read_entity_ref(&attrs, 0, 1, "ref").unwrap(), 42);
    }

    #[test]
    fn read_entity_ref_wrong_type() {
        let attrs = vec![Attribute::Real(1.0)];
        let err = read_entity_ref(&attrs, 0, 1, "ref").unwrap_err();
        assert!(matches!(
            err,
            ConvertError::AttributeType {
                actual: AttributeKindTag::Real,
                ..
            }
        ));
    }

    // --- read_optional_entity_ref ---

    #[test]
    fn read_optional_entity_ref_present() {
        let attrs = vec![Attribute::EntityRef(10)];
        assert_eq!(
            read_optional_entity_ref(&attrs, 0, 1, "axis").unwrap(),
            Some(10)
        );
    }

    #[test]
    fn read_optional_entity_ref_unset() {
        let attrs = vec![Attribute::Unset];
        assert_eq!(
            read_optional_entity_ref(&attrs, 0, 1, "axis").unwrap(),
            None
        );
    }

    #[test]
    fn read_optional_entity_ref_derived() {
        let attrs = vec![Attribute::Derived];
        assert_eq!(
            read_optional_entity_ref(&attrs, 0, 1, "axis").unwrap(),
            None
        );
    }

    #[test]
    fn read_optional_entity_ref_wrong_type() {
        let attrs = vec![Attribute::Real(1.0)];
        let err = read_optional_entity_ref(&attrs, 0, 1, "axis").unwrap_err();
        assert!(matches!(err, ConvertError::AttributeType { .. }));
    }

    // --- read_enum ---

    #[test]
    fn read_enum_ok() {
        let attrs = vec![Attribute::Enum("MILLI".into())];
        assert_eq!(read_enum(&attrs, 0, 1, "prefix").unwrap(), "MILLI");
    }

    #[test]
    fn read_enum_wrong_type() {
        let attrs = vec![Attribute::Integer(1)];
        let err = read_enum(&attrs, 0, 1, "prefix").unwrap_err();
        assert!(matches!(err, ConvertError::AttributeType { .. }));
    }

    // --- read_real_list ---

    #[test]
    fn read_real_list_ok() {
        let attrs = vec![Attribute::List(vec![
            Attribute::Real(1.0),
            Attribute::Real(2.0),
            Attribute::Real(3.0),
        ])];
        assert_eq!(
            read_real_list(&attrs, 0, 1, "coords").unwrap(),
            vec![1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn read_real_list_with_integer_promotion() {
        let attrs = vec![Attribute::List(vec![
            Attribute::Real(1.0),
            Attribute::Integer(0),
            Attribute::Real(3.0),
        ])];
        assert_eq!(
            read_real_list(&attrs, 0, 1, "coords").unwrap(),
            vec![1.0, 0.0, 3.0]
        );
    }

    #[test]
    fn read_real_list_empty() {
        let attrs = vec![Attribute::List(vec![])];
        assert_eq!(read_real_list(&attrs, 0, 1, "coords").unwrap(), vec![]);
    }

    #[test]
    fn read_real_list_not_a_list() {
        let attrs = vec![Attribute::Real(1.0)];
        let err = read_real_list(&attrs, 0, 1, "coords").unwrap_err();
        assert!(matches!(
            err,
            ConvertError::AttributeType {
                expected: "List",
                actual: AttributeKindTag::Real,
                ..
            }
        ));
    }

    #[test]
    fn read_real_list_non_numeric_item() {
        let attrs = vec![Attribute::List(vec![
            Attribute::Real(1.0),
            Attribute::String("bad".into()),
        ])];
        let err = read_real_list(&attrs, 0, 1, "coords").unwrap_err();
        assert!(matches!(
            err,
            ConvertError::AttributeType {
                actual: AttributeKindTag::String,
                ..
            }
        ));
    }

    // --- read_entity_ref_list ---

    #[test]
    fn read_entity_ref_list_ok() {
        let attrs = vec![Attribute::List(vec![
            Attribute::EntityRef(1),
            Attribute::EntityRef(2),
        ])];
        assert_eq!(
            read_entity_ref_list(&attrs, 0, 1, "refs").unwrap(),
            vec![1, 2]
        );
    }

    #[test]
    fn read_entity_ref_list_non_ref_item() {
        let attrs = vec![Attribute::List(vec![
            Attribute::EntityRef(1),
            Attribute::Integer(2),
        ])];
        let err = read_entity_ref_list(&attrs, 0, 1, "refs").unwrap_err();
        assert!(matches!(err, ConvertError::AttributeType { .. }));
    }

    // --- read_real_grid ---

    #[test]
    fn read_real_grid_ok() {
        let attrs = vec![Attribute::List(vec![
            Attribute::List(vec![Attribute::Real(1.0), Attribute::Real(2.0)]),
            Attribute::List(vec![Attribute::Real(3.0), Attribute::Real(4.0)]),
        ])];
        let grid = read_real_grid(&attrs, 0, 1, "weights").unwrap();
        assert_eq!(grid.len(), 2);
        assert_eq!(grid[0].len(), 2);
        assert!((grid[1][1] - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn read_real_grid_non_rectangular() {
        let attrs = vec![Attribute::List(vec![
            Attribute::List(vec![Attribute::Real(1.0), Attribute::Real(2.0)]),
            Attribute::List(vec![Attribute::Real(3.0)]),
        ])];
        let err = read_real_grid(&attrs, 0, 1, "weights").unwrap_err();
        assert!(matches!(err, ConvertError::DimensionMismatch { .. }));
    }

    #[test]
    fn read_real_grid_integer_promotion() {
        let attrs = vec![Attribute::List(vec![Attribute::List(vec![
            Attribute::Integer(1),
            Attribute::Real(2.5),
        ])])];
        let grid = read_real_grid(&attrs, 0, 1, "weights").unwrap();
        assert!((grid[0][0] - 1.0).abs() < f64::EPSILON);
        assert!((grid[0][1] - 2.5).abs() < f64::EPSILON);
    }

    // --- read_*_grid3 (3D, B-spline volume) ---

    #[test]
    fn read_entity_ref_grid3_ok() {
        // 2 planes x 1 row x 2 cols.
        let plane = || {
            Attribute::List(vec![Attribute::List(vec![
                Attribute::EntityRef(10),
                Attribute::EntityRef(11),
            ])])
        };
        let attrs = vec![Attribute::List(vec![plane(), plane()])];
        let g = read_entity_ref_grid3(&attrs, 0, 1, "control_points_list").unwrap();
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].len(), 1);
        assert_eq!(g[1][0], vec![10, 11]);
    }

    #[test]
    fn read_real_grid3_ok_with_promotion() {
        let attrs = vec![Attribute::List(vec![Attribute::List(vec![
            Attribute::List(vec![Attribute::Integer(1), Attribute::Real(2.5)]),
        ])])];
        let g = read_real_grid3(&attrs, 0, 1, "weights_data").unwrap();
        assert!((g[0][0][0] - 1.0).abs() < f64::EPSILON);
        assert!((g[0][0][1] - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn read_entity_ref_grid3_wrong_type() {
        // innermost element is not an EntityRef.
        let attrs = vec![Attribute::List(vec![Attribute::List(vec![
            Attribute::List(vec![Attribute::Real(1.0)]),
        ])])];
        let err = read_entity_ref_grid3(&attrs, 0, 1, "control_points_list").unwrap_err();
        assert!(matches!(err, ConvertError::AttributeType { .. }));
    }

    #[test]
    fn read_real_grid3_non_rectangular() {
        // two planes with differing row counts.
        let attrs = vec![Attribute::List(vec![
            Attribute::List(vec![Attribute::List(vec![Attribute::Real(1.0)])]),
            Attribute::List(vec![
                Attribute::List(vec![Attribute::Real(2.0)]),
                Attribute::List(vec![Attribute::Real(3.0)]),
            ]),
        ])];
        let err = read_real_grid3(&attrs, 0, 1, "weights_data").unwrap_err();
        assert!(matches!(err, ConvertError::DimensionMismatch { .. }));
    }

    // --- is_unset_or_derived ---

    #[test]
    fn is_unset_or_derived_true() {
        let attrs = vec![Attribute::Unset, Attribute::Derived, Attribute::Real(1.0)];
        assert!(is_unset_or_derived(&attrs, 0));
        assert!(is_unset_or_derived(&attrs, 1));
    }

    #[test]
    fn is_unset_or_derived_false() {
        let attrs = vec![Attribute::Real(1.0)];
        assert!(!is_unset_or_derived(&attrs, 0));
    }

    #[test]
    fn is_unset_or_derived_out_of_bounds() {
        let attrs: Vec<Attribute> = vec![];
        assert!(!is_unset_or_derived(&attrs, 0));
    }
}
