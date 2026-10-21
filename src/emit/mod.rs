//! [`write()`] — the Part 21 envelope, the last step of writing.
//!
//! [`write()`] serializes a [`StepModel`] to Part 21 text on an
//! [`io::Write`]. The `DATA` section comes from the generated writer; the
//! `HEADER` section, `FILE_SCHEMA` included, comes from the model's own
//! [`FileHeader`] ([`StepModel::header`]). Conformance is the authoring
//! layer's job: [`crate::Ap242Author`] admits only AP242 entities, checked
//! against the schema as they enter, and stamps the AP242 schema identity on
//! the header, so an authored model writes out as an AP242 file.

use std::io;

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

/// Write the model as Part 21 text to `out` — the model's own header
/// ([`StepModel::header`], `FILE_SCHEMA` included) plus every entity,
/// verbatim.
///
/// An authored model ([`crate::StepBuilder`] / [`crate::Ap242Author`]) is
/// AP242 by construction and carries the AP242 schema identity, so the
/// output is a conforming AP242 file. A model that came from
/// [`read`](crate::read) re-serializes faithfully under its *source* schema
/// — step-io does not convert between APs.
///
/// The text goes to `out` as it is written, in chunks of 64 KiB, and `out`
/// is flushed at the end, so a [`File`](std::fs::File) needs no
/// `BufWriter`. To get the text instead, pass a `&mut Vec<u8>`; the output
/// is ASCII, so it is always UTF-8.
///
/// ```no_run
/// # let model = step_io::StepBuilder::new().finish();
/// step_io::write(&model, std::fs::File::create("part.step")?)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
/// The first error writing to `out`. `out` then holds the file cut short.
pub fn write(model: &StepModel, mut out: impl io::Write) -> io::Result<()> {
    out.write_all(header_text(&model.header).as_bytes())?;
    Writer::new(model).emit_all(&mut out)?;
    out.write_all(b"ENDSEC;\nEND-ISO-10303-21;\n")?;
    out.flush()
}

/// Dump the model as step-io's **Universal** union — every entity, verbatim,
/// under the non-standard `STEPIO_UNIVERSAL` marker (`FILE_SCHEMA` and
/// APD/`application_context` are set to it, keeping the dump internally
/// consistent). Not a STEP output — an internal round-trip oracle; the write
/// path is [`write()`].
///
/// # Errors
/// The first error writing to `out`, as for [`write()`].
#[doc(hidden)]
pub fn dump_universal(model: &mut StepModel, out: impl io::Write) -> io::Result<()> {
    stamp_apd(model, &UNIVERSAL_APD);
    model.header.schema =
        crate::parser::schema::identify_schema(&[UNIVERSAL_FILE_SCHEMA.to_string()]);
    write(model, out)
}

/// Absolutely set the model's APD entities (status/name/year) and
/// `application_context` entities (application = `apd.description`) to `apd`,
/// so the dumped file's APD agrees with the `FILE_SCHEMA` header.
fn stamp_apd(model: &mut StepModel, apd: &ApdInfo) {
    let status = model.add_text(apd.status);
    let name = model.add_text(apd.name);
    let description = model.add_text(apd.description);
    for x in &mut model.application_protocol_definition_arena.items {
        x.status = status;
        x.application_interpreted_model_schema_name = name;
        x.application_protocol_year = apd.year;
    }
    for ac in &mut model.application_context_arena.items {
        ac.application = description;
    }
}

/// Part 21 string literal for the HEADER section, written like the DATA
/// section's strings (plain ASCII, see `crate::parser::string::encode`).
fn header_str(s: &str) -> String {
    crate::parser::string::encode(s)
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

/// The Part 21 text up to the DATA section's entities, built from `header`
/// — `FILE_SCHEMA` from [`FileHeader::schema`]'s raw strings (an empty raw
/// renders as the customary empty `('')`).
fn header_text(header: &FileHeader) -> String {
    let schema = if header.schema.raw.is_empty() {
        "''".to_owned()
    } else {
        header
            .schema
            .raw
            .iter()
            .map(|s| header_str(s))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION({desc},'2;1');\n\
         FILE_NAME({name},{stamp},{authors},{orgs},{pre},{orig},{auth});\n\
         FILE_SCHEMA(({schema}));\nENDSEC;\nDATA;\n",
        desc = header_list(&header.descriptions),
        name = header_str(&header.file_name),
        stamp = header_str(&header.time_stamp),
        authors = header_list(&header.authors),
        orgs = header_list(&header.organizations),
        pre = header_str(&header.preprocessor_version),
        orig = header_str(&header.originating_system),
        auth = header_str(&header.authorisation),
    )
}
