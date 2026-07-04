//! Read geometry API spike: navigate a minimal b-rep solid from `solids()` down
//! to point coordinates, exercising single-variant refs, multi-variant refs,
//! `Vec<Ref>` chains, and `Option<Ref>` resolution. The input only needs to be
//! reference-resolvable (read does not enforce topological closure), so this is a
//! deliberately minimal solid: one planar face, one outer bound, one edge.

use step_io::read;
use step_io::scene::geometry::{CurveKind, SurfaceKind};

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

const BREP: &str = "\
#10=CARTESIAN_POINT('',(0.0,0.0,0.0));\n\
#11=CARTESIAN_POINT('',(1.0,0.0,0.0));\n\
#12=DIRECTION('',(0.0,0.0,1.0));\n\
#13=DIRECTION('',(1.0,0.0,0.0));\n\
#14=DIRECTION('',(1.0,0.0,0.0));\n\
#15=AXIS2_PLACEMENT_3D('',#10,#12,#13);\n\
#16=PLANE('',#15);\n\
#17=VECTOR('',#14,1.0);\n\
#18=LINE('',#10,#17);\n\
#19=VERTEX_POINT('',#10);\n\
#20=VERTEX_POINT('',#11);\n\
#21=EDGE_CURVE('',#19,#20,#18,.T.);\n\
#22=ORIENTED_EDGE('',*,*,#21,.T.);\n\
#23=EDGE_LOOP('',(#22));\n\
#24=FACE_OUTER_BOUND('',#23,.T.);\n\
#25=ADVANCED_FACE('',(#24),#16,.T.);\n\
#26=CLOSED_SHELL('',(#25));\n\
#27=MANIFOLD_SOLID_BREP('',#26);\n";

// A presentation colour styled onto face #25, anchored in an MDGPR so settling
// keeps it. Appended after BREP.
const COLOUR: &str = "\
#30=REPRESENTATION_CONTEXT('','3D');\n\
#31=COLOUR_RGB('',0.8,0.2,0.1);\n\
#32=FILL_AREA_STYLE_COLOUR('',#31);\n\
#33=FILL_AREA_STYLE('',(#32));\n\
#34=SURFACE_STYLE_FILL_AREA(#33);\n\
#35=SURFACE_SIDE_STYLE('',(#34));\n\
#36=SURFACE_STYLE_USAGE(.BOTH.,#35);\n\
#37=PRESENTATION_STYLE_ASSIGNMENT((#36));\n\
#38=STYLED_ITEM('',(#37),#25);\n\
#39=MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',(#38),#30);\n";

#[test]
fn reads_face_colour() {
    let src = format!("{HEADER}{BREP}{COLOUR}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let face = scene.all_faces().next().expect("a face");
    let c = face.color().expect("the face is coloured");
    assert!(
        (c.red - 0.8).abs() < 1e-9 && (c.green - 0.2).abs() < 1e-9 && (c.blue - 0.1).abs() < 1e-9,
        "colour = {c:?}"
    );
    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );

    // Without the styling chain, the same face has no colour.
    let plain = format!("{HEADER}{BREP}{FOOTER}");
    let (m2, _rep) = read(plain.as_bytes()).expect("read");
    assert!(
        m2.scene()
            .all_faces()
            .next()
            .expect("a face")
            .color()
            .is_none(),
        "unstyled face should have no colour"
    );
}

// A rendering style with a transparency property (and a colour) on face #25 —
// exercises both the transparency walk and the rendering colour path.
const RENDER: &str = "\
#30=REPRESENTATION_CONTEXT('','3D');\n\
#31=COLOUR_RGB('',0.5,0.5,0.5);\n\
#40=SURFACE_STYLE_TRANSPARENT(0.3);\n\
#41=SURFACE_STYLE_RENDERING_WITH_PROPERTIES(.NORMAL_SHADING.,#31,(#40));\n\
#42=SURFACE_SIDE_STYLE('',(#41));\n\
#43=SURFACE_STYLE_USAGE(.BOTH.,#42);\n\
#44=PRESENTATION_STYLE_ASSIGNMENT((#43));\n\
#45=STYLED_ITEM('',(#44),#25);\n\
#46=MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',(#45),#30);\n";

