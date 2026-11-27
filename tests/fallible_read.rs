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
/// `entities_kept + dropped.len() == entities_read + entities_added` (every
/// input/synthetic entity is either kept or dropped exactly once — this
/// exercises the read-failure feedback path that well-formed files never
/// touch).
fn read_body(body: &str) -> Report {
    let src = format!("{HEADER}{body}{FOOTER}");
    let (_model, report) = read(src.as_bytes()).expect("source parses");
    assert_eq!(
        report.entities_kept + report.dropped.len(),
        report.entities_read + report.entities_added,
        "accounting invariant: entities_kept + dropped == entities_read + entities_added"
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
    assert!(r.entities_kept >= 1);
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

/// A complex instance fits a slot only through one of its parts. A unit slot
/// takes the complex millimetre `#1` but not the complex context `#2`, which
/// is dropped from `#4` as a wrong-type reference, the same as a simple
/// entity of the wrong type would be.
#[test]
fn wrong_type_complex_ref_drops_only_offender() {
    let r = read_body(
        "#1=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\n\
         #2=(GEOMETRIC_REPRESENTATION_CONTEXT(3)PARAMETRIC_REPRESENTATION_CONTEXT()\
         REPRESENTATION_CONTEXT('',''));\n\
         #3=MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#1);\n\
         #4=MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#2);\n",
    );
    assert_eq!(dropped_kind(&r, 4), Some(&DropKind::NonstandardReference));
    for id in 1..=3 {
        assert!(dropped_kind(&r, id).is_none(), "#{id} must survive: {r:?}");
    }
}

/// Every edition narrows a presentation's context to a geometric one
/// (`SELF\representation.context_of_items : geometric_representation_context`).
/// `#4` names a plain context and is dropped as a wrong-type reference; `#3`,
/// over a geometric context, is kept.
#[test]
fn narrowed_attribute_drops_a_wider_reference() {
    let r = read_body(
        "#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2=(GEOMETRIC_REPRESENTATION_CONTEXT(3)REPRESENTATION_CONTEXT('',''));\n\
         #3=PRESENTATION_VIEW('',(#1),#2);\n\
         #5=REPRESENTATION_CONTEXT('','');\n\
         #4=PRESENTATION_VIEW('',(#1),#5);\n",
    );
    assert_eq!(dropped_kind(&r, 4), Some(&DropKind::NonstandardReference));
    assert!(dropped_kind(&r, 3).is_none(), "#3 must survive: {r:?}");
}

/// The same narrowing made by one part of a complex instance holds for the
/// attribute another part declares: `PRESENTATION_REPRESENTATION` narrows
/// `REPRESENTATION.context_of_items`, so `#4` over a plain context is dropped
/// and `#3` over a geometric one is kept.
#[test]
fn narrowing_by_one_complex_part_holds_for_another() {
    let r = read_body(
        "#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2=(GEOMETRIC_REPRESENTATION_CONTEXT(3)REPRESENTATION_CONTEXT('',''));\n\
         #3=(PRESENTATION_REPRESENTATION()REPRESENTATION('',(#1),#2));\n\
         #5=REPRESENTATION_CONTEXT('','');\n\
         #4=(PRESENTATION_REPRESENTATION()REPRESENTATION('',(#1),#5));\n",
    );
    assert_eq!(dropped_kind(&r, 4), Some(&DropKind::NonstandardReference));
    assert!(dropped_kind(&r, 3).is_none(), "#3 must survive: {r:?}");
}

/// AP214 lets a shape definition representation describe a shape aspect
/// directly; only AP242 narrows `definition` to a property definition. Reading
/// accepts what any edition allows, so the aspect form is kept.
#[test]
fn older_edition_form_of_a_narrowed_attribute_is_kept() {
    let r = read_body(
        "#1=APPLICATION_CONTEXT('test');\n\
         #2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
         #3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
         #10=PRODUCT('p','Part','',(#2));\n\
         #11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
         #12=PRODUCT_DEFINITION('design','',#11,#3);\n\
         #13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
         #20=CARTESIAN_POINT('',(0.,0.,0.));\n\
         #36=REPRESENTATION_CONTEXT('','3D');\n\
         #37=SHAPE_REPRESENTATION('',(#20),#36);\n\
         #40=SHAPE_ASPECT('hole','',#13,.T.);\n\
         #50=SHAPE_DEFINITION_REPRESENTATION(#40,#37);\n",
    );
    assert!(r.dropped.is_empty(), "{:?}", r.dropped);
}

/// AP203 and AP214 let a styled item style any representation item; AP242
/// types it as a `styled_item_target`, which a descriptive item is not.
/// Reading accepts what any edition allows, so the AP214 form is kept.
#[test]
fn older_edition_declaration_of_an_attribute_is_kept() {
    let r = read_body(
        "#1=DESCRIPTIVE_REPRESENTATION_ITEM('note','text');\n\
         #2=COLOUR_RGB('',0.8,0.2,0.1);\n\
         #3=FILL_AREA_STYLE_COLOUR('',#2);\n\
         #4=FILL_AREA_STYLE('',(#3));\n\
         #5=SURFACE_STYLE_FILL_AREA(#4);\n\
         #6=SURFACE_SIDE_STYLE('',(#5));\n\
         #7=SURFACE_STYLE_USAGE(.BOTH.,#6);\n\
         #8=PRESENTATION_STYLE_ASSIGNMENT((#7));\n\
         #9=STYLED_ITEM('',(#8),#1);\n",
    );
    assert!(r.dropped.is_empty(), "{:?}", r.dropped);
}

/// AP242 edition 1 narrowed a tessellated shape representation's items to
/// tessellated items; edition 2 dropped that. A PMI drawing model that is also
/// a tessellated shape representation, holding a non-tessellated item as
/// edition 2 allows, is kept.
#[test]
fn a_narrowing_only_one_edition_made_is_not_enforced() {
    let r = read_body(
        "#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2=(GEOMETRIC_REPRESENTATION_CONTEXT(3)REPRESENTATION_CONTEXT('',''));\n\
         #3=(DRAUGHTING_MODEL()REPRESENTATION('',(#1),#2)SHAPE_REPRESENTATION()\
         TESSELLATED_SHAPE_REPRESENTATION());\n",
    );
    assert!(r.dropped.is_empty(), "{:?}", r.dropped);
}

/// A fully valid body drops nothing and keeps everything.
#[test]
fn valid_body_keeps_all() {
    let r = read_body("#1=CARTESIAN_POINT('',(0.,0.,0.));\n#2=DIRECTION('',(1.,0.,0.));\n");
    assert_eq!(r.dropped.len(), 0);
    assert_eq!(r.entities_kept, 2);
    assert!(r.parse_warnings.is_empty(), "{:?}", r.parse_warnings);
}

/// A repair the parser makes reaches the caller of `read()`, with a message
/// that says where and what.
#[test]
fn parser_repairs_are_reported() {
    let r = read_body("#1=CARTESIAN_POINT('NAUO8\\Redondeo1',(0.,0.,0.));\n");
    assert_eq!(r.entities_kept, 1, "the entity is kept");
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
                (slot, format!("{:?}: {}", d.kind, d.detail))
            })
            .collect();
        drops.sort();
        seen.push((
            r.entities_read,
            r.entities_kept,
            drops,
            common::text(&model),
        ));
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
        "#10=PRESENTATION_LAYER_ASSIGNMENT('l','',());",
    ];
    let read_lines = |lines: &[&str]| {
        let src = format!("{HEADER}{}\n{FOOTER}", lines.join("\n"));
        let (model, mut r) = read(src.as_bytes()).expect("source parses");
        r.normalized.sort_unstable();
        r.kept_nonstandard.sort_unstable();
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
/// written in either order read the same, but for the order of the notes.
#[test]
fn synthetic_colour_is_kept_and_numbered_by_its_creator() {
    let renderings = [
        "#20=SURFACE_STYLE_RENDERING(.NORMAL_SHADING.,$);",
        "#10=SURFACE_STYLE_RENDERING_WITH_PROPERTIES(.NORMAL_SHADING.,$,(#999));",
    ];
    let read_lines = |lines: &[&str]| {
        let src = format!("{HEADER}{}\n{FOOTER}", lines.join("\n"));
        let (model, mut r) = read(src.as_bytes()).expect("source parses");
        r.normalized.sort_unstable();
        (common::text(&model), r)
    };
    let (text, r) = read_lines(&renderings);
    assert_eq!((r.entities_read, r.entities_added), (2, 2));
    assert_eq!(dropped_kind(&r, 10), Some(&DropKind::Cascade));
    assert_eq!(r.entities_kept, 3, "#20 and both colours: {:?}", r.dropped);
    assert_eq!(text.matches("COLOUR()").count(), 2, "{text}");
    let colour_note = "missing surface colour set to an unspecified COLOUR";
    assert_eq!(r.normalized, [(10, colour_note), (20, colour_note)]);
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
            .map(|(_, d)| d.detail.as_str())
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

/// An entity that fails to read takes its referrers with it, as a cascade,
/// and the read still accounts for every entity once.
#[test]
fn read_failures_reach_their_referrers() {
    let r = read_body(
        "#1=CARTESIAN_POINT('',('x','y','z'));\n\
         #2=VERTEX_POINT('',#1);\n\
         #3=VERTEX_POINT('',#4);\n\
         #4=CARTESIAN_POINT('',(0.,0.,0.));\n",
    );
    assert_eq!(dropped_kind(&r, 1), Some(&DropKind::Unclassified));
    let d2 = r.dropped.iter().find(|(i, _)| *i == 2).map(|(_, d)| d);
    let d2 = d2.unwrap_or_else(|| panic!("{:?}", r.dropped));
    assert_eq!(
        (&d2.kind, d2.detail.as_str()),
        (&DropKind::Cascade, "via-unclassified")
    );
    assert_eq!(r.entities_kept, 2, "{:?}", r.dropped);
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

/// A file that does not end as Part 21 requires after its last entity is an
/// error, with no repeated id to report ahead of it.
#[test]
fn a_bad_trailer_is_an_error() {
    let body = "#1=CARTESIAN_POINT('',(0.,0.,0.));\n";
    let src = format!("{HEADER}{body}ENDSEC;\n");
    assert!(matches!(
        read(src.as_bytes()),
        Err(step_io::Error::UnexpectedEof { .. })
    ));
    let src = format!("{HEADER}{body}{FOOTER}#7=CARTESIAN_POINT('',(0.,0.,0.));\n");
    assert!(matches!(
        read(src.as_bytes()),
        Err(step_io::Error::UnexpectedToken {
            expected: "end of file",
            ..
        })
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
            .map(|(_, d)| d.detail.as_str())
    };
    assert_eq!(key(1), Some("list has more values than its type allows"));
    assert_eq!(dropped_kind(&r, 2), Some(&DropKind::Cascade));
    assert_eq!(r.entities_kept, 1, "{:?}", r.dropped);
}

const SHORT: &str = "list shorter than its schema bounds";
const LONG: &str = "list longer than its schema bounds";

/// A complex entity's parts out of name order are put in it, with a note,
/// and what references the entity still resolves to it; parts already in
/// order get no note.
#[test]
fn complex_parts_out_of_order_are_sorted_and_noted() {
    let body = "#1=(SI_UNIT(.MILLI.,.METRE.)NAMED_UNIT(*)LENGTH_UNIT());\n\
                #2=(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.));\n\
                #3=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-07),#1,'d','');\n";
    let r = read_body(body);
    assert!(r.dropped.is_empty(), "{:?}", r.dropped);
    assert_eq!(
        r.normalized,
        [(1, "complex entity parts sorted into name order")]
    );
    let (model, _) = read(format!("{HEADER}{body}{FOOTER}").as_bytes()).expect("source parses");
    let text = common::text(&model);
    assert!(
        text.contains("#1 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );"),
        "{text}"
    );
    assert!(
        text.contains("UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.E-7),#1,"),
        "{text}"
    );
}

/// A reference repeated in a SET is kept once, with a note; the same in a
/// SELECT's named SET (`SET_REPRESENTATION_ITEM`).
#[test]
fn duplicate_set_elements_are_removed_and_noted() {
    let body = "#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
                #2=PRESENTATION_LAYER_ASSIGNMENT('l','',(#1,#1));\n\
                #3=COMPOUND_REPRESENTATION_ITEM('',SET_REPRESENTATION_ITEM((#1,#1)));\n";
    let r = read_body(body);
    assert!(r.dropped.is_empty(), "{:?}", r.dropped);
    let note = "duplicate set element removed";
    assert_eq!(r.normalized, [(2, note), (3, note)]);
    assert!(r.kept_nonstandard.is_empty(), "{:?}", r.kept_nonstandard);
    let (model, _) = read(format!("{HEADER}{body}{FOOTER}").as_bytes()).expect("source parses");
    let text = common::text(&model);
    for name in [
        "PRESENTATION_LAYER_ASSIGNMENT(",
        "COMPOUND_REPRESENTATION_ITEM(",
    ] {
        let line = text.lines().find(|l| l.contains(name)).expect(name);
        assert_eq!(line.matches('#').count(), 2, "{line}");
    }
}

/// A sphere's face with its bounds left empty, below `SET [1:?]`, is kept
/// with the solid it bounds, and noted.
#[test]
fn list_below_its_lower_bound_is_kept_and_noted() {
    let r = read_body(
        "#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2=AXIS2_PLACEMENT_3D('',#1,$,$);\n\
         #3=SPHERICAL_SURFACE('',#2,4.05);\n\
         #4=ADVANCED_FACE('',(),#3,.T.);\n\
         #5=CLOSED_SHELL('',(#4));\n\
         #6=MANIFOLD_SOLID_BREP('',#5);\n\
         #7=COMPOUND_REPRESENTATION_ITEM('',SET_REPRESENTATION_ITEM(()));\n",
    );
    assert!(r.dropped.is_empty(), "{:?}", r.dropped);
    assert_eq!(r.entities_kept, 7);
    assert!(r.normalized.is_empty(), "{:?}", r.normalized);
    assert_eq!(r.kept_nonstandard, [(4, SHORT), (7, SHORT)]);
}

/// An OPTIONAL list given no elements where `LIST [1:?]` needs one reads as
/// unset, with a note.
#[test]
fn empty_optional_list_that_needs_elements_reads_as_unset() {
    let body = "#1=PERSON('1',$,'Doe',(),('A'),$);\n";
    let r = read_body(body);
    assert!(r.dropped.is_empty(), "{:?}", r.dropped);
    assert_eq!(r.normalized, [(1, "empty optional list set to unset")]);
    assert!(r.kept_nonstandard.is_empty(), "{:?}", r.kept_nonstandard);
    let (model, _) = read(format!("{HEADER}{body}{FOOTER}").as_bytes()).expect("source parses");
    let text = common::text(&model);
    assert!(text.contains("PERSON('1',$,'Doe',$,('A'),$);"), "{text}");
}

/// Three trims where `SET [1:2]` allows two are kept, and noted.
#[test]
fn list_above_its_upper_bound_is_kept_and_noted() {
    let r = read_body(
        "#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2=DIRECTION('',(1.,0.,0.));\n\
         #3=VECTOR('',#2,1.);\n\
         #4=LINE('',#1,#3);\n\
         #5=TRIMMED_CURVE('',#4,(PARAMETER_VALUE(0.),PARAMETER_VALUE(1.),\
         PARAMETER_VALUE(2.)),(PARAMETER_VALUE(3.)),.T.,.PARAMETER.);\n",
    );
    assert!(r.dropped.is_empty(), "{:?}", r.dropped);
    assert!(r.normalized.is_empty(), "{:?}", r.normalized);
    assert_eq!(r.kept_nonstandard, [(5, LONG)]);
}

/// A control-point row of one point, where each row of
/// `LIST [2:?] OF LIST [2:?]` needs two, is kept, and noted.
#[test]
fn nested_list_below_its_lower_bound_is_kept_and_noted() {
    let r = read_body(
        "#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
         #2=CARTESIAN_POINT('',(1.,0.,0.));\n\
         #3=CARTESIAN_POINT('',(0.,1.,0.));\n\
         #4=B_SPLINE_SURFACE_WITH_KNOTS('',1,1,((#1,#2),(#3)),.UNSPECIFIED.,.F.,.F.,.F.,\
         (2,2),(2,2),(0.,1.),(0.,1.),.UNSPECIFIED.);\n",
    );
    assert!(r.dropped.is_empty(), "{:?}", r.dropped);
    assert!(r.normalized.is_empty(), "{:?}", r.normalized);
    assert_eq!(r.kept_nonstandard, [(4, SHORT)]);
}

/// A BOOLEAN reads `.T.` as true and `.F.` as false; anything else fails the
/// read, and what references the entity goes with it. An edge with a
/// dangling reference is dropped for that first.
#[test]
fn boolean_neither_true_nor_false_drops_the_entity() {
    let body = "#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
                #2=VERTEX_POINT('',#1);\n\
                #3=DIRECTION('',(1.,0.,0.));\n\
                #4=VECTOR('',#3,1.);\n\
                #5=LINE('',#1,#4);\n\
                #10=EDGE_CURVE('',#2,#2,#5,.T.);\n\
                #11=EDGE_CURVE('',#2,#2,#5,.F.);\n\
                #12=EDGE_CURVE('',#2,#2,#5,.U.);\n\
                #13=EDGE_CURVE('',#2,#2,#5,$);\n\
                #14=EDGE_CURVE('',#2,#2,#5,1);\n\
                #15=EDGE_CURVE('',#2,#2,#0,);\n\
                #20=ORIENTED_EDGE('',*,*,#12,.T.);\n";
    let r = read_body(body);
    let detail = |id: u64| {
        r.dropped
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, d)| d.detail.as_str())
    };
    for (id, got) in [(12, "Enum(\"U\")"), (13, "Unset"), (14, "Integer(1)")] {
        assert_eq!(dropped_kind(&r, id), Some(&DropKind::Unclassified));
        let want = format!("EDGE_CURVE.same_sense: expected boolean, got {got}");
        assert_eq!(detail(id), Some(want.as_str()));
    }
    assert_eq!(dropped_kind(&r, 15), Some(&DropKind::Cascade));
    assert_eq!(detail(15), Some("via-dangling"));
    assert_eq!(dropped_kind(&r, 20), Some(&DropKind::Cascade));
    assert_eq!(detail(20), Some("via-unclassified"));
    assert_eq!(r.dropped.len(), 5, "{:?}", r.dropped);
    let (model, _) = read(format!("{HEADER}{body}{FOOTER}").as_bytes()).expect("source parses");
    let senses: Vec<bool> = model
        .all::<step_io::model::EdgeCurve>()
        .map(|(_, e)| e.same_sense)
        .collect();
    assert_eq!(senses, [true, false]);
}

/// An entity, or a complex entity's part, with more attributes than it has
/// is dropped as a nonstandard value; a trailing comma is one more.
#[test]
fn extra_attributes_drop_the_entity() {
    let r = read_body(
        "#1=CARTESIAN_POINT('',(0.,0.,0.),5.);\n\
         #2=CARTESIAN_POINT('',(0.,0.,0.),);\n\
         #3=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.,1));\n\
         #4=(LENGTH_UNIT(1)NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\n\
         #5=CARTESIAN_POINT('',(0.,0.,0.));\n\
         #6=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\n",
    );
    for id in 1..=4 {
        let d = r.dropped.iter().find(|(i, _)| *i == id).map(|(_, d)| d);
        let d = d.unwrap_or_else(|| panic!("#{id} kept: {:?}", r.dropped));
        assert_eq!(d.kind, DropKind::NonstandardValue);
        assert_eq!(d.detail, "more attributes than the entity has");
    }
    assert_eq!(r.entities_kept, 2, "{:?}", r.dropped);
}

/// A complex entity whose parts the schema does not combine in any edition is
/// dropped as a nonstandard value, with the rule it breaks.
#[test]
fn complex_entities_the_schema_does_not_combine_are_dropped() {
    let r = read_body(
        "#1=(LENGTH_UNIT()LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\n\
         #2=(LENGTH_UNIT()SI_UNIT(.MILLI.,.METRE.));\n\
         #3=(LENGTH_UNIT()NAMED_UNIT(*)REPRESENTATION_CONTEXT('','')SI_UNIT(.MILLI.,.METRE.));\n\
         #4=(LENGTH_UNIT()MASS_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\n\
         #5=(CAMERA_MODEL()GEOMETRIC_REPRESENTATION_ITEM()REPRESENTATION_ITEM(''));\n\
         #6=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));\n",
    );
    let detail = |id: u64| {
        r.dropped
            .iter()
            .find(|(i, d)| *i == id && d.kind == DropKind::NonstandardValue)
            .map(|(_, d)| d.detail.as_str())
    };
    assert_eq!(detail(1), Some("complex entity repeats a part"));
    assert_eq!(detail(2), Some("complex entity lacks a supertype part"));
    assert_eq!(detail(3), Some("complex entity joins unrelated parts"));
    assert_eq!(
        detail(4),
        Some("complex entity has parts of two ONEOF branches")
    );
    assert_eq!(
        detail(5),
        Some("complex entity has an abstract part without a subtype part")
    );
    assert_eq!(r.entities_kept, 1, "{:?}", r.dropped);
}

/// A combination one edition allows is kept: an area unit that is a named
/// unit (AP203 edition 1), and an item both mapped and styled (AP214).
#[test]
fn older_edition_complex_forms_are_kept() {
    let r = read_body(
        "#1=(AREA_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));\n\
         #2=CARTESIAN_POINT('',(0.,0.,0.));\n\
         #3=REPRESENTATION_CONTEXT('','');\n\
         #4=SHAPE_REPRESENTATION('',(#2),#3);\n\
         #5=REPRESENTATION_MAP(#2,#4);\n\
         #6=CARTESIAN_POINT('',(1.,0.,0.));\n\
         #7=COLOUR_RGB('',0.8,0.2,0.1);\n\
         #8=FILL_AREA_STYLE_COLOUR('',#7);\n\
         #9=FILL_AREA_STYLE('',(#8));\n\
         #10=SURFACE_STYLE_FILL_AREA(#9);\n\
         #11=SURFACE_SIDE_STYLE('',(#10));\n\
         #12=SURFACE_STYLE_USAGE(.BOTH.,#11);\n\
         #13=PRESENTATION_STYLE_ASSIGNMENT((#12));\n\
         #14=(MAPPED_ITEM(#5,#6)REPRESENTATION_ITEM('')STYLED_ITEM((#13),#6));\n",
    );
    assert!(r.dropped.is_empty(), "{:?}", r.dropped);
}
