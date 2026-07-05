//! [`write()`] — the Part 21 envelope, the last step of writing.
//!
//! [`write()`] serializes a [`StepModel`] to Part 21 text. The `DATA`
//! section comes from the generated writer; the `HEADER` section,
//! `FILE_SCHEMA` included, comes from the model's own [`FileHeader`]
//! ([`StepModel::header`]). Conformance is the authoring layer's job:
//! [`crate::Ap242Author`] admits only AP242 entities and stamps the AP242
//! schema identity on the header, so an authored model writes out as a
//! valid AP242 file.

use crate::generated::model::StepModel;
use crate::generated::write::Writer;
use crate::header::FileHeader;

/// APD/`application_context` field values stamped by [`dump_universal`] so
/// the dumped file's APD entities agree with its `FILE_SCHEMA` marker.
struct ApdInfo {
    status: &'static str,
    name: &'static str,
    year: i64,
    description: &'static str,
}

/// Universal dump marker — step-io's all-AP union is not a real Application
/// Protocol, so the `FILE_SCHEMA` header and APD declare a non-standard
/// format. This keeps the dump internally consistent and prevents it
/// masquerading as a real AP.
const UNIVERSAL_FILE_SCHEMA: &str = "STEPIO_UNIVERSAL";
static UNIVERSAL_APD: ApdInfo = ApdInfo {
    status: "not a standard",
    name: "stepio_universal",
    year: 0,
    description: "step-io universal union (non-standard, all-AP superset)",
};

/// Serialize the model to Part 21 text — the model's own header
/// ([`StepModel::header`], `FILE_SCHEMA` included) plus every entity,
/// verbatim.
///
/// An authored model ([`crate::StepBuilder`] / [`crate::Ap242Author`]) is
/// AP242 by construction and carries the AP242 schema identity, so the
/// output is a conforming AP242 file. A model that came from
/// [`read`](crate::read) re-serializes faithfully under its *source* schema
/// — step-io does not convert between APs.
#[must_use]
pub fn write(model: &StepModel) -> String {
    wrap_envelope(&Writer::new(model).emit_all(), &model.header)
}

/// Dump the model as step-io's **Universal** union — every entity, verbatim,
/// under the non-standard `STEPIO_UNIVERSAL` marker (`FILE_SCHEMA` and
/// APD/`application_context` are set to it, keeping the dump internally
/// consistent). Not a STEP output — the round-trip oracle for the external
/// verification harness; the write path is [`write()`].
#[doc(hidden)]
#[must_use]
pub fn dump_universal(model: &mut StepModel) -> String {
    stamp_apd(model, &UNIVERSAL_APD);
    model.header.schema =
        crate::parser::schema::identify_schema(&[UNIVERSAL_FILE_SCHEMA.to_string()]);
    write(model)
}

/// Absolutely set the model's APD entities (status/name/year) and
/// `application_context` entities (application = `apd.description`) to `apd`,
/// so the dumped file's APD agrees with the `FILE_SCHEMA` header.
fn stamp_apd(model: &mut StepModel, apd: &ApdInfo) {
    for x in &mut model.application_protocol_definition_arena.items {
        x.status = apd.status.to_string();
        x.application_interpreted_model_schema_name = apd.name.to_string();
        x.application_protocol_year = apd.year;
    }
    for ac in &mut model.application_context_arena.items {
        ac.application = apd.description.to_string();
    }
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

/// Wrap a DATA body in the Part 21 envelope built from `header` —
/// `FILE_SCHEMA` from [`FileHeader::schema`]'s raw strings (an absent raw
/// renders as the customary empty `('')`).
fn wrap_envelope(data_body: &str, header: &FileHeader) -> String {
    let schema = header.schema.raw().map_or_else(
        || "''".to_owned(),
        |r| {
            r.iter()
                .map(|s| header_str(s))
                .collect::<Vec<_>>()
                .join(",")
        },
    );
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