#[test]
fn reads_face_transparency() {
    let src = format!("{HEADER}{BREP}{RENDER}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let face = scene.all_faces().next().expect("a face");
    let t = face.transparency().expect("the face has a transparency");
    assert!((t - 0.3).abs() < 1e-9, "transparency = {t}");

    // The same rendering style also carries the surface colour.
    let c = face.color().expect("the face is coloured");
    assert!(
        (c.red - 0.5).abs() < 1e-9 && (c.green - 0.5).abs() < 1e-9 && (c.blue - 0.5).abs() < 1e-9,
        "colour = {c:?}"
    );

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );

    // An unstyled face has no transparency.
    let plain = format!("{HEADER}{BREP}{FOOTER}");
    let (m2, _rep) = read(plain.as_bytes()).expect("read");
    assert!(
        m2.scene()
            .all_faces()
            .next()
            .expect("a face")
            .transparency()
            .is_none(),
        "unstyled face should have no transparency"
    );
}

fn only_face_layer(src: &str) -> Option<String> {
    let (model, _rep) = read(src.as_bytes()).expect("read");
    model
        .scene()
        .all_faces()
        .next()
        .expect("a face")
        .layer()
        .map(str::to_owned)
}

#[test]
fn reads_layer() {
    // Common case: the layer contains the face's STYLED_ITEM (#38 from COLOUR).
    let via_si = format!(
        "{HEADER}{BREP}{COLOUR}#50=PRESENTATION_LAYER_ASSIGNMENT('Layer1','',(#38));\n{FOOTER}"
    );
    assert_eq!(only_face_layer(&via_si).as_deref(), Some("Layer1"));

    // The layer can also contain the geometry directly.
    let direct =
        format!("{HEADER}{BREP}#50=PRESENTATION_LAYER_ASSIGNMENT('L2','',(#25));\n{FOOTER}");
    assert_eq!(only_face_layer(&direct).as_deref(), Some("L2"));

    // No layer assignment → None.
    assert_eq!(only_face_layer(&format!("{HEADER}{BREP}{FOOTER}")), None);
}

fn only_face_visible(src: &str) -> bool {
    let (model, _rep) = read(src.as_bytes()).expect("read");
    model
        .scene()
        .all_faces()
        .next()
        .expect("a face")
        .is_visible()
}

#[test]
fn reads_visibility() {
    // The face's styled item is made invisible.
    let si_hidden = format!("{HEADER}{BREP}{COLOUR}#50=INVISIBILITY((#38));\n{FOOTER}");
    assert!(!only_face_visible(&si_hidden), "styled item hidden");

    // The whole layer (holding the face's styled item) is made invisible.
    let layer_hidden = format!(
        "{HEADER}{BREP}{COLOUR}#50=PRESENTATION_LAYER_ASSIGNMENT('L','',(#38));\n\
         #51=INVISIBILITY((#50));\n{FOOTER}"
    );
    assert!(!only_face_visible(&layer_hidden), "layer hidden");

    // Nothing hides it → visible by default.
    assert!(only_face_visible(&format!("{HEADER}{BREP}{FOOTER}")));
}

// A curve style (colour + width + line font) on the edge's underlying LINE #18 —
// curve styles apply to curve geometry (reached via `edge.curve()`), not the
// topological EDGE_CURVE.
const CURVE_STYLE: &str = "\
#30=REPRESENTATION_CONTEXT('','3D');\n\
#31=COLOUR_RGB('',0.1,0.2,0.3);\n\
#32=DRAUGHTING_PRE_DEFINED_CURVE_FONT('continuous');\n\
#33=CURVE_STYLE('cs',#32,POSITIVE_LENGTH_MEASURE(0.5),#31);\n\
#34=PRESENTATION_STYLE_ASSIGNMENT((#33));\n\
#35=STYLED_ITEM('',(#34),#18);\n\
#36=MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',(#35),#30);\n";

