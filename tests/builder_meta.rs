//! Product-metadata authoring through `StepBuilder`: part number, version,
//! contributors, approvals (with approvers and a date), documents, and
//! security classifications — read back through the Scene's metadata
//! accessors, which are the consumer contract this mirrors.

use step_io::build::{ApprovalInput, DateTimeInput, DocumentInput, PartOptions, PersonOrg};
use step_io::{StepBuilder, read};

#[test]
#[allow(clippy::too_many_lines)]
fn full_metadata_round_trips_and_reads_back() {
    let mut b = StepBuilder::new().expect("builder");
    let wheel = b
        .part_with(
            "wheel",
            &PartOptions {
                id: Some("W-100".to_owned()),
                description: Some("front wheel".to_owned()),
                version: Some("B".to_owned()),
                version_description: Some("second revision".to_owned()),
                definition_description: Some("design view".to_owned()),
            },
        )
        .expect("part");

    let john = b
        .person_and_org(&PersonOrg {
            person_id: "p1".to_owned(),
            first_name: Some("John".to_owned()),
            last_name: Some("Doe".to_owned()),
            organization: "ACME".to_owned(),
        })
        .expect("person");

    b.contributor(wheel, "creator", john).expect("contributor");
    b.approve(
        wheel,
        &ApprovalInput {
            status: "approved".to_owned(),
            level: "final".to_owned(),
            approvers: vec![(john, "approver".to_owned())],
            date: Some(DateTimeInput {
                year: 2024,
                month: 3,
                day: 15,
                hour: 14,
                minute: Some(30),
            }),
        },
    )
    .expect("approve");
    b.document(
        wheel,
        &DocumentInput {
            id: "DOC-1".to_owned(),
            name: "spec sheet".to_owned(),
            description: Some("the spec".to_owned()),
            kind: "drawing".to_owned(),
        },
    )
    .expect("document");
    b.security(wheel, "SC-1", "export control", "confidential")
        .expect("security");

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let products: Vec<_> = scene.all_products().collect();
    assert_eq!(products.len(), 1);
    let product = &products[0];

    // Identity + version labels.
    assert_eq!(product.id(), "W-100");
    assert_eq!(product.name(), "wheel");
    assert_eq!(product.description(), Some("front wheel"));
    let versions = product.versions();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].id(), "B");
    assert_eq!(versions[0].description(), Some("second revision"));
    let defs: Vec<_> = product.definitions().collect();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].version(), Some("B"));
    assert_eq!(defs[0].description(), Some("design view"));

    // Contributor.
    let contributors = product.contributors();
    assert_eq!(contributors.len(), 1);
    assert_eq!(contributors[0].role(), "creator");
    assert_eq!(contributors[0].person().first_name(), Some("John"));
    assert_eq!(contributors[0].person().last_name(), Some("Doe"));
    assert_eq!(contributors[0].organization(), "ACME");

    // Approval with approver and date.
    let approvals = product.approvals();
    assert_eq!(approvals.len(), 1);
    assert_eq!(approvals[0].status(), "approved");
    assert_eq!(approvals[0].level(), "final");
    let approvers = approvals[0].approvers();
    assert_eq!(approvers.len(), 1);
    assert_eq!(approvers[0].role(), "approver");
    assert_eq!(
        approvers[0].person().and_then(|p| p.last_name()),
        Some("Doe")
    );
    assert_eq!(approvers[0].organization(), Some("ACME"));
    let date = approvals[0].date().expect("approval date");
    assert_eq!(
        (date.year, date.month, date.day, date.hour, date.minute),
        (Some(2024), Some(3), Some(15), Some(14), Some(30))
    );

    // Document.
    let documents = product.documents();
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].id(), "DOC-1");
    assert_eq!(documents[0].name(), "spec sheet");
    assert_eq!(documents[0].description(), Some("the spec"));
    assert_eq!(documents[0].kind(), "drawing");

    // Security classification.
    let security = product.security_classifications();
    assert_eq!(security.len(), 1);
    assert_eq!(security[0].name(), "SC-1");
    assert_eq!(security[0].purpose(), "export control");
    assert_eq!(security[0].level(), "confidential");
}

#[test]
fn shared_person_across_parts_and_roles() {
    let mut b = StepBuilder::new().expect("builder");
    let a_part = b.part("body").expect("body");
    let b_part = b.part("lid").expect("lid");
    let alice = b
        .person_and_org(&PersonOrg {
            person_id: "p2".to_owned(),
            first_name: Some("Alice".to_owned()),
            last_name: None,
            organization: "ACME".to_owned(),
        })
        .expect("person");
    b.contributor(a_part, "creator", alice).expect("c1");
    b.contributor(b_part, "design_owner", alice).expect("c2");

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    // One shared person entity, two role assignments.
    assert_eq!(model.person_arena.items.len(), 1);
    assert_eq!(
        model
            .applied_person_and_organization_assignment_arena
            .items
            .len(),
        2
    );

    let scene = model.scene();
    let roles: Vec<String> = scene
        .all_products()
        .flat_map(|p| {
            p.contributors()
                .iter()
                .map(|c| c.role().to_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(roles.len(), 2);
    assert!(roles.contains(&"creator".to_owned()));
    assert!(roles.contains(&"design_owner".to_owned()));
}
