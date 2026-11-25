//! PMI dimensions API: a `DIMENSIONAL_SIZE` whose nominal value is reached
//! through `DIMENSIONAL_CHARACTERISTIC_REPRESENTATION` → a
//! `SHAPE_DIMENSION_REPRESENTATION`'s `MEASURE_REPRESENTATION_ITEM`. The DCR is a
//! root, so it anchors the whole chain through settling.

use step_io::model::EntityKey;
use step_io::read;
use step_io::scene::{CurveKind, SurfaceKind};
use step_io::scene::{DimensionKind, FeatureGeometry, FeatureKind, ToleranceKind};

// A shape aspect linked to a real planar ADVANCED_FACE via
// GEOMETRIC_ITEM_SPECIFIC_USAGE. A dimension targets the aspect (to obtain the
// Feature), and the GISU + dimension are roots, so settling keeps the brep.
const GEO: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('p','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#20=CARTESIAN_POINT('',(0.0,0.0,0.0));\n\
#21=CARTESIAN_POINT('',(1.0,0.0,0.0));\n\
#22=DIRECTION('',(0.0,0.0,1.0));\n\
#23=DIRECTION('',(1.0,0.0,0.0));\n\
#24=DIRECTION('',(1.0,0.0,0.0));\n\
#25=AXIS2_PLACEMENT_3D('',#20,#22,#23);\n\
#26=PLANE('',#25);\n\
#27=VECTOR('',#24,1.0);\n\
#28=LINE('',#20,#27);\n\
#29=VERTEX_POINT('',#20);\n\
#30=VERTEX_POINT('',#21);\n\
#31=EDGE_CURVE('',#29,#30,#28,.T.);\n\
#32=ORIENTED_EDGE('',*,*,#31,.T.);\n\
#33=EDGE_LOOP('',(#32));\n\
#34=FACE_OUTER_BOUND('',#33,.T.);\n\
#35=ADVANCED_FACE('',(#34),#26,.T.);\n\
#36=REPRESENTATION_CONTEXT('','3D');\n\
#37=SHAPE_REPRESENTATION('',(#35),#36);\n\
#40=SHAPE_ASPECT('hole','',#13,.T.);\n\
#41=DIMENSIONAL_SIZE(#40,'D1');\n\
#42=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#40,#37,#35);\n";

#[test]
fn feature_resolves_to_its_face() {
    let src = format!("{HEADER}{GEO}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // Reach the feature via the dimension that targets it, then walk to geometry.
    let dim = scene.all_dimensions().next().expect("a dimension");
    let feat: Vec<_> = dim.features().collect();
    assert_eq!(feat.len(), 1, "size targets one feature");
    assert_eq!(feat[0].name(), "hole");

    let geo: Vec<_> = feat[0].geometry().collect();
    assert_eq!(geo.len(), 1, "feature designates one item");
    match &geo[0] {
        FeatureGeometry::Face(f) => {
            assert!(
                matches!(f.surface().kind(), SurfaceKind::Plane(_)),
                "the identified face is a plane"
            );
        }
        other => panic!("expected a face, got {:?}", discriminant(other)),
    }

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

#[test]
fn feature_resolves_face_surfaces_and_solids_with_voids() {
    // The same feature, its face a FACE_SURFACE, plus a second GISU naming a
    // BREP_WITH_VOIDS.
    let geo = GEO
        .replace("#35=ADVANCED_FACE(", "#35=FACE_SURFACE(")
        .replace(
            "SHAPE_REPRESENTATION('',(#35)",
            "SHAPE_REPRESENTATION('',(#35,#45)",
        );
    let src = format!(
        "{HEADER}{geo}\
#43=CLOSED_SHELL('',(#35));\n\
#44=ORIENTED_CLOSED_SHELL('',*,#43,.F.);\n\
#45=BREP_WITH_VOIDS('',#43,(#44));\n\
#46=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#40,#37,#45);\n\
{FOOTER}"
    );
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let feat: Vec<_> = scene
        .all_dimensions()
        .next()
        .expect("a dimension")
        .features()
        .collect();
    let kinds: Vec<_> = feat[0].geometry().map(|g| discriminant(&g)).collect();
    assert_eq!(kinds, ["Face", "Solid"]);
}

// A label for the FeatureGeometry variant, for test failure messages.
fn discriminant(g: &FeatureGeometry) -> &'static str {
    match g {
        FeatureGeometry::Face(_) => "Face",
        FeatureGeometry::Edge(_) => "Edge",
        FeatureGeometry::Point(_) => "Point",
        FeatureGeometry::Solid(_) => "Solid",
        FeatureGeometry::Curve(_) => "Curve",
        FeatureGeometry::Other(_) => "Other",
        _ => "unknown",
    }
}

// A shape aspect whose GISU identified_item is a curve (a CIRCLE) rather than a
// face — exercises FeatureGeometry::Curve.
const GEO_CURVE: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('p','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#20=CARTESIAN_POINT('',(0.0,0.0,0.0));\n\
#21=DIRECTION('',(0.0,0.0,1.0));\n\
#22=DIRECTION('',(1.0,0.0,0.0));\n\
#23=AXIS2_PLACEMENT_3D('',#20,#21,#22);\n\
#24=CIRCLE('',#23,5.0);\n\
#25=REPRESENTATION_CONTEXT('','3D');\n\
#26=SHAPE_REPRESENTATION('',(#24),#25);\n\
#40=SHAPE_ASPECT('rim','',#13,.T.);\n\
#41=DIMENSIONAL_SIZE(#40,'D1');\n\
#42=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#40,#26,#24);\n";

// Two shape aspects (hole/slot) plus a DATUM. scene.all_features() lists the two
// shape aspects but NOT the datum (which is a datum callout, reached via
// scene.all_datums()); the hole is linked to a real face via GISU.
const FEATURES: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('p','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#20=CARTESIAN_POINT('',(0.0,0.0,0.0));\n\
#21=CARTESIAN_POINT('',(1.0,0.0,0.0));\n\
#22=DIRECTION('',(0.0,0.0,1.0));\n\
#23=DIRECTION('',(1.0,0.0,0.0));\n\
#24=DIRECTION('',(1.0,0.0,0.0));\n\
#25=AXIS2_PLACEMENT_3D('',#20,#22,#23);\n\
#26=PLANE('',#25);\n\
#27=VECTOR('',#24,1.0);\n\
#28=LINE('',#20,#27);\n\
#29=VERTEX_POINT('',#20);\n\
#30=VERTEX_POINT('',#21);\n\
#31=EDGE_CURVE('',#29,#30,#28,.T.);\n\
#32=ORIENTED_EDGE('',*,*,#31,.T.);\n\
#33=EDGE_LOOP('',(#32));\n\
#34=FACE_OUTER_BOUND('',#33,.T.);\n\
#35=ADVANCED_FACE('',(#34),#26,.T.);\n\
#36=REPRESENTATION_CONTEXT('','3D');\n\
#37=SHAPE_REPRESENTATION('',(#35),#36);\n\
#40=SHAPE_ASPECT('hole','',#13,.T.);\n\
#43=SHAPE_ASPECT('slot','',#13,.T.);\n\
#44=DATUM('datum a','',#13,.T.,'A');\n\
#42=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#40,#37,#35);\n";

#[test]
fn scene_enumerates_features_excluding_datums() {
    let src = format!("{HEADER}{FEATURES}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // features() lists the two shape aspects; the DATUM is excluded (narrow).
    let mut names: Vec<_> = scene.all_features().map(|f| f.name().to_owned()).collect();
    names.sort();
    assert_eq!(names, vec!["hole".to_owned(), "slot".to_owned()]);

    // The datum is not a feature but IS a datum.
    assert_eq!(scene.all_datums().count(), 1);
    assert_eq!(scene.all_datums().next().expect("a datum").letter(), "A");

    // A scene-enumerated feature walks straight to its geometry, and kind()
    // names its backing entity without matching on EntityKey.
    let hole = scene
        .all_features()
        .find(|f| f.name() == "hole")
        .expect("the hole feature");
    assert_eq!(hole.kind(), FeatureKind::ShapeAspect);
    match &hole.geometry().next().expect("a geometry item") {
        FeatureGeometry::Face(f) => {
            assert!(matches!(f.surface().kind(), SurfaceKind::Plane(_)));
        }
        other => panic!("expected a face, got {}", discriminant(other)),
    }

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

// A DATUM ('A') established from a DATUM_FEATURE via a SHAPE_ASPECT_RELATIONSHIP;
// the datum feature is linked to a real face by GISU. datum.datum_feature() must
// cross the relationship and reach that face.
const DATUM_FEATURE: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('p','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#20=CARTESIAN_POINT('',(0.0,0.0,0.0));\n\
#21=CARTESIAN_POINT('',(1.0,0.0,0.0));\n\
#22=DIRECTION('',(0.0,0.0,1.0));\n\
#23=DIRECTION('',(1.0,0.0,0.0));\n\
#24=DIRECTION('',(1.0,0.0,0.0));\n\
#25=AXIS2_PLACEMENT_3D('',#20,#22,#23);\n\
#26=PLANE('',#25);\n\
#27=VECTOR('',#24,1.0);\n\
#28=LINE('',#20,#27);\n\
#29=VERTEX_POINT('',#20);\n\
#30=VERTEX_POINT('',#21);\n\
#31=EDGE_CURVE('',#29,#30,#28,.T.);\n\
#32=ORIENTED_EDGE('',*,*,#31,.T.);\n\
#33=EDGE_LOOP('',(#32));\n\
#34=FACE_OUTER_BOUND('',#33,.T.);\n\
#35=ADVANCED_FACE('',(#34),#26,.T.);\n\
#37=SHAPE_REPRESENTATION('',(#35),#36);\n\
#36=REPRESENTATION_CONTEXT('','3D');\n\
#50=DATUM_FEATURE('df','',#13,.T.);\n\
#51=DATUM('datum a','',#13,.T.,'A');\n\
#52=SHAPE_ASPECT_RELATIONSHIP('','',#50,#51);\n\
#53=GEOMETRIC_ITEM_SPECIFIC_USAGE('','datum feature',#50,#37,#35);\n";

#[test]
fn datum_reaches_its_datum_feature_geometry() {
    let src = format!("{HEADER}{DATUM_FEATURE}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let datum = scene.all_datums().next().expect("a datum");
    assert_eq!(datum.letter(), "A");

    // Cross DATUM -> SHAPE_ASPECT_RELATIONSHIP -> DATUM_FEATURE.
    let df = datum.datum_feature().expect("datum feature");
    assert_eq!(df.kind(), FeatureKind::DatumFeature);

    // ...and on to the face the datum feature is established on.
    match &df.geometry().next().expect("a geometry item") {
        FeatureGeometry::Face(f) => {
            assert!(matches!(f.surface().kind(), SurfaceKind::Plane(_)));
        }
        other => panic!("expected a face, got {}", discriminant(other)),
    }
}

/// A `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE` is a datum feature too: the datum
/// reaches it.
#[test]
fn a_datum_reaches_a_dimensional_size_with_datum_feature() {
    let datum_feature = DATUM_FEATURE.replace(
        "#50=DATUM_FEATURE('df','',#13,.T.);",
        "#50=DIMENSIONAL_SIZE_WITH_DATUM_FEATURE('df','',#13,.T.,#50,'size');",
    );
    let src = format!("{HEADER}{datum_feature}{FOOTER}");
    let (model, rep) = read(src.as_bytes()).expect("read");
    assert!(rep.dropped.is_empty(), "dropped: {:?}", rep.dropped);
    let scene = model.scene();
    let datum = scene.all_datums().next().expect("a datum");
    let df = datum.datum_feature().expect("the datum feature");
    assert!(df.geometry().next().is_some(), "reaches its face");
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}

// A composite datum feature written as a complex instance, as NIST's AP242
// test files write one: the complex is the datum feature of datum 'A', joins
// two plain shape aspects (on a face and an edge) by
// SHAPE_ASPECT_RELATIONSHIPs, and carries a dimension and a tolerance. It has
// no GISU of its own; the datum has one, on a point.
fn complex_datum_feature() -> String {
    DATUM_FEATURE
        .replace(
            "#50=DATUM_FEATURE('df','',#13,.T.);",
            "#50=(COMPOSITE_SHAPE_ASPECT()DATUM_FEATURE()SHAPE_ASPECT('DF A','',#13,.T.));",
        )
        .replace(
            "#53=GEOMETRIC_ITEM_SPECIFIC_USAGE('','datum feature',#50,#37,#35);",
            "#54=SHAPE_ASPECT('side 1','',#13,.T.);\n\
#55=SHAPE_ASPECT('side 2','',#13,.T.);\n\
#56=SHAPE_ASPECT_RELATIONSHIP('','',#50,#54);\n\
#57=SHAPE_ASPECT_RELATIONSHIP('','',#50,#55);\n\
#58=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#54,#37,#35);\n\
#59=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#55,#37,#31);\n\
#60=DIMENSIONAL_SIZE(#50,'width');\n\
#61=FLATNESS_TOLERANCE('flat',$,$,#50);\n\
#62=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#51,#37,#21);",
        )
}

/// A shape aspect written as a complex instance with a `SHAPE_ASPECT` part is
/// a feature: the datum, the dimension and the tolerance reach it, and it
/// belongs to its part.
#[test]
fn a_complex_datum_feature_is_a_feature() {
    let src = format!("{HEADER}{}{FOOTER}", complex_datum_feature());
    let (model, rep) = read(src.as_bytes()).expect("read");
    assert!(rep.dropped.is_empty(), "dropped: {:?}", rep.dropped);
    let scene = model.scene();

    let datum = scene.all_datums().next().expect("a datum");
    let df = datum.datum_feature().expect("the datum feature");
    assert_eq!(df.kind(), FeatureKind::Complex);
    assert_eq!(df.name(), "DF A");
    assert_eq!(df.description(), Some(""));
    assert!(matches!(df.key(), EntityKey::ComplexEntity(_)));

    assert!(scene.all_features().any(|f| f.key() == df.key()));
    let part = scene
        .all_product_definitions()
        .find(|d| d.product().is_some_and(|p| p.name() == "Part"))
        .expect("the part");
    assert!(part.features().any(|f| f.key() == df.key()));

    let dim = scene.all_dimensions().next().expect("a dimension");
    assert!(dim.features().any(|f| f.key() == df.key()));
    let tol = scene.all_tolerances().next().expect("a tolerance");
    assert_eq!(tol.feature().map(|f| f.key()), Some(df.key()));
    assert_eq!(part.dimensions().count(), 1);
    assert_eq!(part.tolerances().count(), 1);

    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}

/// A complex datum feature's geometry is its components': the face and the
/// edge, not the point on the datum it establishes.
#[test]
fn a_complex_datum_feature_gives_its_components_geometry() {
    let src = format!("{HEADER}{}{FOOTER}", complex_datum_feature());
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();
    let df = scene
        .all_datums()
        .next()
        .and_then(|d| d.datum_feature())
        .expect("the datum feature");
    let kinds: Vec<_> = df.geometry().map(|g| discriminant(&g)).collect();
    assert_eq!(kinds, ["Face", "Edge"]);
}

// A plain DATUM_FEATURE with no GISU, as NIST's AP242 test files write some:
// its geometry is behind SHAPE_ASPECT_RELATIONSHIPs, on a plain aspect (on
// the point #20) and on a group whose components are a plain aspect (on the
// face, designated twice) and a nested composite (on the edge). An
// all-around aspect wraps the group. A dimensional location from the datum
// feature to another aspect (on the point #21) is not a component.
const DATUM_FEATURE_COMPONENTS: &str = "\
#50=DATUM_FEATURE('df','',#13,.T.);\n\
#51=DATUM('datum a','',#13,.T.,'A');\n\
#52=SHAPE_ASPECT_RELATIONSHIP('','',#50,#51);\n\
#70=COMPOSITE_GROUP_SHAPE_ASPECT('grp','',#13,.T.);\n\
#71=SHAPE_ASPECT_RELATIONSHIP('','datum feature',#50,#70);\n\
#72=SHAPE_ASPECT('a','',#13,.T.);\n\
#73=SHAPE_ASPECT_RELATIONSHIP('','',#70,#72);\n\
#74=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#72,#37,#35);\n\
#75=COMPOSITE_SHAPE_ASPECT('c','',#13,.T.);\n\
#76=SHAPE_ASPECT_RELATIONSHIP('','',#70,#75);\n\
#77=SHAPE_ASPECT('b','',#13,.T.);\n\
#78=SHAPE_ASPECT_RELATIONSHIP('','',#75,#77);\n\
#79=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#77,#37,#31);\n\
#80=SHAPE_ASPECT('other','',#13,.T.);\n\
#81=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#80,#37,#21);\n\
#82=DIMENSIONAL_LOCATION('loc','',#50,#80);\n\
#83=ALL_AROUND_SHAPE_ASPECT('around','',#13,.T.);\n\
#84=SHAPE_ASPECT_RELATIONSHIP('','',#83,#70);\n\
#85=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#72,#37,#35);\n\
#86=SHAPE_ASPECT('d','',#13,.T.);\n\
#87=SHAPE_ASPECT_RELATIONSHIP('','datum feature',#50,#86);\n\
#88=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#86,#37,#20);\n";

/// Composite features and datum features give the geometry of their
/// components, nested ones included, each item once; a relationship subtype
/// and the parent side of a relationship are not followed.
#[test]
fn composite_and_datum_features_give_their_components_geometry() {
    let base = DATUM_FEATURE
        .split("#50=")
        .next()
        .expect("the part and its geometry");
    let src = format!("{HEADER}{base}{DATUM_FEATURE_COMPONENTS}{FOOTER}");
    let (model, rep) = read(src.as_bytes()).expect("read");
    assert!(rep.dropped.is_empty(), "dropped: {:?}", rep.dropped);
    let scene = model.scene();
    let kinds = |name: &str| -> Vec<&'static str> {
        let f = scene
            .all_features()
            .find(|f| f.name() == name)
            .expect("the feature");
        let mut kinds: Vec<_> = f.geometry().map(|g| discriminant(&g)).collect();
        kinds.sort_unstable();
        kinds
    };
    assert_eq!(kinds("df"), ["Edge", "Face", "Point"]);
    assert_eq!(kinds("grp"), ["Edge", "Face"]);
    assert_eq!(kinds("c"), ["Edge"]);
    assert_eq!(kinds("around"), ["Edge", "Face"]);
    let df = scene
        .all_datums()
        .next()
        .and_then(|d| d.datum_feature())
        .expect("the datum feature");
    assert_eq!(df.geometry().count(), 3);
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}

// Features linked to their geometry by a plain ITEM_IDENTIFIED_REPRESENTATION_USAGE
// naming a set of items, as some of NIST's AP242 test files write them: a
// datum feature on the face and the edge, and a group whose component aspect
// is on the face. The aspect also has a GISU on the face, in a second
// representation that holds the face too.
const IIRU_FEATURES: &str = "\
#38=SHAPE_REPRESENTATION('',(#35),#36);\n\
#50=DATUM_FEATURE('df','',#13,.T.);\n\
#51=DATUM('datum a','',#13,.T.,'A');\n\
#52=SHAPE_ASPECT_RELATIONSHIP('','',#50,#51);\n\
#53=ITEM_IDENTIFIED_REPRESENTATION_USAGE('','',#50,#37,SET_REPRESENTATION_ITEM((#35,#31)));\n\
#70=COMPOSITE_GROUP_SHAPE_ASPECT('grp','',#13,.T.);\n\
#72=SHAPE_ASPECT('a','',#13,.T.);\n\
#73=SHAPE_ASPECT_RELATIONSHIP('','',#70,#72);\n\
#74=ITEM_IDENTIFIED_REPRESENTATION_USAGE('','',#72,#37,SET_REPRESENTATION_ITEM((#35)));\n\
#75=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#72,#38,#35);\n";

/// A plain `ITEM_IDENTIFIED_REPRESENTATION_USAGE` gives a feature each item of
/// the set it names; an item it shares with a GISU comes once.
#[test]
fn a_feature_gives_the_items_an_item_identified_representation_usage_names() {
    let base = DATUM_FEATURE
        .split("#50=")
        .next()
        .expect("the part and its geometry");
    let src = format!("{HEADER}{base}{IIRU_FEATURES}{FOOTER}");
    let (model, rep) = read(src.as_bytes()).expect("read");
    assert!(rep.dropped.is_empty(), "dropped: {:?}", rep.dropped);
    let scene = model.scene();
    let df = scene
        .all_datums()
        .next()
        .and_then(|d| d.datum_feature())
        .expect("the datum feature");
    let kinds: Vec<_> = df.geometry().map(|g| discriminant(&g)).collect();
    assert_eq!(kinds, ["Face", "Edge"]);
    for name in ["grp", "a"] {
        let f = scene
            .all_features()
            .find(|f| f.name() == name)
            .expect("the feature");
        let kinds: Vec<_> = f.geometry().map(|g| discriminant(&g)).collect();
        assert_eq!(kinds, ["Face"], "{name}");
    }
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}

// Two parts (A, B), each with its own shape aspect + dimension; part A also has
// a feature tolerance, a whole-part (PDS-targeted) tolerance, and a datum.
// Per-part queries must return only that part's PMI.
const TWO_PARTS: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('pa','partA','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#14=SHAPE_ASPECT('holeA','',#13,.T.);\n\
#15=DIMENSIONAL_SIZE(#14,'DA');\n\
#16=FLATNESS_TOLERANCE('flatA',$,$,#14);\n\
#17=SURFACE_PROFILE_TOLERANCE('profA',$,$,#13);\n\
#18=DATUM('datum a','',#13,.T.,'A');\n\
#20=PRODUCT('pb','partB','',(#2));\n\
#21=PRODUCT_DEFINITION_FORMATION('1','',#20);\n\
#22=PRODUCT_DEFINITION('design','',#21,#3);\n\
#23=PRODUCT_DEFINITION_SHAPE('','',#22);\n\
#24=SHAPE_ASPECT('holeB','',#23,.T.);\n\
#25=DIMENSIONAL_SIZE(#24,'DB');\n";

#[test]
fn per_part_pmi_is_isolated() {
    let src = format!("{HEADER}{TWO_PARTS}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let part = |name: &str| {
        scene
            .all_product_definitions()
            .find(|pd| pd.product().is_some_and(|p| p.name() == name))
            .unwrap_or_else(|| panic!("part {name}"))
    };
    let a = part("partA");
    let b = part("partB");

    // Part A: only its own feature / dimension / datum, plus two tolerances (one
    // on the feature, one on the whole part).
    let a_features: Vec<_> = a.features().map(|f| f.name().to_owned()).collect();
    assert_eq!(a_features, vec!["holeA".to_owned()]);
    assert_eq!(a.dimensions().count(), 1);
    assert_eq!(a.tolerances().count(), 2, "feature + whole-part tolerance");
    let a_datums: Vec<_> = a.datums().map(|d| d.letter().to_owned()).collect();
    assert_eq!(a_datums, vec!["A".to_owned()]);

    // Part B: only its own feature / dimension; no tolerances or datums.
    let b_features: Vec<_> = b.features().map(|f| f.name().to_owned()).collect();
    assert_eq!(b_features, vec!["holeB".to_owned()]);
    assert_eq!(b.dimensions().count(), 1);
    assert_eq!(b.tolerances().count(), 0);
    assert_eq!(b.datums().count(), 0);

    // Cross-isolation: A's queries never surface B's entities.
    assert!(a.features().all(|f| f.name() != "holeB"));

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

#[test]
fn feature_resolves_to_its_curve() {
    let src = format!("{HEADER}{GEO_CURVE}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let dim = scene.all_dimensions().next().expect("a dimension");
    let feat: Vec<_> = dim.features().collect();
    assert_eq!(feat[0].name(), "rim");

    let geo: Vec<_> = feat[0].geometry().collect();
    assert_eq!(geo.len(), 1, "feature designates one item");
    match &geo[0] {
        FeatureGeometry::Curve(c) => {
            assert!(
                matches!(c.kind(), CurveKind::Circle(_)),
                "the identified curve is a circle"
            );
        }
        other => panic!("expected a curve, got {}", discriminant(other)),
    }

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

// A datum reference frame reached two ways: a complex position tolerance whose
// GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE facet points at a DATUM_SYSTEM (→
// compartment → datum A), and a standalone perpendicularity tolerance whose
// datum_system field holds a DATUM_REFERENCE (→ datum B). Both tolerances are
// roots, so settling keeps the whole datum chain.
const DAT: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('p','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#14=SHAPE_ASPECT('feature','',#13,.T.);\n\
#22=( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
#30=DATUM('datum a','',#13,.T.,'A');\n\
#31=DATUM('datum b','',#13,.T.,'B');\n\
#32=DATUM_REFERENCE_COMPARTMENT('','',#13,.T.,#30,$);\n\
#33=DATUM_SYSTEM('','',#13,.T.,(#32));\n\
#34=DATUM_REFERENCE(1,#31);\n\
#40=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.1),#22);\n\
#41=( GEOMETRIC_TOLERANCE('pos',$,#40,#14) GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE((#33)) POSITION_TOLERANCE() );\n\
#42=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.05),#22);\n\
#43=PERPENDICULARITY_TOLERANCE('perp',$,#42,#14,(#34));\n";

#[test]
fn reads_datums_and_tolerance_reference_frame() {
    let src = format!("{HEADER}{DAT}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // scene.all_datums(): both DATUMs, by letter.
    let mut letters: Vec<_> = scene.all_datums().map(|d| d.letter().to_owned()).collect();
    letters.sort();
    assert_eq!(letters, vec!["A".to_owned(), "B".to_owned()]);

    // Complex position tolerance → DATUM_SYSTEM → compartment → datum A.
    let tols: Vec<_> = scene.all_tolerances().collect();
    let pos = tols
        .iter()
        .find(|t| t.kind() == ToleranceKind::Position)
        .expect("a position tolerance");
    let pos_datums: Vec<_> = pos.datums().collect();
    assert_eq!(pos_datums.len(), 1, "position references one datum");
    assert_eq!(pos_datums[0].letter(), "A");

    // Standalone perpendicularity tolerance → DATUM_REFERENCE → datum B.
    let perp = tols
        .iter()
        .find(|t| t.kind() == ToleranceKind::Perpendicularity)
        .expect("a perpendicularity tolerance");
    let perp_datums: Vec<_> = perp.datums().collect();
    assert_eq!(
        perp_datums.len(),
        1,
        "perpendicularity references one datum"
    );
    assert_eq!(perp_datums[0].letter(), "B");

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

// Feature linkage: two named SHAPE_ASPECTs (hole/slot) that dimensions and
// tolerances point at. A size targets one feature, a location spans two; a
// dedicated and a complex tolerance each target one. Every PMI entity is a root,
// so settling keeps the whole set.
const FEAT: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('p','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#14=SHAPE_ASPECT('hole','',#13,.T.);\n\
#15=SHAPE_ASPECT('slot','',#13,.T.);\n\
#20=DIMENSIONAL_SIZE(#14,'D1');\n\
#21=DIMENSIONAL_LOCATION('L1','',#14,#15);\n\
#30=FLATNESS_TOLERANCE('flat',$,$,#14);\n\
#31=( GEOMETRIC_TOLERANCE('pos',$,$,#15) GEOMETRIC_TOLERANCE_WITH_MODIFIERS((.MAXIMUM_MATERIAL_REQUIREMENT.)) POSITION_TOLERANCE() );\n";

#[test]
fn links_dimensions_and_tolerances_to_features() {
    let src = format!("{HEADER}{FEAT}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // A size dimension targets one feature; a location spans two (relating,
    // related order).
    let dims: Vec<_> = scene.all_dimensions().collect();
    let size = dims
        .iter()
        .find(|d| d.kind() == DimensionKind::Size)
        .expect("a size");
    let size_features: Vec<_> = size.features().collect();
    assert_eq!(size_features.len(), 1);
    assert_eq!(size_features[0].name(), "hole");

    let loc = dims
        .iter()
        .find(|d| d.kind() == DimensionKind::Location)
        .expect("a location");
    let loc_features: Vec<_> = loc.features().map(|f| f.name()).collect();
    assert_eq!(loc_features, vec!["hole", "slot"]);

    // A dedicated tolerance and a complex tolerance each resolve their target.
    let tols: Vec<_> = scene.all_tolerances().collect();
    let flat = tols
        .iter()
        .find(|t| t.kind() == ToleranceKind::Flatness)
        .expect("a flatness tolerance");
    assert_eq!(flat.feature().expect("flatness feature").name(), "hole");

    let pos = tols
        .iter()
        .find(|t| t.kind() == ToleranceKind::Position)
        .expect("a position tolerance");
    assert_eq!(pos.feature().expect("position feature").name(), "slot");

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

const DIM: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('p','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#14=SHAPE_ASPECT('feature','',#13,.T.);\n\
#20=DIMENSIONAL_SIZE(#14,'D1');\n\
#21=REPRESENTATION_CONTEXT('','');\n\
#22=( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
#23=MEASURE_REPRESENTATION_ITEM('',LENGTH_MEASURE(10.),#22);\n\
#24=SHAPE_DIMENSION_REPRESENTATION('',(#23),#21);\n\
#25=DIMENSIONAL_CHARACTERISTIC_REPRESENTATION(#20,#24);\n";

#[test]
fn reads_dimension_value() {
    let src = format!("{HEADER}{DIM}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let dims: Vec<_> = scene.all_dimensions().collect();
    assert_eq!(dims.len(), 1, "one dimension");
    let d = dims[0];
    assert_eq!(d.kind(), DimensionKind::Size);
    assert_eq!(d.name(), "D1");

    let v = d.value().expect("the dimension has a value");
    assert!((v - 10.0).abs() < 1e-9, "value = {v}");

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

// The value and the magnitude as NIST's test files write them: a complex
// measure whose MEASURE_WITH_UNIT part holds the number.
const COMPLEX_MEASURES: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('p','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#14=SHAPE_ASPECT('feature','',#13,.T.);\n\
#20=DIMENSIONAL_SIZE(#14,'D1');\n\
#21=REPRESENTATION_CONTEXT('','');\n\
#22=( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
#23=( LENGTH_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM() MEASURE_WITH_UNIT(POSITIVE_LENGTH_MEASURE(5.),#22) QUALIFIED_REPRESENTATION_ITEM((#26)) REPRESENTATION_ITEM('nominal value') );\n\
#24=SHAPE_DIMENSION_REPRESENTATION('',(#23),#21);\n\
#25=DIMENSIONAL_CHARACTERISTIC_REPRESENTATION(#20,#24);\n\
#26=VALUE_FORMAT_TYPE_QUALIFIER('NR2 1.3');\n\
#30=( LENGTH_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM() MEASURE_WITH_UNIT(LENGTH_MEASURE(0.25),#22) REPRESENTATION_ITEM('magnitude') );\n\
#31=FLATNESS_TOLERANCE('flat','',#30,#14);\n";

#[test]
fn reads_a_complex_measure_as_a_dimension_value_and_a_tolerance_magnitude() {
    let src = format!("{HEADER}{COMPLEX_MEASURES}{FOOTER}");
    let (model, rep) = read(src.as_bytes()).expect("read");
    assert!(rep.dropped.is_empty(), "dropped: {:?}", rep.dropped);
    let scene = model.scene();

    let dim = scene.all_dimensions().next().expect("a dimension");
    let v = dim.value().expect("the complex measure item is the value");
    assert!((v - 5.0).abs() < 1e-9, "value = {v}");

    let tol = scene.all_tolerances().next().expect("a tolerance");
    let m = tol
        .magnitude()
        .expect("the complex measure is the magnitude");
    assert!((m - 0.25).abs() < 1e-9, "magnitude = {m}");

    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}

// A toleranced dimension as NIST writes it: the representation lists the lower
// limit, the nominal value and the upper limit, each a complex measure item.
const LIMITS_SCAFFOLD: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('p','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#14=SHAPE_ASPECT('feature','',#13,.T.);\n\
#20=DIMENSIONAL_SIZE(#14,'radius');\n\
#21=REPRESENTATION_CONTEXT('','');\n\
#22=( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
#25=DIMENSIONAL_CHARACTERISTIC_REPRESENTATION(#20,#24);\n";

fn dimension_value(items: &str) -> Option<f64> {
    let src = format!("{HEADER}{LIMITS_SCAFFOLD}{items}{FOOTER}");
    let (model, rep) = read(src.as_bytes()).expect("read");
    assert!(rep.dropped.is_empty(), "dropped: {:?}", rep.dropped);
    let scene = model.scene();
    let v = scene.all_dimensions().next().expect("a dimension").value();
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
    v
}

#[test]
fn a_dimension_value_is_not_one_of_its_limits() {
    let complex = dimension_value(
        "#30=( LENGTH_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM() MEASURE_WITH_UNIT(POSITIVE_LENGTH_MEASURE(0.012),#22) REPRESENTATION_ITEM('lower limit') );\n\
         #31=( LENGTH_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM() MEASURE_WITH_UNIT(POSITIVE_LENGTH_MEASURE(0.017),#22) REPRESENTATION_ITEM('nominal value') );\n\
         #32=( LENGTH_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM() MEASURE_WITH_UNIT(POSITIVE_LENGTH_MEASURE(0.022),#22) REPRESENTATION_ITEM('upper limit') );\n\
         #24=SHAPE_DIMENSION_REPRESENTATION('',(#30,#31,#32),#21);\n",
    );
    assert_eq!(complex, Some(0.017));

    let plain = dimension_value(
        "#30=MEASURE_REPRESENTATION_ITEM('upper limit',LENGTH_MEASURE(10.1),#22);\n\
         #31=MEASURE_REPRESENTATION_ITEM('lower limit',LENGTH_MEASURE(9.9),#22);\n\
         #32=MEASURE_REPRESENTATION_ITEM('',LENGTH_MEASURE(10.),#22);\n\
         #24=SHAPE_DIMENSION_REPRESENTATION('',(#30,#31,#32),#21);\n",
    );
    assert_eq!(plain, Some(10.0));
}

#[test]
fn a_dimension_with_only_limits_has_no_value() {
    let v = dimension_value(
        "#30=MEASURE_REPRESENTATION_ITEM('upper limit',LENGTH_MEASURE(10.1),#22);\n\
         #31=( LENGTH_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM() REPRESENTATION_ITEM('lower limit') );\n\
         #24=SHAPE_DIMENSION_REPRESENTATION('',(#30,#31),#21);\n",
    );
    assert_eq!(v, None);
}

// Three geometric tolerances sharing the product/aspect/unit scaffold: a
// standalone FLATNESS_TOLERANCE (dedicated arena), a complex position tolerance
// (multi-supertype ComplexEntity), and a magnitude-less STRAIGHTNESS_TOLERANCE.
// Each tolerance is a root (nothing references it), so settling keeps it.
const TOL: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('p','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#14=SHAPE_ASPECT('feature','',#13,.T.);\n\
#22=( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
#40=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.05),#22);\n\
#41=FLATNESS_TOLERANCE('flat',$,#40,#14);\n\
#42=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.1),#22);\n\
#43=( GEOMETRIC_TOLERANCE('pos',$,#42,#14) GEOMETRIC_TOLERANCE_WITH_MODIFIERS((.MAXIMUM_MATERIAL_REQUIREMENT.)) POSITION_TOLERANCE() );\n\
#44=STRAIGHTNESS_TOLERANCE('str',$,$,#14);\n";

#[test]
fn reads_geometric_tolerances() {
    let src = format!("{HEADER}{TOL}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let tols: Vec<_> = scene.all_tolerances().collect();
    assert_eq!(tols.len(), 3, "three tolerances");

    // Standalone leaf tolerance in a dedicated arena.
    let flat = tols
        .iter()
        .find(|t| t.kind() == ToleranceKind::Flatness)
        .expect("a flatness tolerance");
    assert_eq!(flat.name(), "flat");
    let m = flat.magnitude().expect("flatness has a magnitude");
    assert!((m - 0.05).abs() < 1e-9, "flatness magnitude = {m}");

    // Complex multi-supertype tolerance: kind from the POSITION_TOLERANCE facet,
    // magnitude/name from the GEOMETRIC_TOLERANCE facet.
    let pos = tols
        .iter()
        .find(|t| t.kind() == ToleranceKind::Position)
        .expect("a position tolerance");
    assert_eq!(pos.name(), "pos");
    let m = pos.magnitude().expect("position has a magnitude");
    assert!((m - 0.1).abs() < 1e-9, "position magnitude = {m}");

    // A tolerance with no magnitude yields None.
    let straight = tols
        .iter()
        .find(|t| t.kind() == ToleranceKind::Straightness)
        .expect("a straightness tolerance");
    assert_eq!(straight.name(), "str");
    assert!(
        straight.magnitude().is_none(),
        "magnitude-less tolerance should be None"
    );

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

const AP214_HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }'));\n\
ENDSEC;\nDATA;\n";

// AP214 types a tolerance magnitude as any MEASURE_WITH_UNIT: length values
// in two, a plane angle in the other.
const TOL_AP214: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('p','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#14=SHAPE_ASPECT('feature','',#13,.T.);\n\
#22=( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );\n\
#23=( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );\n\
#40=MEASURE_WITH_UNIT(LENGTH_MEASURE(0.05),#22);\n\
#41=FLATNESS_TOLERANCE('flat','',#40,#14);\n\
#42=MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.1),#23);\n\
#43=STRAIGHTNESS_TOLERANCE('str','',#42,#14);\n\
#44=MEASURE_WITH_UNIT(NON_NEGATIVE_LENGTH_MEASURE(0.02),#22);\n\
#45=ROUNDNESS_TOLERANCE('round','',#44,#14);\n";

#[test]
fn reads_an_ap214_plain_measure_magnitude_only_as_a_length() {
    let src = format!("{AP214_HEADER}{TOL_AP214}{FOOTER}");
    let (model, rep) = read(src.as_bytes()).expect("read");
    assert!(rep.dropped.is_empty(), "dropped: {:?}", rep.dropped);
    let scene = model.scene();
    let tols: Vec<_> = scene.all_tolerances().collect();
    assert_eq!(tols.len(), 3, "every tolerance kept");

    let flat = tols
        .iter()
        .find(|t| t.kind() == ToleranceKind::Flatness)
        .expect("a flatness tolerance");
    let m = flat.magnitude().expect("a length magnitude");
    assert!((m - 0.05).abs() < 1e-9, "flatness magnitude = {m}");
    let round = tols
        .iter()
        .find(|t| t.kind() == ToleranceKind::Roundness)
        .expect("a roundness tolerance");
    let m = round.magnitude().expect("a non-negative length magnitude");
    assert!((m - 0.02).abs() < 1e-9, "roundness magnitude = {m}");
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());

    let straight = tols
        .iter()
        .find(|t| t.kind() == ToleranceKind::Straightness)
        .expect("a straightness tolerance");
    assert!(straight.magnitude().is_none(), "an angle is not a length");
    assert_eq!(scene.warnings().len(), 1);
    assert!(scene.warnings()[0].message.contains("not a length measure"));
}

// A documented-part subtype (PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS) that
// owns PMI: two shape aspects (hole/slot), a DATUM, and a DIMENSIONAL_SIZE on the
// hole. Its PRODUCT_DEFINITION_SHAPE.definition points at the subtype, so this
// exercises that ProductDef::{features,datums,dimensions} resolve for the subtype
// exactly as for a plain PRODUCT_DEFINITION. The documentation ref must resolve
// (DOCUMENT + DOCUMENT_TYPE) or the part would cascade-drop.
const WITHDOCS_PMI: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('p','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS('design','',#11,#3,(#50));\n\
#13=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#50=DOCUMENT('d','doc','',#51);\n\
#51=DOCUMENT_TYPE('');\n\
#20=CARTESIAN_POINT('',(0.0,0.0,0.0));\n\
#21=CARTESIAN_POINT('',(1.0,0.0,0.0));\n\
#22=DIRECTION('',(0.0,0.0,1.0));\n\
#23=DIRECTION('',(1.0,0.0,0.0));\n\
#24=DIRECTION('',(1.0,0.0,0.0));\n\
#25=AXIS2_PLACEMENT_3D('',#20,#22,#23);\n\
#26=PLANE('',#25);\n\
#27=VECTOR('',#24,1.0);\n\
#28=LINE('',#20,#27);\n\
#29=VERTEX_POINT('',#20);\n\
#30=VERTEX_POINT('',#21);\n\
#31=EDGE_CURVE('',#29,#30,#28,.T.);\n\
#32=ORIENTED_EDGE('',*,*,#31,.T.);\n\
#33=EDGE_LOOP('',(#32));\n\
#34=FACE_OUTER_BOUND('',#33,.T.);\n\
#35=ADVANCED_FACE('',(#34),#26,.T.);\n\
#36=REPRESENTATION_CONTEXT('','3D');\n\
#37=SHAPE_REPRESENTATION('',(#35),#36);\n\
#40=SHAPE_ASPECT('hole','',#13,.T.);\n\
#41=DIMENSIONAL_SIZE(#40,'D1');\n\
#43=SHAPE_ASPECT('slot','',#13,.T.);\n\
#44=DATUM('datum a','',#13,.T.,'A');\n\
#42=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#40,#37,#35);\n";

#[test]
fn withdocs_part_reports_its_pmi() {
    let src = format!("{HEADER}{WITHDOCS_PMI}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // The sole product definition is the documented-part subtype.
    let part = scene
        .all_product_definitions()
        .next()
        .expect("a product definition");
    assert_eq!(
        part.documents().count(),
        1,
        "part is the documented subtype"
    );

    // Before the fix these were all empty for the subtype (PMI keyed on the plain
    // ProductDefinitionId only); they now resolve like a plain PRODUCT_DEFINITION.
    let mut feats: Vec<_> = part.features().map(|f| f.name().to_owned()).collect();
    feats.sort();
    assert_eq!(feats, vec!["hole".to_owned(), "slot".to_owned()]);
    assert_eq!(
        part.datums().count(),
        1,
        "documented part surfaces its datum"
    );
    assert_eq!(
        part.dimensions().count(),
        1,
        "documented part surfaces its dimension"
    );

    assert!(
        scene.warnings().is_empty(),
        "warnings: {:?}",
        scene.warnings()
    );
}

/// A feature can designate a display mesh or a geometric set: the rim of
/// `GEO_CURVE` given as a triangulated face, then as a curve set.
#[test]
fn feature_geometry_reaches_meshes_and_geometric_sets() {
    let circle = "#24=CIRCLE('',#23,5.0);";
    let mesh = GEO_CURVE.replace(
        circle,
        "#27=COORDINATES_LIST('',3,((0.,0.,0.),(1.,0.,0.),(0.,1.,0.)));\n#24=COMPLEX_TRIANGULATED_FACE('m',#27,3,(),$,(),((1,2,3)),());",
    );
    let set = GEO_CURVE.replace(circle, "#24=GEOMETRIC_CURVE_SET('w',(#20));");
    for (fixture, want_mesh) in [(mesh, true), (set, false)] {
        let src = format!("{HEADER}{fixture}{FOOTER}");
        let (model, report) = read(src.as_bytes()).expect("read");
        assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
        let scene = model.scene();
        let feat = scene.all_features().next().expect("the rim");
        let geo: Vec<_> = feat.geometry().collect();
        assert_eq!(geo.len(), 1);
        match geo[0] {
            FeatureGeometry::Mesh(m) if want_mesh => assert_eq!(m.name(), "m"),
            FeatureGeometry::GeometricSet(g) if !want_mesh => assert_eq!(g.name(), "w"),
            other => panic!("unexpected {other:?}"),
        }
        assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
    }
}

/// A tolerance on a dimension belongs to the part the dimension's feature is
/// on.
#[test]
fn a_tolerance_on_a_dimension_belongs_to_its_part() {
    let src = format!("{HEADER}{TWO_PARTS}#30=POSITION_TOLERANCE('posDA',$,$,#15);\n{FOOTER}");
    let (model, rep) = read(src.as_bytes()).expect("read");
    assert!(rep.dropped.is_empty(), "dropped: {:?}", rep.dropped);
    let scene = model.scene();
    let names = |product: &str| -> Vec<String> {
        scene
            .all_product_definitions()
            .find(|d| d.product().is_some_and(|p| p.name() == product))
            .expect("the part")
            .tolerances()
            .map(|t| t.name().to_owned())
            .collect()
    };
    assert!(names("partA").contains(&"posDA".to_owned()));
    assert!(!names("partB").contains(&"posDA".to_owned()));
    assert!(scene.warnings().is_empty(), "{:?}", scene.warnings());
}
