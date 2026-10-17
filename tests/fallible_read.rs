//! The generated reader is per-entity fallible: a malformed entity is dropped
//! with a reason (never panicking, never failing the whole read), and valid
//! entities around it survive. These probes feed deliberately broken DATA bodies
//! (off-kind scalar, short arity, non-list vec, dangling/wrong-type ref) — they
//! are adversarial byte inputs, not valid-CAD fixtures.

use step_io::{DropKind, ParseWarning, Report, read, write};

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
/// DIRECTION. `#2` is dropped (`NonstandardReference` via graph, or `Unclassified`
/// via read — either is a graceful drop); the valid `#1` survives. No panic.
#[test]
fn wrong_type_ref_drops_only_offender() {
    let r = read_body("#1=DIRECTION('',(1.,0.,0.));\n#2=VERTEX_POINT('',#1);\n");
    let k = dropped_kind(&r, 2);
    assert!(
        matches!(
            k,
            Some(DropKind::NonstandardReference | DropKind::Unclassified)
        ),
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
    assert!(r.parse_warnings.is_empty(), "{:?}", r.parse_warnings);
}

/// A repair the parser makes reaches the caller of `read()`, with a message
/// that says where and what.
#[test]
fn parser_repairs_are_reported() {
    let r = read_body("#1=CARTESIAN_POINT('NAUO8\\Redondeo1',(0.,0.,0.));\n");
    assert_eq!(r.validated, 1, "the entity is kept");
    assert!(
        matches!(
            r.parse_warnings[..],
            [ParseWarning::MalformedStringEscape { .. }]
        ),
        "{:?}",
        r.parse_warnings
    );
    // The body starts on line 8, after the seven header lines; the string is
    // the token at column 20.
    let message = r.parse_warnings[0].to_string();
    assert_eq!(
        message,
        "warning at line 8, column 20: string escape could not be decoded; kept as written"
    );
}

/// The same entities under three numberings: dense, stepped by ten (as some
/// exporters write), and with one far-off id. Each reads to the same model and
/// the same drops — a dangling ref, its cascade, and a wrong-type ref
/// included — since the writer renumbers, the rewritten files match exactly.
#[test]
fn id_layout_does_not_change_the_read() {
    // Slot k of the body gets id `ids[k]`; slot 8 is never defined (dangling).
    fn body(ids: &[u64; 9]) -> String {
        let [p, d, v, l, vp, dangle, wrong, edge, missing] = *ids;
        format!(
            "#{p}=CARTESIAN_POINT('',(0.,0.,0.));\n\
             #{d}=DIRECTION('',(1.,0.,0.));\n\
             #{v}=VECTOR('',#{d},1.);\n\
             #{l}=LINE('',#{p},#{v});\n\
             #{vp}=VERTEX_POINT('',#{p});\n\
             #{dangle}=VERTEX_POINT('',#{missing});\n\
             #{wrong}=VERTEX_POINT('',#{d});\n\
             #{edge}=EDGE_CURVE('',#{dangle},#{vp},#{l},.T.);\n"
        )
    }
    let layouts: [[u64; 9]; 3] = [
        [1, 2, 3, 4, 5, 6, 7, 8, 9],
        [10, 20, 30, 40, 50, 60, 70, 80, 90],
        [1, 2, 3, 4, 5, 6, 7, 5_000_000, 999],
    ];
    let mut seen = Vec::new();
    for ids in &layouts {
        let src = format!("{HEADER}{}{FOOTER}", body(ids));
        let (model, r) = read(src.as_bytes()).expect("source parses");
        // Drops by body slot, so the layouts compare directly.
        let mut drops: Vec<(usize, String)> = r
            .dropped
            .iter()
            .map(|(id, d)| {
                let slot = ids
                    .iter()
                    .position(|i| i == id)
                    .expect("dropped id is in the body");
                (slot, format!("{:?}: {}", d.kind, d.key))
            })
            .collect();
        drops.sort();
        seen.push((r.n_in, r.validated, drops, write(&model)));
    }
    let (n_in, validated, drops, _) = &seen[0];
    assert_eq!((*n_in, *validated), (8, 5), "drops: {drops:?}");
    assert_eq!(
        drops.iter().map(|(slot, _)| *slot).collect::<Vec<_>>(),
        [5, 6, 7],
        "the dangling ref, the wrong-type ref, and the edge on the dangling vertex"
    );
    for (ids, other) in layouts.iter().zip(&seen).skip(1) {
        assert_eq!(other, &seen[0], "layout {ids:?}");
    }
}