#[test]
fn reads_curve_style() {
    let src = format!("{HEADER}{BREP}{CURVE_STYLE}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let curve = scene.all_edges().next().expect("an edge").curve();
    let c = curve.color().expect("the curve is coloured");
    assert!(
        (c.red - 0.1).abs() < 1e-9 && (c.green - 0.2).abs() < 1e-9 && (c.blue - 0.3).abs() < 1e-9,
        "colour = {c:?}"
    );
    let w = curve.width().expect("the curve has a width");
    assert!((w - 0.5).abs() < 1e-9, "width = {w}");
    assert_eq!(curve.line_font(), Some("continuous"));

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );

    // An unstyled curve has no style.
    let plain = format!("{HEADER}{BREP}{FOOTER}");
    let (m2, _rep) = read(plain.as_bytes()).expect("read");
    let s2 = m2.scene();
    let c2 = s2.all_edges().next().expect("an edge").curve();
    assert!(c2.color().is_none() && c2.width().is_none() && c2.line_font().is_none());
}

#[test]
fn navigates_brep_solid_down_to_point_coords() {
    let src = format!("{HEADER}{BREP}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // Solid → Face (scoped navigation)
    let solids: Vec<_> = scene.all_solids().collect();
    assert_eq!(solids.len(), 1, "expected one solid");
    let faces: Vec<_> = solids[0].faces().collect();
    assert_eq!(faces.len(), 1, "expected one face");
    let face = faces[0];

    // Face → Surface (Plane)
    assert!(
        matches!(face.surface().kind(), SurfaceKind::Plane(_)),
        "face geometry should be a plane"
    );

    // Face → Bound → Edge
    let bounds: Vec<_> = face.bounds().collect();
    assert_eq!(bounds.len(), 1, "expected one bound");
    assert!(bounds[0].is_outer(), "the bound is a FACE_OUTER_BOUND");
    let edges: Vec<_> = bounds[0].edges().collect();
    assert_eq!(edges.len(), 1, "expected one edge");
    let edge = edges[0];

    // Edge → Curve (Line)
    assert!(
        matches!(edge.curve().kind(), CurveKind::Line(_)),
        "edge geometry should be a line"
    );

    // Edge → Vertex → Point → coordinates
    let start = edge
        .start()
        .expect("start vertex")
        .point()
        .expect("start point");
    assert_eq!(start.coords(), &[0.0, 0.0, 0.0]);
    let end = edge.end().expect("end vertex").point().expect("end point");
    assert_eq!(end.coords(), &[1.0, 0.0, 0.0]);
}

#[test]
fn reverse_navigation_via_refgraph() {
    let src = format!("{HEADER}{BREP}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // Face → Solid (reverse): a solid's face navigates back to that solid, whose
    // forward faces round-trip to the one face we started from. No `&rg` needed —
    // the scene builds the reverse index lazily on first use.
    let solid = scene.all_solids().next().expect("a solid");
    let face = solid.faces().next().expect("a face");
    let back = face.solid().expect("face belongs to a solid");
    assert_eq!(back.faces().count(), 1, "round-trips to the same solid");
    assert!(matches!(
        back.faces().next().expect("a face").surface().kind(),
        SurfaceKind::Plane(_)
    ));

    // Vertex → Edges (reverse): each of the two vertices is used by the one edge.
    let mut vertices = scene.all_vertices();
    for _ in 0..2 {
        let vertex = vertices.next().expect("a vertex");
        assert_eq!(vertex.edges().count(), 1, "each vertex is used by one edge");
    }
}

#[test]
fn key_is_a_stable_shared_identity() {
    let src = format!("{HEADER}{BREP}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // The same face reached two ways (scoped from the solid vs model-wide) has
    // the same key — that is what lets a consumer dedup shared entities.
    let scoped = scene
        .all_solids()
        .next()
        .expect("a solid")
        .faces()
        .next()
        .expect("a scoped face");
    let global = scene.all_faces().next().expect("a global face");
    assert_eq!(scoped.key(), global.key(), "same face → same key");

    // Distinct entities have distinct keys; an edge's two endpoints are both
    // model vertices (unify) and differ from each other (distinguish).
    let edge = scene.all_edges().next().expect("an edge");
    let start = edge.start().expect("start vertex");
    let end = edge.end().expect("end vertex");
    assert_ne!(start.key(), end.key(), "two endpoints → distinct keys");
    let vkeys: Vec<_> = scene.all_vertices().map(|v| v.key()).collect();
    assert!(vkeys.contains(&start.key()) && vkeys.contains(&end.key()));

    // Surface/Curve (reference-view handles) also carry a key, via the generated
    // `Ref::entity_key`. The plane reached from two faces is one key (stable);
    // the face's surface and the edge's curve are distinct entities.
    let surf = scoped.surface().key();
    assert_eq!(surf, global.surface().key(), "same surface → same key");
    assert_ne!(surf, edge.curve().key(), "surface vs curve → distinct keys");
}

#[test]
fn model_wide_all_queries_flatten_the_brep() {
    let src = format!("{HEADER}{BREP}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // `all_*` returns every instance of a type across the whole model.
    assert_eq!(scene.all_solids().count(), 1);
    assert_eq!(scene.all_faces().count(), 1);
    assert_eq!(scene.all_edges().count(), 1);
    assert_eq!(scene.all_vertices().count(), 2);
    assert!(
        scene.all_points().count() >= 2,
        "two distinct vertices' points"
    );

    // The model-wide face and the solid-scoped face are the same object.
    let global = scene.all_faces().next().expect("a face");
    let scoped = scene
        .all_solids()
        .next()
        .expect("a solid")
        .faces()
        .next()
        .expect("a scoped face");
    assert!(matches!(global.surface().kind(), SurfaceKind::Plane(_)));
    assert!(matches!(scoped.surface().kind(), SurfaceKind::Plane(_)));
}

// Orientation flags for wire assembly: the FACE_BOUND orientation, each
// ORIENTED_EDGE's per-edge direction, and the EDGE_CURVE same_sense.
const ORIENTED: &str = "\
#1=CARTESIAN_POINT('',(0.,0.,0.));\n\
#2=CARTESIAN_POINT('',(1.,0.,0.));\n\
#3=CARTESIAN_POINT('',(1.,1.,0.));\n\
#4=DIRECTION('',(0.,0.,1.));\n\
#5=DIRECTION('',(1.,0.,0.));\n\
#6=AXIS2_PLACEMENT_3D('',#1,#4,#5);\n\
#7=PLANE('',#6);\n\
#10=VERTEX_POINT('',#1);\n\
#11=VERTEX_POINT('',#2);\n\
#12=VERTEX_POINT('',#3);\n\
#13=VECTOR('',#5,1.0);\n\
#14=LINE('',#1,#13);\n\
#15=EDGE_CURVE('a',#10,#11,#14,.T.);\n\
#16=EDGE_CURVE('b',#11,#12,#14,.F.);\n\
#17=ORIENTED_EDGE('',*,*,#15,.T.);\n\
#18=ORIENTED_EDGE('',*,*,#16,.F.);\n\
#19=EDGE_LOOP('',(#17,#18));\n\
#20=FACE_BOUND('',#19,.F.);\n\
#21=ADVANCED_FACE('',(#20),#7,.T.);\n\
#22=CLOSED_SHELL('',(#21));\n\
#23=MANIFOLD_SOLID_BREP('',#22);\n";

#[test]
fn bounds_expose_the_orientation_layers() {
    let src = format!("{HEADER}{ORIENTED}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let solid = scene.all_solids().next().expect("solid");
    let face = solid.faces().next().expect("face");
    let bound = face.bounds().next().expect("bound");

    // The whole-loop flag (.F. here) and the outer/inner distinction.
    assert!(!bound.orientation());
    assert!(!bound.is_outer());

    // Per-edge orientation is preserved in loop order (edge identity via key).
    let oriented: Vec<_> = bound.oriented_edges().collect();
    assert_eq!(oriented.len(), 2);
    assert!(oriented[0].1, "first oriented edge is forward");
    assert!(!oriented[1].1, "second oriented edge is reversed");

    // edges() is the same walk without the flag.
    let plain: Vec<_> = bound.edges().map(|e| e.key()).collect();
    let from_oriented: Vec<_> = oriented.iter().map(|(e, _)| e.key()).collect();
    assert_eq!(plain, from_oriented);

    // The curve-parameter flag on the edge itself (.T. then .F. in the fixture).
    assert!(oriented[0].0.same_sense());
    assert!(!oriented[1].0.same_sense());

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}
