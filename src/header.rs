//! [`FileHeader`] — the Part 21 HEADER section, shared by reading and writing.
//!
//! Every STEP file opens with three HEADER records. `FILE_DESCRIPTION` says
//! what the file contains; `FILE_NAME` records where it came from — file
//! name, timestamp, authors, organizations, the STEP processor, and the
//! originating CAD system; `FILE_SCHEMA` names the schema. [`FileHeader`]
//! models all three and lives on the [`StepModel`]: reading surfaces the
//! source file's header as [`StepModel::header`], and authoring fills it
//! through [`StepBuilder::header`](crate::StepBuilder::header) for the
//! written file.

use crate::generated::model::StepModel;
use crate::parser::{Attribute, RawEntity, SchemaId};

/// The Part 21 HEADER section (`FILE_DESCRIPTION` + `FILE_NAME` +
/// `FILE_SCHEMA`). Reading decodes it from the source file; the builder
/// fills it from user input plus its automatic timestamp and preprocessor
/// stamp. The default is the customary all-empty header.
#[derive(Debug, Clone, Default)]
pub struct FileHeader {
    /// `FILE_DESCRIPTION.description`; multiple entries read as one joined
    /// string.
    pub description: String,
    /// `FILE_NAME.name`.
    pub file_name: String,
    /// `FILE_NAME.time_stamp`.
    pub time_stamp: String,
    /// `FILE_NAME.author`; empty renders as the customary `('')`.
    pub authors: Vec<String>,
    /// `FILE_NAME.organization`; empty renders as the customary `('')`.
    pub organizations: Vec<String>,
    /// `FILE_NAME.preprocessor_version` — the STEP processor or library
    /// that produced the file.
    pub preprocessor_version: String,
    /// `FILE_NAME.originating_system` — the CAD application the file came
    /// from.
    pub originating_system: String,
    /// `FILE_NAME.authorisation`.
    pub authorisation: String,
    /// `FILE_SCHEMA` — the identified schema (AP family, edition, stage)
    /// plus the raw strings. The source schema on a read model; the AP242
    /// identity on an authored one.
    pub schema: SchemaId,
}

impl StepModel {
    /// The file's HEADER section. On a model returned by
    /// [`read`](crate::read) this is the source file's header, the
    /// identified [`schema`](FileHeader::schema) included.
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
