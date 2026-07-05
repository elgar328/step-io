//! Product / assembly API spike: navigate a minimal 1-level assembly — two
//! products (assembly + part) linked by a NAUO, with a solid on the assembly's
//! definition. Exercises reverse scans (children/parents/definitions/solids),
//! multi-hop forward (`def → formation → product`), and the cross-domain bridge
//! into the geometry `Solid` handle. The input is the minimum that resolves all
//! required references (contexts included); read does not enforce topology.

use step_io::read;
use step_io::scene::Scene;
use step_io::scene::product::ProductDef;
use step_io::scene::{Contributor, Scope, Target};

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

const ASSEMBLY: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#4=REPRESENTATION_CONTEXT('','3D');\n\
#10=PRODUCT('asm','Assembly','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('1','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#20=PRODUCT('part','Part','',(#2));\n\
#21=PRODUCT_DEFINITION_FORMATION('1','',#20);\n\
#22=PRODUCT_DEFINITION('design','',#21,#3);\n\
#30=NEXT_ASSEMBLY_USAGE_OCCURRENCE('1','asm-part','',#12,#22,$);\n\
#40=PRODUCT_DEFINITION_SHAPE('','',#12);\n\
#41=SHAPE_REPRESENTATION('',(#43),#4);\n\
#42=SHAPE_DEFINITION_REPRESENTATION(#40,#41);\n\
#43=MANIFOLD_SOLID_BREP('',#44);\n\
#44=CLOSED_SHELL('',());\n\
#50=PRODUCT_DEFINITION_SHAPE('','',#30);\n\
#51=SHAPE_REPRESENTATION('',(),#4);\n\
#52=CARTESIAN_POINT('',(10.,20.,30.));\n\
#53=DIRECTION('',(0.,0.,1.));\n\
#54=DIRECTION('',(1.,0.,0.));\n\
#55=AXIS2_PLACEMENT_3D('',#52,#53,#54);\n\
#56=CARTESIAN_POINT('',(0.,0.,0.));\n\
#57=AXIS2_PLACEMENT_3D('',#56,$,$);\n\
#58=ITEM_DEFINED_TRANSFORMATION('','',#57,#55);\n\
#59=( REPRESENTATION_RELATIONSHIP('','',#41,#51) \
REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#58) SHAPE_REPRESENTATION_RELATIONSHIP() );\n\
#60=CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#59,#50);\n";

fn approx(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
}

fn def_for_product<'a>(scene: &'a Scene<'_>, product_name: &str) -> ProductDef<'a> {
    scene
        .all_product_definitions()
        .find(|d| d.product().map(|p| p.name()) == Some(product_name))
        .unwrap_or_else(|| panic!("definition for product {product_name}"))
}

