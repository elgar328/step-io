//! The generated reader is per-entity fallible: a malformed entity is dropped
//! with a reason (never panicking, never failing the whole read), and valid
//! entities around it survive. These probes feed deliberately broken DATA bodies
//! (off-kind scalar, short arity, non-list vec, dangling/wrong-type ref) — they
//! are adversarial byte inputs, not valid-CAD fixtures.

use step_io::{DropKind, Report, read};

const HEADER: &str = "ISO-10303-21;\nHEADER;\n\
FILE_DESCRIPTION(('probe'),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

/// Read a DATA body wrapped in a standard header/footer, asserting the read does
/// not fail (parse ok) and the provenance accounting invariant holds:
/// `validated + dropped == n_in + n_synth` (every input/synthetic entity is
/// either kept or dropped exactly once — this exercises the read-failure feedback
/// path that the clean corpus never touches).
fn read_body(body: &str) -> Report {
    let src = format!("{HEADER}{body}{FOOTER}");
    let (_model, report) = read(src.as_bytes()).expect("source parses");
    assert_eq!(
        report.validated + report.dropped.len(),
        report.n_in + report.n_synth,
        "accounting invariant: validated + dropped == n_in + n_synth"
    );
    report
}

fn dropped_kind(r: &Report, id: u64) -> Option<&DropKind> {
    r.dropped
        .iter()
        .find(|(i, _)| *i == id)
        .map(|(_, d)| &d.kind)
}

/// Off-kind scalar: `#2` has strings where `coordinates` expects reals. `#2` is
/// dropped Unclassified; the valid `#1` survives. No panic.
#[test]
fn off_kind_scalar_drops_only_offender() {
    let r =
        read_body("#1=CARTESIAN_POINT('',(0.,0.,0.));\n#2=CARTESIAN_POINT('',('x','y','z'));\n");
    assert_eq!(dropped_kind(&r, 2), Some(&DropKind::Unclassified));
    assert!(dropped_kind(&r, 1).is_none(), "valid #1 must survive");
    assert!(r.validated >= 1);
}

/// Short arity: `#2` is missing its `coordinates` slot. Dropped Unclassified.
#[test]
fn short_arity_drops_only_offender() {
    let r = read_body("#1=CARTESIAN_POINT('',(0.,0.,0.));\n#2=CARTESIAN_POINT('');\n");
    assert_eq!(dropped_kind(&r, 2), Some(&DropKind::Unclassified));
    assert!(dropped_kind(&r, 1).is_none());
}

/// Non-list where a `LIST OF REAL` is expected: `coordinates` is a bare real.
#[test]
fn non_list_vec_drops_only_offender() {
    let r = read_body("#1=CARTESIAN_POINT('',(0.,0.,0.));\n#2=CARTESIAN_POINT('',5.);\n");
    assert_eq!(dropped_kind(&r, 2), Some(&DropKind::Unclassified));
    assert!(dropped_kind(&r, 1).is_none());
}

/// Dangling ref: `#2` references a non-existent `#999`. The graph cascade drops
/// it (via-dangling); the valid `#1` survives. No panic.
#[test]
fn dangling_ref_cascades() {
    let r = read_body("#1=CARTESIAN_POINT('',(0.,0.,0.));\n#2=VERTEX_POINT('',#999);\n");
    assert_eq!(dropped_kind(&r, 2), Some(&DropKind::Cascade));
    assert!(dropped_kind(&r, 1).is_none());
}

/// Wrong-type ref: `VERTEX_POINT.vertex_geometry` must be a point, but `#1` is a
/// DIRECTION. `#2` is dropped (Nonstandard via graph, or Unclassified via read —
/// either is a graceful drop); the valid `#1` survives. No panic.
#[test]
fn wrong_type_ref_drops_only_offender() {
    let r = read_body("#1=DIRECTION('',(1.,0.,0.));\n#2=VERTEX_POINT('',#1);\n");
    let k = dropped_kind(&r, 2);
    assert!(
        matches!(k, Some(DropKind::Nonstandard | DropKind::Unclassified)),
        "wrong-type ref dropped, got {k:?}"
    );
    assert!(
        dropped_kind(&r, 1).is_none(),
        "valid DIRECTION #1 must survive"
    );
}

/// A fully valid body drops nothing and keeps everything.
#[test]
fn valid_body_keeps_all() {
    let r = read_body("#1=CARTESIAN_POINT('',(0.,0.,0.));\n#2=DIRECTION('',(1.,0.,0.));\n");
    assert_eq!(r.dropped.len(), 0);
    assert_eq!(r.validated, 2);
}
