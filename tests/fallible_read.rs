//! The generated reader is per-entity fallible: a malformed entity is dropped
//! with a reason (never panicking, never failing the whole read), and valid
//! entities around it survive. These probes feed deliberately broken DATA bodies
//! (off-kind scalar, short arity, non-list vec, dangling/wrong-type ref) — they
//! are adversarial byte inputs, not valid-CAD fixtures.

mod common;

use step_io::{DropKind, ParseWarning, Report, read};

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
        seen.push((r.n_in, r.validated, drops, common::text(&model)));
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

/// Entities written out of id order read the same as in id order: the model
/// is built in id order, and only the order of the normalization notes, which
/// follows the file, differs.
#[test]
fn file_order_does_not_change_the_read() {
    let lines = [
        "#1=CARTESIAN_POINT($,(0.,0.,0.));",
        "#2=DIRECTION('',(1.,0.,0.));",
        "#3=VECTOR('',#2,1);",
        "#4=LINE('',#1,#3);",
        "#5=VERTEX_POINT('',#1);",
        "#6=VERTEX_POINT('',#9);",
        "#7=VERTEX_POINT('',#2);",
        "#8=EDGE_CURVE('',#6,#5,#4,.T.);",
    ];
    let read_lines = |lines: &[&str]| {
        let src = format!("{HEADER}{}\n{FOOTER}", lines.join("\n"));
        let (model, mut r) = read(src.as_bytes()).expect("source parses");
        r.norm.sort_unstable();
        (common::text(&model), format!("{r:?}"))
    };
    let mut reversed = lines;
    reversed.reverse();
    let in_order = read_lines(&lines);
    assert_eq!(read_lines(&reversed), in_order);
    assert!(in_order.0.contains("VERTEX_POINT"));
}

/// A rendering with no colour gets a bare `COLOUR()`, which stays in the model
/// even when the rendering itself is dropped later. Two such renderings
/// written in either order read the same.
#[test]
fn synthetic_colour_is_kept_and_numbered_by_its_creator() {
    let renderings = [
        "#20=SURFACE_STYLE_RENDERING(.NORMAL_SHADING.,$);",
        "#10=SURFACE_STYLE_RENDERING_WITH_PROPERTIES(.NORMAL_SHADING.,$,(#999));",
    ];
    let read_lines = |lines: &[&str]| {
        let src = format!("{HEADER}{}\n{FOOTER}", lines.join("\n"));
        let (model, r) = read(src.as_bytes()).expect("source parses");
        (common::text(&model), r)
    };
    let (text, r) = read_lines(&renderings);
    assert_eq!((r.n_in, r.n_synth), (2, 2));
    assert_eq!(dropped_kind(&r, 10), Some(&DropKind::Cascade));
    assert_eq!(r.validated, 3, "#20 and both colours: {:?}", r.dropped);
    assert_eq!(text.matches("COLOUR()").count(), 2, "{text}");
    let (text_rev, r_rev) = read_lines(&[renderings[1], renderings[0]]);
    assert_eq!(text_rev, text);
    assert_eq!(format!("{r_rev:?}"), format!("{r:?}"));
}

/// An entity normalization drops is named by its reason in its referrers'
/// cascade, and the drops come back in id order.
#[test]
fn normalization_drops_name_their_reason_in_cascades() {
    let r = read_body(
        "#5=NOT_A_SCHEMA_ENTITY('x');\n\
         #3=EDGE_CURVE('',#1,#1,#4,.T.);\n\
         #1=VERTEX_POINT('',$);\n\
         #4=LINE('',#6,#7);\n",
    );
    let ids: Vec<u64> = r.dropped.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, [1, 3, 4, 5], "{:?}", r.dropped);
    let key = |id: u64| {
        r.dropped
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, d)| d.key.as_str())
    };
    assert_eq!(dropped_kind(&r, 1), Some(&DropKind::NonstandardValue));
    assert_eq!(key(3), Some("via-required reference is missing"));
    assert_eq!(key(4), Some("via-dangling"));
    assert_eq!(dropped_kind(&r, 5), Some(&DropKind::Unimplemented));
}

