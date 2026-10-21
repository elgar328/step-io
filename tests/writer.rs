//! `write` is a pure serializer — the model's own header (`FILE_SCHEMA`
//! included) plus every entity, verbatim, lossless. `dump_universal` re-marks
//! the model as step-io's non-standard universal union (`FILE_SCHEMA` and the
//! APD/`application_context` entities agree). Inline STEP docs, not valid-CAD
//! fixtures.

mod common;

use step_io::{read, write};

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

fn doc(body: &str) -> String {
    format!("{HEADER}{body}{FOOTER}")
}

#[test]
fn write_reserializes_faithfully_under_source_schema() {
    // No projection: every kept entity survives the write, and the source
    // FILE_SCHEMA is echoed — step-io does not convert between APs.
    let src = doc("#1=PRE_DEFINED_PRESENTATION_STYLE('pps');\n#2=STATE_OBSERVED('st',$);\n");
    let (model, rep) = read(src.as_bytes()).expect("read");
    assert_eq!(rep.dropped.len(), 0, "input clean: {:?}", rep.dropped);

    let out = common::text(&model);
    assert!(out.contains("STATE_OBSERVED"), "{out}");
    assert!(out.contains("PRE_DEFINED_PRESENTATION_STYLE"), "{out}");
    assert!(
        out.contains("FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }'))"),
        "source schema echoed:\n{out}"
    );

    // The output re-reads with zero drops.
    let (_m2, rep2) = read(out.as_bytes()).expect("re-read");
    assert_eq!(
        rep2.dropped.len(),
        0,
        "output not clean: {:?}",
        rep2.dropped
    );
}

#[test]
fn dump_universal_marks_header_and_apd() {
    // Input declares AP214-flavoured APD/AC; the dump absolutely sets BOTH the
    // FILE_SCHEMA header and the APD/AC entities to the universal marker so
    // the dump is internally consistent (header ↔ APD agree).
    let body = "#1=APPLICATION_CONTEXT('core data for automotive mechanical design processes');\n\
                #2=APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2010,#1);\n";
    let (mut model, _) = read(doc(body).as_bytes()).expect("read");

    let uni = universal(&mut model);
    assert!(uni.contains("FILE_SCHEMA(('STEPIO_UNIVERSAL'))"), "{uni}");
    assert!(
        uni.contains("APPLICATION_PROTOCOL_DEFINITION('not a standard','stepio_universal',0"),
        "{uni}"
    );
    assert!(
        uni.contains(
            "APPLICATION_CONTEXT('step-io universal union (non-standard, all-AP superset)')"
        ),
        "{uni}"
    );
}

/// The model as step-io's universal dump, through [`step_io::dump_universal`].
fn universal(model: &mut step_io::StepModel) -> String {
    let mut out = Vec::new();
    step_io::dump_universal(model, &mut out).expect("writing to a Vec does not fail");
    String::from_utf8(out).expect("Part 21 output is ASCII")
}

/// A builder model of `n` free vertices — about 100 bytes of text each.
fn vertices(n: u32) -> step_io::StepModel {
    let mut b = step_io::StepBuilder::new();
    for i in 0..n {
        b.vertex([f64::from(i), 0.0, 0.0]).expect("vertex");
    }
    b.finish()
}

/// Records each write the writer makes.
#[derive(Default)]
struct Writes {
    sizes: Vec<usize>,
    bytes: Vec<u8>,
    flushes: usize,
}

impl std::io::Write for Writes {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sizes.push(buf.len());
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.flushes += 1;
        Ok(())
    }
}

#[test]
fn write_sends_the_text_a_chunk_at_a_time() {
    const CHUNK: usize = 64 * 1024;
    let model = vertices(5_000);
    let mut out = Writes::default();
    write(&model, &mut out).expect("write");

    let text = common::text(&model);
    assert!(text.len() > 4 * CHUNK, "only {} bytes", text.len());
    assert_eq!(
        out.bytes,
        text.as_bytes(),
        "the same text, however it is sent"
    );
    assert!(out.sizes.len() > 4, "{} writes", out.sizes.len());
    let largest = out.sizes.iter().max().copied().unwrap_or(0);
    assert!(largest < 2 * CHUNK, "a write of {largest} bytes");
    assert_eq!(out.flushes, 1, "flushed once, at the end");
}

/// An output with room for `room` bytes, failing once it is full.
struct Full {
    room: usize,
}

impl std::io::Write for Full {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.room == 0 {
            return Err(std::io::Error::other("the disk is full"));
        }
        let taken = buf.len().min(self.room);
        self.room -= taken;
        Ok(taken)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn write_reports_a_failing_output() {
    let model = vertices(5_000);
    // Full at once, during the header, and partway through the entities.
    for room in [0, 100, 200_000] {
        let err = write(&model, Full { room }).expect_err("the output is full");
        assert_eq!(err.to_string(), "the disk is full", "room {room}");
    }
}
