//! PMI dimensions API: a `DIMENSIONAL_SIZE` whose nominal value is reached
//! through `DIMENSIONAL_CHARACTERISTIC_REPRESENTATION` → a
//! `SHAPE_DIMENSION_REPRESENTATION`'s `MEASURE_REPRESENTATION_ITEM`. The DCR is a
//! root, so it anchors the whole chain through settling.

use step_io::read;
use step_io::scene::geometry::{CurveKind, SurfaceKind};
use step_io::scene::pmi::{DimensionKind, Feature, FeatureGeometry, FeatureKind, ToleranceKind};

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
    let dim = scene.dimensions().next().expect("a dimension");
    let feat = dim.features();
    assert_eq!(feat.len(), 1, "size targets one feature");
    assert_eq!(feat[0].name(), "hole");

    let geo = feat[0].geometry();
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

// A label for the FeatureGeometry variant, for test failure messages.
fn discriminant(g: &FeatureGeometry) -> &'static str {
    match g {
        FeatureGeometry::Face(_) => "Face",
        FeatureGeometry::Edge(_) => "Edge",
        FeatureGeometry::Point(_) => "Point",
        FeatureGeometry::Solid(_) => "Solid",
        FeatureGeometry::Curve(_) => "Curve",
        FeatureGeometry::Other(_) => "Other",
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

// Two shape aspects (hole/slot) plus a DATUM. scene.features() lists the two
// shape aspects but NOT the datum (which is a datum callout, reached via
// scene.datums()); the hole is linked to a real face via GISU.
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
    let mut names: Vec<_> = scene.features().map(|f| f.name().to_owned()).collect();
    names.sort();
    assert_eq!(names, vec!["hole".to_owned(), "slot".to_owned()]);

    // The datum is not a feature but IS a datum.
    assert_eq!(scene.datums().count(), 1);
    assert_eq!(scene.datums().next().expect("a datum").letter(), "A");

    // A scene-enumerated feature walks straight to its geometry, and kind()
    // names its backing entity without matching on EntityKey.
    let hole = scene
        .features()
        .find(|f| f.name() == "hole")
        .expect("the hole feature");
    assert_eq!(hole.kind(), FeatureKind::ShapeAspect);
    match &hole.geometry()[0] {
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

    let dim = scene.dimensions().next().expect("a dimension");
    let feat = dim.features();
    assert_eq!(feat[0].name(), "rim");

    let geo = feat[0].geometry();
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

    // scene.datums(): both DATUMs, by letter.
    let mut letters: Vec<_> = scene.datums().map(|d| d.letter().to_owned()).collect();
    letters.sort();
    assert_eq!(letters, vec!["A".to_owned(), "B".to_owned()]);

    // Complex position tolerance → DATUM_SYSTEM → compartment → datum A.
    let tols: Vec<_> = scene.tolerances().collect();
    let pos = tols
        .iter()
        .find(|t| t.kind() == ToleranceKind::Position)
        .expect("a position tolerance");
    let pos_datums = pos.datums();
    assert_eq!(pos_datums.len(), 1, "position references one datum");
    assert_eq!(pos_datums[0].letter(), "A");

    // Standalone perpendicularity tolerance → DATUM_REFERENCE → datum B.
    let perp = tols
        .iter()
        .find(|t| t.kind() == ToleranceKind::Perpendicularity)
        .expect("a perpendicularity tolerance");
    let perp_datums = perp.datums();
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
    let dims: Vec<_> = scene.dimensions().collect();
    let size = dims
        .iter()
        .find(|d| d.kind() == DimensionKind::Size)
        .expect("a size");
    let size_features = size.features();
    assert_eq!(size_features.len(), 1);
    assert_eq!(size_features[0].name(), "hole");

    let loc = dims
        .iter()
        .find(|d| d.kind() == DimensionKind::Location)
        .expect("a location");
    let loc_features: Vec<_> = loc.features().iter().map(Feature::name).collect();
    assert_eq!(loc_features, vec!["hole", "slot"]);

    // A dedicated tolerance and a complex tolerance each resolve their target.
    let tols: Vec<_> = scene.tolerances().collect();
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

    let dims: Vec<_> = scene.dimensions().collect();
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

// Three geometric tolerances sharing the product/aspect/unit scaffold: a
// standalone FLATNESS_TOLERANCE (dedicated arena), a complex position tolerance
// (multi-supertype ComplexUnit), and a magnitude-less STRAIGHTNESS_TOLERANCE.
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

    let tols: Vec<_> = scene.tolerances().collect();
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
