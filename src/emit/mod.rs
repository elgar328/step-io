//! The Part 21 envelope — the last step of writing.
//!
//! The `DATA` section — the entities themselves — comes from the generated
//! writer ([`generated::write`](crate::generated::write)); this module wraps
//! it in the Part 21 envelope, the fixed `ISO-10303-21;` …
//! `END-ISO-10303-21;` frame around a `HEADER` section. [`FileHeader`]
//! models the header's `FILE_DESCRIPTION` and `FILE_NAME` records (file
//! name, authors, timestamp, …); the third record, `FILE_SCHEMA`, comes
//! from the output target.

mod projection;
pub use projection::LossReport;

use crate::generated::model::StepModel;
use crate::generated::profile::{ApdInfo, SchemaProfile, SchemaTarget};
use crate::generated::write::Writer;

/// Universal output marker — step-io's all-AP union is not a real Application
/// Protocol, so the `FILE_SCHEMA` header and APD declare a non-standard format. This
/// keeps the output internally consistent and prevents it masquerading as a real AP.
/// `UNIVERSAL_APD` mirrors a real target's `SchemaProfile::apd`; `description` is the
/// `application_context` text (the `legal`/`downgrade` sets have no Universal analog —
/// Universal skips projection).
const UNIVERSAL_FILE_SCHEMA: &[&str] = &["STEPIO_UNIVERSAL"];
static UNIVERSAL_APD: ApdInfo = ApdInfo {
    status: "not a standard",
    name: "stepio_universal",
    year: 0,
    description: "step-io universal union (non-standard, all-AP superset)",
};

/// Emit the model as the **Universal** target — the full read model, no projection
/// (the union schema is a superset, so nothing is illegal). The `FILE_SCHEMA` header
/// and APD/`application_context` entities are set to the non-standard universal
/// marker so the output is internally consistent (header ↔ APD agree).
#[doc(hidden)]
#[must_use]
pub fn write_universal(model: &mut StepModel) -> String {
    write_target(model, SchemaTarget::Universal).0
}

/// Emit the model to a specific output target. The output is always internally
/// consistent: the `FILE_SCHEMA` header AND the APD/`application_context` entities are
/// absolutely set to the target's values (no restore needed — a later write simply
/// overwrites again). For real AP targets the model is projected onto the target's
/// legal entity set: rename-safe subtypes are downgraded, the rest dropped
/// (referential-closure cascade), stranded orphans pruned — all recorded in the
/// returned [`LossReport`]. `Universal` skips projection (union superset).
///
/// The original input schema is preserved in `Report.schema` (read time) and is not
/// affected by writes.
///
/// There is no ground-truth to compare a projected output against; correctness
/// is "valid + fully accounted" — the output re-reads with zero drops, and
/// `input == kept + report.dropped + report.downgraded`.
/// Internal round-trip oracle — not a supported entry point; the supported
/// write path is the authoring API ([`crate::StepBuilder`] / [`crate::Ap242Author`]).
#[doc(hidden)]
#[must_use]
pub fn write_target(model: &mut StepModel, target: SchemaTarget) -> (String, LossReport) {
    write_target_with_header(model, target, &FileHeader::default())
}

/// [`write_target`] with explicit Part 21 HEADER fields (the plain form
/// emits the all-empty default header).
#[doc(hidden)]
#[must_use]
pub fn write_target_with_header(
    model: &mut StepModel,
    target: SchemaTarget,
    header: &FileHeader,
) -> (String, LossReport) {
    let profile = SchemaProfile::for_target(target);

    // Absolutely set the header schema + APD/AC entities to the target's values.
    stamp_header(model, profile.map_or(&UNIVERSAL_APD, |p| &p.apd));
    let file_schema = profile.map_or(UNIVERSAL_FILE_SCHEMA, |p| p.file_schema);

    // Project (real AP) or pass through (Universal), then wrap.
    let (body, loss) = match profile {
        Some(profile) => {
            let w = Writer::new(&*model);
            let plan = projection::plan(&w, profile);
            (w.emit_all_with_plan(&plan.dropped, plan.rename), plan.loss)
        }
        None => (Writer::new(&*model).emit_all(), LossReport::default()),
    };
    (wrap_envelope(&body, file_schema, header), loss)
}

/// Absolutely set the model's APD entities (status/name/year) and
/// `application_context` entities (application = `apd.description`) to `apd`, so the
/// emitted file's APD agrees with the `FILE_SCHEMA` header.
fn stamp_header(model: &mut StepModel, apd: &ApdInfo) {
    for x in &mut model.application_protocol_definition_arena.items {
        x.status = apd.status.to_string();
        x.application_interpreted_model_schema_name = apd.name.to_string();
        x.application_protocol_year = apd.year;
    }
    for ac in &mut model.application_context_arena.items {
        ac.application = apd.description.to_string();
    }
}

/// The Part 21 HEADER section fields (`FILE_DESCRIPTION` + `FILE_NAME`).
/// The default (all empty) matches what step-io has always emitted; the
/// builder fills it from user input plus its automatic timestamp and
/// preprocessor stamp.
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
}

/// Part 21 string literal for the HEADER section — same convention as the
/// generated DATA writer's `step_str` (inner `'` doubled, non-ASCII passes
/// through raw).
fn header_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push('\'');
        }
        out.push(ch);
    }
    out.push('\'');
    out
}

/// A HEADER string list; an empty list renders as the customary `('')`.
fn header_list(items: &[String]) -> String {
    if items.is_empty() {
        return "('')".to_owned();
    }
    let inner = items
        .iter()
        .map(|s| header_str(s))
        .collect::<Vec<_>>()
        .join(",");
    format!("({inner})")
}

/// Wrap a DATA body in the Part 21 envelope with the given `FILE_SCHEMA`
/// and HEADER fields.
fn wrap_envelope(data_body: &str, file_schema: &[&str], header: &FileHeader) -> String {
    let schema = file_schema
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(({desc}),'2;1');\n\
         FILE_NAME({name},{stamp},{authors},{orgs},{pre},{orig},{auth});\n\
         FILE_SCHEMA(({schema}));\nENDSEC;\nDATA;\n{data_body}ENDSEC;\nEND-ISO-10303-21;\n",
        desc = header_str(&header.description),
        name = header_str(&header.file_name),
        stamp = header_str(&header.time_stamp),
        authors = header_list(&header.authors),
        orgs = header_list(&header.organizations),
        pre = header_str(&header.preprocessor_version),
        orig = header_str(&header.originating_system),
        auth = header_str(&header.authorisation),
    )
}