/// A complex entity's references resolve to the parts that hold them, also
/// when a part without references comes first.
#[test]
fn complex_entity_references_resolve_in_place() {
    let body = "#1=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\n\
                #2=(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.));\n\
                #3=(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT());\n\
                #4=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-07),#1,'distance_accuracy_value','');\n\
                #5=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#4))\
                GLOBAL_UNIT_ASSIGNED_CONTEXT((#1,#2,#3))REPRESENTATION_CONTEXT('',''));\n";
    let r = read_body(body);
    assert!(r.dropped.is_empty(), "{:?}", r.dropped);
    let (model, _) = read(format!("{HEADER}{body}{FOOTER}").as_bytes()).expect("source parses");
    let text = common::text(&model);
    let (again, r2) = read(text.as_bytes()).expect("written file parses");
    assert!(r2.dropped.is_empty(), "{:?}", r2.dropped);
    assert_eq!(common::text(&again), text);
    assert!(text.contains("GLOBAL_UNIT_ASSIGNED_CONTEXT((#"), "{text}");
}

/// An entity that fails to read takes its referrers with it, and the read
/// still accounts for every entity once.
#[test]
fn read_failures_reach_their_referrers() {
    let r = read_body(
        "#1=CARTESIAN_POINT('',('x','y','z'));\n\
         #2=VERTEX_POINT('',#1);\n\
         #3=VERTEX_POINT('',#4);\n\
         #4=CARTESIAN_POINT('',(0.,0.,0.));\n",
    );
    assert_eq!(dropped_kind(&r, 1), Some(&DropKind::Unclassified));
    assert!(dropped_kind(&r, 2).is_some(), "{:?}", r.dropped);
    assert_eq!(r.validated, 2, "{:?}", r.dropped);
}

/// A repeated id is an error, reported at the repeat that comes first in the
/// file, even when the file breaks off after it.
#[test]
fn duplicate_ids_are_reported_at_the_first_repeat() {
    let body = "#5=CARTESIAN_POINT('',(0.,0.,0.));\n\
                #3=CARTESIAN_POINT('',(0.,0.,0.));\n\
                #5=CARTESIAN_POINT('',(0.,0.,0.));\n\
                #3=CARTESIAN_POINT('',(0.,0.,0.));\n";
    for footer in [FOOTER, "ENDSEC;\n", "#9=CARTESIAN_POINT(\n"] {
        let src = format!("{HEADER}{body}{footer}");
        match read(src.as_bytes()) {
            Err(step_io::Error::DuplicateEntityId {
                id,
                line,
                first_line,
            }) => {
                // The body starts on line 8.
                assert_eq!((id, line, first_line), (5, 10, 8), "footer {footer:?}");
            }
            other => panic!("footer {footer:?}: {other:?}"),
        }
        assert!(matches!(
            step_io::parser::parse(src.as_bytes()),
            Err(step_io::Error::DuplicateEntityId {
                id: 5,
                line: 10,
                first_line: 8
            })
        ));
    }
    // A repeat after the point where the file breaks is never reached.
    let src = format!("{HEADER}#1=CARTESIAN_POINT(;\n#1=CARTESIAN_POINT('',(0.,0.,0.));\n{FOOTER}");
    assert!(matches!(
        read(src.as_bytes()),
        Err(e) if !matches!(e, step_io::Error::DuplicateEntityId { .. })
    ));
}

/// A point with more coordinates than `LIST [1:3]` allows is dropped as a
/// nonstandard value, and what references it cascades; a two-coordinate point
/// is within the bounds and kept.
#[test]
fn coordinates_past_the_bound_drop_the_point() {
    let r = read_body(
        "#1=CARTESIAN_POINT('',(0.,0.,0.,0.));\n\
         #2=VERTEX_POINT('',#1);\n\
         #3=CARTESIAN_POINT('',(0.,0.));\n",
    );
    assert_eq!(dropped_kind(&r, 1), Some(&DropKind::NonstandardValue));
    let key = |id: u64| {
        r.dropped
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, d)| d.key.as_str())
    };
    assert_eq!(key(1), Some("list has more values than its type allows"));
    assert_eq!(dropped_kind(&r, 2), Some(&DropKind::Cascade));
    assert_eq!(r.validated, 1, "{:?}", r.dropped);
}