#[test]
fn navigates_assembly_tree_and_bridges_to_geometry() {
    let src = format!("{HEADER}{ASSEMBLY}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // Model-wide queries.
    assert_eq!(scene.all_products().count(), 2);
    assert_eq!(scene.all_product_definitions().count(), 2);

    let asm = def_for_product(&scene, "Assembly");
    let part = def_for_product(&scene, "Part");

    // Reverse navigation needs no `&rg` — the scene holds the lazily-built index.
    // Assembly tree — reverse via NAUO referrers; child resolved forward to product.
    let children: Vec<_> = asm.children().collect();
    assert_eq!(children.len(), 1, "assembly has one component");
    assert_eq!(children[0].product().unwrap().name(), "Part");

    let parents: Vec<_> = part.parents().collect();
    assert_eq!(parents.len(), 1, "part is used by one assembly");
    assert_eq!(parents[0].product().unwrap().name(), "Assembly");

    // A product's definitions — reverse via formation referrers.
    assert_eq!(asm.product().unwrap().definitions().count(), 1);

    // Keys distinguish entities and unify the same one reached two ways.
    assert_ne!(
        asm.key(),
        part.key(),
        "distinct definitions → distinct keys"
    );
    assert_eq!(asm.key(), def_for_product(&scene, "Assembly").key());

    // Cross-domain bridge: the assembly definition's shape carries one solid,
    // returned as a geometry `Solid` handle.
    assert_eq!(asm.solids().count(), 1, "assembly def has one solid");
    assert_eq!(part.solids().count(), 0, "part def has no shape here");
}

/// A part definition whose shape is a placement `SHAPE_REPRESENTATION` (no
/// solid), with the geometry in a separate `ADVANCED_BREP_SHAPE_REPRESENTATION`
/// bridged by a plain `SHAPE_REPRESENTATION_RELATIONSHIP`. `rel` is the
/// relationship line, so callers can flip `rep_1/rep_2` order.
fn bridged_part(rel: &str) -> String {
    format!(
        "{HEADER}\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#4=REPRESENTATION_CONTEXT('','3D');\n\
#20=PRODUCT('part','Part','',(#2));\n\
#21=PRODUCT_DEFINITION_FORMATION('1','',#20);\n\
#22=PRODUCT_DEFINITION('design','',#21,#3);\n\
#40=PRODUCT_DEFINITION_SHAPE('','',#22);\n\
#41=SHAPE_REPRESENTATION('',(#45),#4);\n\
#42=SHAPE_DEFINITION_REPRESENTATION(#40,#41);\n\
#45=AXIS2_PLACEMENT_3D('',#46,$,$);\n\
#46=CARTESIAN_POINT('',(0.,0.,0.));\n\
#50=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#51),#4);\n\
#51=MANIFOLD_SOLID_BREP('',#52);\n\
#52=CLOSED_SHELL('',());\n\
{rel}{FOOTER}"
    )
}

#[test]
fn solids_follow_shape_representation_relationship() {
    // The common assembly export: a component's own shape rep holds only a
    // placement, and its solid lives in a bridged ADVANCED_BREP rep. solids()
    // must cross the plain SHAPE_REPRESENTATION_RELATIONSHIP to find it.
    for rel in [
        "#60=SHAPE_REPRESENTATION_RELATIONSHIP('','',#41,#50);\n",
        "#60=SHAPE_REPRESENTATION_RELATIONSHIP('','',#50,#41);\n", // reversed order
    ] {
        let src = bridged_part(rel);
        let (model, _rep) = read(src.as_bytes()).expect("read");
        let scene = model.scene();
        let part = def_for_product(&scene, "Part");
        assert_eq!(
            part.solids().count(),
            1,
            "solid reached across the relationship (rel: {rel})"
        );
    }
}

#[test]
fn reads_occurrence_placement_transform() {
    let src = format!("{HEADER}{ASSEMBLY}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let asm = def_for_product(&scene, "Assembly");
    let occ = asm.occurrences().next().expect("one occurrence");
    assert_eq!(
        occ.definition().unwrap().product().unwrap().name(),
        "Part",
        "occurrence places the Part"
    );

    // The placement: transform_item_2 (`to`) carries the component position.
    let t = occ.transform().expect("a placement transform");
    assert!(approx(t.to.origin, [10.0, 20.0, 30.0]));
    assert!(approx(t.to.axis, [0.0, 0.0, 1.0]));
    assert!(approx(t.to.ref_direction, [1.0, 0.0, 0.0]));
    // transform_item_1 (`from`) — origin at 0 with omitted axes => STEP defaults.
    assert!(approx(t.from.origin, [0.0, 0.0, 0.0]));
    assert!(approx(t.from.axis, [0.0, 0.0, 1.0]));
    assert!(approx(t.from.ref_direction, [1.0, 0.0, 0.0]));

    // A well-formed model produces no navigation warnings.
    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

// A part version (formation id "RevB") with a person/organization assignment
// (John Doe / ACME, role "creator") on the formation — the corpus-dominant
// target for such assignments.
const META: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('part','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('RevB','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#20=PERSON('p1','Doe','John',$,$,$);\n\
#21=ORGANIZATION('o1','ACME',$);\n\
#22=PERSON_AND_ORGANIZATION(#20,#21);\n\
#23=PERSON_AND_ORGANIZATION_ROLE('creator');\n\
#24=CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT(#22,#23,(#11));\n";

#[test]
fn reads_version_and_contributors() {
    let src = format!("{HEADER}{META}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let def = scene
        .all_product_definitions()
        .next()
        .expect("a definition");
    assert_eq!(def.version(), Some("RevB"));

    let contributors = def.contributors();
    assert_eq!(contributors.len(), 1, "one contributor");
    let c = contributors[0];
    assert_eq!(c.role(), "creator");
    assert_eq!(c.person().last_name(), Some("Doe"));
    assert_eq!(c.person().first_name(), Some("John"));
    assert_eq!(c.organization(), "ACME");

    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

// One person (John Doe / ACME) contributes at all three tree levels via three
// separate assignments: creator on the product, design_owner on the formation,
// checker on the definition — to prove product-wide metadata reaches down the
// product → formation → definition tree.
const PRODUCT_META: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('part','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('RevB','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#20=PERSON('p1','Doe','John',$,$,$);\n\
#21=ORGANIZATION('o1','ACME',$);\n\
#22=PERSON_AND_ORGANIZATION(#20,#21);\n\
#23=PERSON_AND_ORGANIZATION_ROLE('creator');\n\
#24=CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT(#22,#23,(#10));\n\
#25=PERSON_AND_ORGANIZATION_ROLE('design_owner');\n\
#26=CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT(#22,#25,(#11));\n\
#27=PERSON_AND_ORGANIZATION_ROLE('checker');\n\
#28=CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT(#22,#27,(#12));\n";

#[test]
fn reads_product_level_metadata() {
    let src = format!("{HEADER}{PRODUCT_META}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let product = scene.all_products().next().expect("a product");
    let mut roles: Vec<&str> = product
        .contributors()
        .iter()
        .map(Contributor::role)
        .collect();
    roles.sort_unstable();
    // Reaches all three levels (product/formation/definition), not just the
    // product key — a product-key-only scope would return "creator" alone.
    assert_eq!(roles, ["checker", "creator", "design_owner"]);

    // Each contributor is labelled with the tree level it was assigned at.
    for c in &product.contributors() {
        let expected = match c.role() {
            "creator" => Scope::Product,
            "design_owner" => Scope::Version,
            "checker" => Scope::Definition,
            other => panic!("unexpected role {other}"),
        };
        assert_eq!(c.assigned_scope(), expected, "role {}", c.role());
    }

    // The definition's view gathers def + product + formation, so here it sees
    // the same three.
    let def = scene
        .all_product_definitions()
        .next()
        .expect("a definition");
    assert_eq!(def.contributors().len(), 3);

    // The symmetric accessors exist and are empty when nothing is assigned.
    assert!(product.approvals().is_empty());
    assert!(product.documents().is_empty());
    assert!(product.security_classifications().is_empty());

    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

// One product carrying two versions (RevA, RevB), each with its own definition —
// to exercise version listing and grouping definitions by version.
const VERSIONS: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('part','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('RevA','first',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_FORMATION('RevB','second',#10);\n\
#14=PRODUCT_DEFINITION('design','',#13,#3);\n";

#[test]
fn reads_product_versions() {
    let src = format!("{HEADER}{VERSIONS}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let product = scene.all_products().next().expect("a product");
    let versions = product.versions();
    assert_eq!(versions.len(), 2, "two versions");

    let mut labels: Vec<(&str, Option<&str>)> =
        versions.iter().map(|v| (v.id(), v.description())).collect();
    labels.sort_unstable();
    assert_eq!(labels, [("RevA", Some("first")), ("RevB", Some("second"))]);

    // Each version groups exactly its own definition (label matches the def's
    // version), confirming the version → definition edge.
    for v in &versions {
        let defs: Vec<_> = v.definitions().collect();
        assert_eq!(defs.len(), 1, "one definition per version");
        assert_eq!(defs[0].version(), Some(v.id()));
    }

    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

// Two assignments whose `items` list multiple disjoint targets: role "a" spans a
// version (RevA) plus a definition in the *other* branch (RevB's def); role "b"
// spans two definitions (RevA's + RevB's) — same level, distinct entities.
const TARGETS: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('part','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('RevA','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#13=PRODUCT_DEFINITION_FORMATION('RevB','',#10);\n\
#14=PRODUCT_DEFINITION('design','',#13,#3);\n\
#20=PERSON('p1','Doe','John',$,$,$);\n\
#21=ORGANIZATION('o1','ACME',$);\n\
#22=PERSON_AND_ORGANIZATION(#20,#21);\n\
#23=PERSON_AND_ORGANIZATION_ROLE('a');\n\
#24=CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT(#22,#23,(#11,#14));\n\
#25=PERSON_AND_ORGANIZATION_ROLE('b');\n\
#26=CC_DESIGN_PERSON_AND_ORGANIZATION_ASSIGNMENT(#22,#25,(#12,#14));\n";

// A version by its label, a definition by the version it belongs to.
fn describe_target(t: &Target<'_>) -> String {
    match t {
        Target::Product(p) => format!("prod:{}", p.name()),
        Target::Version(v) => format!("ver:{}", v.id()),
        Target::Definition(d) => format!("def:{}", d.version().unwrap_or("?")),
    }
}

#[test]
fn reads_assigned_targets() {
    let src = format!("{HEADER}{TARGETS}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();
    let product = scene.all_products().next().expect("a product");

    let targets_for = |role: &str| {
        let c = product
            .contributors()
            .into_iter()
            .find(|c| c.role() == role)
            .expect("role");
        let mut d: Vec<String> = c.assigned_targets().iter().map(describe_target).collect();
        d.sort();
        d
    };

    // Different levels — both kept, each identified by its entity.
    assert_eq!(targets_for("a"), ["def:RevB", "ver:RevA"]);
    // Same level (two definitions) — NOT collapsed; distinct entities preserved.
    assert_eq!(targets_for("b"), ["def:RevA", "def:RevB"]);

    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

// An approval (status "approved", level "final") on the formation, authorised by
// John Doe / ACME with role "approver".
const APPROVAL: &str = "\
#1=APPLICATION_CONTEXT('test');\n\
#2=PRODUCT_CONTEXT('',#1,'mechanical');\n\
#3=PRODUCT_DEFINITION_CONTEXT('',#1,'design');\n\
#10=PRODUCT('part','Part','',(#2));\n\
#11=PRODUCT_DEFINITION_FORMATION('RevB','',#10);\n\
#12=PRODUCT_DEFINITION('design','',#11,#3);\n\
#20=APPROVAL_STATUS('approved');\n\
#21=APPROVAL(#20,'final');\n\
#22=CC_DESIGN_APPROVAL(#21,(#11));\n\
#30=PERSON('p1','Doe','John',$,$,$);\n\
#31=ORGANIZATION('o1','ACME',$);\n\
#32=PERSON_AND_ORGANIZATION(#30,#31);\n\
#33=APPROVAL_ROLE('approver');\n\
#34=APPROVAL_PERSON_ORGANIZATION(#32,#21,#33);\n\
#40=CALENDAR_DATE(2024,15,3);\n\
#41=COORDINATED_UNIVERSAL_TIME_OFFSET(0,$,.EXACT.);\n\
#42=LOCAL_TIME(14,30,$,#41);\n\
#43=DATE_AND_TIME(#40,#42);\n\
#44=APPROVAL_DATE_TIME(#43,#21);\n\
#50=DOCUMENT_TYPE('drawing');\n\
#51=DOCUMENT('DOC-1','spec sheet','the spec',#50);\n\
#52=APPLIED_DOCUMENT_REFERENCE(#51,'source',(#11));\n\
#60=SECURITY_CLASSIFICATION_LEVEL('confidential');\n\
#61=SECURITY_CLASSIFICATION('SC-1','export control',#60);\n\
#62=CC_DESIGN_SECURITY_CLASSIFICATION(#61,(#11));\n";

#[test]
fn reads_approvals() {
    let src = format!("{HEADER}{APPROVAL}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let def = scene
        .all_product_definitions()
        .next()
        .expect("a definition");
    let approvals = def.approvals();
    assert_eq!(approvals.len(), 1, "one approval");
    let a = approvals[0];
    assert_eq!(a.status(), "approved");
    assert_eq!(a.level(), "final");
    // Assigned on the formation (#11) → version scope. `level()` (the approval's
    // own "final") and `assigned_scope()` are distinct concepts.
    assert_eq!(a.assigned_scope(), Scope::Version);

    let approvers = a.approvers();
    assert_eq!(approvers.len(), 1, "one approver");
    let who = approvers[0];
    assert_eq!(who.role(), "approver");
    assert_eq!(who.person().expect("a person").last_name(), Some("Doe"));
    assert_eq!(who.organization(), Some("ACME"));

    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

#[test]
fn reads_security_classification() {
    let src = format!("{HEADER}{APPROVAL}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let def = scene
        .all_product_definitions()
        .next()
        .expect("a definition");
    let classes = def.security_classifications();
    assert_eq!(classes.len(), 1, "one classification");
    let sc = classes[0];
    assert_eq!(sc.name(), "SC-1");
    assert_eq!(sc.purpose(), "export control");
    assert_eq!(sc.level(), "confidential");

    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

#[test]
fn reads_documents() {
    let src = format!("{HEADER}{APPROVAL}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let def = scene
        .all_product_definitions()
        .next()
        .expect("a definition");
    let docs = def.documents();
    assert_eq!(docs.len(), 1, "one document");
    let doc = docs[0];
    assert_eq!(doc.id(), "DOC-1");
    assert_eq!(doc.name(), "spec sheet");
    assert_eq!(doc.description(), Some("the spec"));
    assert_eq!(doc.kind(), "drawing");

    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

#[test]
fn reads_approval_date() {
    use step_io::scene::ApprovalDate;

    let src = format!("{HEADER}{APPROVAL}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    let def = scene
        .all_product_definitions()
        .next()
        .expect("a definition");
    let a = def.approvals()[0];
    assert_eq!(
        a.date(),
        Some(ApprovalDate {
            year: Some(2024),
            month: Some(3),
            day: Some(15),
            hour: Some(14),
            minute: Some(30),
        }),
    );
    assert!(
        scene.warnings().is_empty(),
        "unexpected warnings: {:?}",
        scene.warnings()
    );
}

#[test]
fn root_definitions_exclude_assembly_children() {
    let src = format!("{HEADER}{ASSEMBLY}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");
    let scene = model.scene();

    // The assembly definition is a root; the part is somebody's child.
    let roots: Vec<_> = scene.root_definitions().collect();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].product().expect("product").name(), "Assembly");
}
