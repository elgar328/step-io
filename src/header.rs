//! The Part 21 HEADER section — [`FileHeader`], shared by reading and writing.
//!
//! A STEP file's HEADER holds three records: `FILE_DESCRIPTION`, `FILE_NAME`,
//! and `FILE_SCHEMA`. [`FileHeader`] models all three; it lives on the
//! [`StepModel`] so a read surfaces the source header via
//! [`StepModel::header`] and a write emits the model's own header back out.

use crate::generated::model::StepModel;
use crate::parser::{Attribute, RawEntity, SchemaId};

/// The Part 21 HEADER section (`FILE_DESCRIPTION` + `FILE_NAME` +
/// `FILE_SCHEMA`). Reading fills it from the source file; the builder fills
/// it from user input plus its automatic timestamp and preprocessor stamp.
/// The default (all empty) matches what step-io has always emitted.
#[derive(Debug, Clone, Default)]
pub struct FileHeader {
    /// `FILE_DESCRIPTION.description` (single entry).
    pub description: String,
    /// `FILE_NAME.name`.
    pub file_name: String,
    /// `FILE_NAME.time_stamp`.
    pub time_stamp: String,
    /// `FILE_NAME.author`; empty renders as the customary `('')`.
    pub authors: Vec<String>,
    /// `FILE_NAME.organization`; empty renders as the customary `('')`.
    pub organizations: Vec<String>,
    /// `FILE_NAME.preprocessor_version`.
    pub preprocessor_version: String,
    /// `FILE_NAME.originating_system`.
    pub originating_system: String,
    /// `FILE_NAME.authorisation`.
    pub authorisation: String,
    /// `FILE_SCHEMA` — the identified schema (AP family, edition, stage) plus
    /// the raw `FILE_SCHEMA` strings. On a read model this records the
    /// *source* schema.
    pub schema: SchemaId,
}

impl StepModel {
    /// The file's HEADER section. On a model returned by
    /// [`read`](crate::read) this is the source file's header (including the
    /// identified [`schema`](FileHeader::schema)).
    #[must_use]
    pub fn header(&self) -> &FileHeader {
        &self.header
    }
}

/// Decode the raw HEADER entities into a [`FileHeader`]. Positional per Part
/// 21: `FILE_NAME(name, time_stamp, author, organization,
/// preprocessor_version, originating_system, authorisation)` and
/// `FILE_DESCRIPTION(description, implementation_level)`. Lenient on
/// non-standard input — a missing or off-kind field reads as empty.
pub(crate) fn from_raw(raw: &[RawEntity], schema: SchemaId) -> FileHeader {
    let mut h = FileHeader {
        schema,
        ..FileHeader::default()
    };
    for ent in raw {
        let RawEntity::Simple {
            name, attributes, ..
        } = ent
        else {
            continue;
        };
        let at = |i: usize| attributes.get(i);
        match name.as_str() {
            "FILE_NAME" => {
                h.file_name = attr_str(at(0));
                h.time_stamp = attr_str(at(1));
                h.authors = attr_list(at(2));
                h.organizations = attr_list(at(3));
                h.preprocessor_version = attr_str(at(4));
                h.originating_system = attr_str(at(5));
                h.authorisation = attr_str(at(6));
            }
            "FILE_DESCRIPTION" => {
                // The description list customarily holds one entry; join any
                // extras rather than dropping them.
                h.description = attr_list(at(0)).join(", ");
            }
            _ => {}
        }
    }
    h
}

/// A HEADER string attribute; anything else reads as empty.
fn attr_str(a: Option<&Attribute>) -> String {
    match a {
        Some(Attribute::String(s)) => s.clone(),
        _ => String::new(),
    }
}

/// A HEADER string-list attribute; empty entries are dropped (the customary
/// empty list renders as `('')`).
fn attr_list(a: Option<&Attribute>) -> Vec<String> {
    match a {
        Some(Attribute::List(items)) => items
            .iter()
            .filter_map(|it| match it {
                Attribute::String(s) if !s.is_empty() => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}
