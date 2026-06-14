use crate::parser::entity::Attribute;

/// Errors encountered when converting the raw `EntityGraph` into typed IR.
///
/// Distinct from [`crate::ParseError`] which covers Part 21 syntax.
/// `ConvertError` covers semantic issues: wrong attribute types, missing
/// references, unsupported entities, etc.
#[derive(Debug, Clone, PartialEq)]
pub enum ConvertError {
    /// Attribute count mismatch.
    AttributeCount {
        entity_id: u64,
        entity_name: String,
        expected: usize,
        actual: usize,
    },
    /// An attribute had an unexpected variant.
    AttributeType {
        entity_id: u64,
        field_name: &'static str,
        expected: &'static str,
        actual: AttributeKindTag,
    },
    /// Attribute index out of bounds.
    AttributeIndex {
        entity_id: u64,
        field_name: &'static str,
        index: usize,
        len: usize,
    },
    /// A referenced entity `#to` was not found in the graph or conversion maps.
    MissingReference {
        from: u64,
        to: u64,
        field_name: &'static str,
    },
    /// A referenced entity had an unexpected type name.
    WrongEntityType {
        entity_id: u64,
        field_name: &'static str,
        expected: &'static str,
        actual: String,
    },
    /// The entity is Complex where Simple was expected (or vice versa).
    UnexpectedEntityForm { entity_id: u64, detail: String },
    /// An entity type that the reader does not handle.
    UnsupportedEntity { entity_id: u64, name: String },
    /// A complex (multi-part AND) instance whose exact part-set matches no
    /// complex handler's declared cases — dropped. A distinct variant (not
    /// `UnexpectedEntityForm`) so it can be told apart from genuine defects:
    /// it means "a complex shape we have not modelled appeared — investigate".
    UnhandledComplex { entity_id: u64, parts: Vec<String> },
    /// Coordinate list has wrong dimensionality.
    DimensionMismatch {
        entity_id: u64,
        field_name: &'static str,
        expected: usize,
        actual: usize,
    },
    /// The input file violated the ISO 10303 schema — a required field was
    /// Unset (or carried an unrecognized value) on `count` entities; the
    /// reader normalized each to a standard default. Aggregated per file.
    /// This is an INPUT defect, not a step-io defect.
    ///
    /// CONTRACT: emit this variant *only* when the source file is
    /// non-standard and the reader recovered it by normalizing to a standard
    /// default — never for a step-io-side defect or an unmodelled entity.
    /// It marks a category that round-trip analysis must not count as data
    /// loss (the entity is preserved, just normalized).
    NonStandardInput {
        field: String,
        count: usize,
        normalized_to: String,
    },
    /// A strict ENUM bind received a token outside the EXPRESS enumeration.
    /// Distinct from `UnexpectedEntityForm` so the dispatcher can reclassify it
    /// as a `NonStandardInput` drop (NORM) rather than a step-io defect (LOSS) —
    /// rejecting a non-standard value is correct behaviour, not a coverage gap.
    /// Never reaches `warnings` directly: the dispatcher always reclassifies it.
    NonStandardEnumValue {
        entity_id: u64,
        field: String,
        token: String,
    },
}

/// Lightweight tag identifying the `Attribute` variant without carrying its
/// payload. Used in error messages to avoid cloning large `Attribute::List`
/// values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeKindTag {
    Integer,
    Real,
    String,
    Enum,
    Binary,
    EntityRef,
    Unset,
    Derived,
    List,
    Typed,
}

impl AttributeKindTag {
    #[must_use]
    pub fn from_attribute(attr: &Attribute) -> Self {
        match attr {
            Attribute::Integer(_) => Self::Integer,
            Attribute::Real(_) => Self::Real,
            Attribute::String(_) => Self::String,
            Attribute::Enum(_) => Self::Enum,
            Attribute::Binary(_) => Self::Binary,
            Attribute::EntityRef(_) => Self::EntityRef,
            Attribute::Unset => Self::Unset,
            Attribute::Derived => Self::Derived,
            Attribute::List(_) => Self::List,
            Attribute::Typed { .. } => Self::Typed,
        }
    }
}

impl std::fmt::Display for AttributeKindTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer => write!(f, "Integer"),
            Self::Real => write!(f, "Real"),
            Self::String => write!(f, "String"),
            Self::Enum => write!(f, "Enum"),
            Self::Binary => write!(f, "Binary"),
            Self::EntityRef => write!(f, "EntityRef"),
            Self::Unset => write!(f, "Unset"),
            Self::Derived => write!(f, "Derived"),
            Self::List => write!(f, "List"),
            Self::Typed => write!(f, "Typed"),
        }
    }
}

impl std::fmt::Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AttributeCount {
                entity_id,
                entity_name,
                expected,
                actual,
            } => write!(
                f,
                "entity #{entity_id} ({entity_name}): \
                 expected {expected} attributes, got {actual}"
            ),
            Self::AttributeType {
                entity_id,
                field_name,
                expected,
                actual,
            } => write!(
                f,
                "entity #{entity_id}: field '{field_name}' \
                 expected {expected}, found {actual}"
            ),
            Self::AttributeIndex {
                entity_id,
                field_name,
                index,
                len,
            } => write!(
                f,
                "entity #{entity_id}: field '{field_name}' \
                 index {index} out of bounds (len {len})"
            ),
            Self::MissingReference {
                from,
                to,
                field_name,
            } => write!(
                f,
                "entity #{from}: field '{field_name}' \
                 references #{to} which was not found"
            ),
            Self::WrongEntityType {
                entity_id,
                field_name,
                expected,
                actual,
            } => write!(
                f,
                "entity #{entity_id}: field '{field_name}' \
                 expected {expected}, found {actual}"
            ),
            Self::UnexpectedEntityForm { entity_id, detail } => {
                write!(f, "entity #{entity_id}: {detail}")
            }
            Self::UnsupportedEntity { entity_id, name } => {
                write!(f, "entity #{entity_id}: unsupported type {name}")
            }
            Self::UnhandledComplex { entity_id, parts } => {
                write!(
                    f,
                    "entity #{entity_id}: complex matches no handler case — parts ({})",
                    parts.join(" ")
                )
            }
            Self::DimensionMismatch {
                entity_id,
                field_name,
                expected,
                actual,
            } => write!(
                f,
                "entity #{entity_id}: field '{field_name}' \
                 expected {expected}D, got {actual}D"
            ),
            Self::NonStandardInput {
                field,
                count,
                normalized_to,
            } => write!(
                f,
                "non-standard input: {count}× {field} violates ISO 10303 \
                 (required field); reader normalized to {normalized_to}. \
                 The source file is non-standard; this is not a step-io defect."
            ),
            Self::NonStandardEnumValue {
                entity_id,
                field,
                token,
            } => write!(
                f,
                "entity #{entity_id}: field '{field}' has non-standard enum \
                 value '.{token}.' (outside the EXPRESS enumeration)"
            ),
        }
    }
}

impl std::error::Error for ConvertError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_kind_tag_from_all_variants() {
        assert_eq!(
            AttributeKindTag::from_attribute(&Attribute::Integer(1)),
            AttributeKindTag::Integer,
        );
        assert_eq!(
            AttributeKindTag::from_attribute(&Attribute::Real(1.0)),
            AttributeKindTag::Real,
        );
        assert_eq!(
            AttributeKindTag::from_attribute(&Attribute::String("x".into())),
            AttributeKindTag::String,
        );
        assert_eq!(
            AttributeKindTag::from_attribute(&Attribute::Enum("T".into())),
            AttributeKindTag::Enum,
        );
        assert_eq!(
            AttributeKindTag::from_attribute(&Attribute::Binary("FF".into())),
            AttributeKindTag::Binary,
        );
        assert_eq!(
            AttributeKindTag::from_attribute(&Attribute::EntityRef(1)),
            AttributeKindTag::EntityRef,
        );
        assert_eq!(
            AttributeKindTag::from_attribute(&Attribute::Unset),
            AttributeKindTag::Unset,
        );
        assert_eq!(
            AttributeKindTag::from_attribute(&Attribute::Derived),
            AttributeKindTag::Derived,
        );
        assert_eq!(
            AttributeKindTag::from_attribute(&Attribute::List(vec![])),
            AttributeKindTag::List,
        );
        assert_eq!(
            AttributeKindTag::from_attribute(&Attribute::Typed {
                type_name: "X".into(),
                value: Box::new(Attribute::Integer(1)),
            }),
            AttributeKindTag::Typed,
        );
    }

    #[test]
    fn attribute_kind_tag_display() {
        assert_eq!(AttributeKindTag::Real.to_string(), "Real");
        assert_eq!(AttributeKindTag::EntityRef.to_string(), "EntityRef");
        assert_eq!(AttributeKindTag::List.to_string(), "List");
    }

    #[test]
    fn convert_error_display_attribute_type() {
        let err = ConvertError::AttributeType {
            entity_id: 53,
            field_name: "axis",
            expected: "EntityRef",
            actual: AttributeKindTag::Real,
        };
        assert_eq!(
            err.to_string(),
            "entity #53: field 'axis' expected EntityRef, found Real"
        );
    }

    #[test]
    fn convert_error_display_missing_reference() {
        let err = ConvertError::MissingReference {
            from: 10,
            to: 99,
            field_name: "location",
        };
        assert_eq!(
            err.to_string(),
            "entity #10: field 'location' references #99 which was not found"
        );
    }

    #[test]
    fn convert_error_implements_std_error() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        let err = ConvertError::UnsupportedEntity {
            entity_id: 1,
            name: "UNKNOWN".into(),
        };
        assert_error(&err);
    }
}
