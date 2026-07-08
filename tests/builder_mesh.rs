//! Display-mesh authoring through `StepBuilder::mesh`: a plain vertex array
//! with index triangles round-trips exactly through the read side's
//! Mesh/MeshGroup (including the 1-based strip winding encoding), with the
//! group linked back to the b-rep solid it tessellates.

use step_io::build::{
    CurveInput, FaceBoundInput, Frame, MeshInput, MeshNormalsInput, SurfaceInput,
};
use step_io::scene::mesh::MeshNormals;
use step_io::{StepBuilder, read};

fn frame(origin: [f64; 3], axis: [f64; 3], ref_dir: [f64; 3]) -> Frame {
    Frame {
        origin,
        axis,
        ref_dir,
    }
}

/// A quad split into two triangles, as a kernel tessellation would hand it
/// over: vertex array + 0-based index triangles + per-vertex normals.
fn quad_mesh() -> MeshInput {
    MeshInput {
        points: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        triangles: vec![[0, 1, 2], [0, 2, 3]],
        normals: MeshNormalsInput::PerVertex(vec![[0.0, 0.0, 1.0]; 4]),
    }
}

#[test]
fn mesh_round_trips_with_solid_back_link() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("plate").expect("part");

    // A minimal b-rep solid (single planar face) for the mesh to link to.
    let v0 = b.vertex([0.0, 0.0, 0.0]).expect("v0");
    let v1 = b.vertex([1.0, 0.0, 0.0]).expect("v1");
    let v2 = b.vertex([1.0, 1.0, 0.0]).expect("v2");
    let e0 = b.edge(v0, v1, CurveInput::Line).expect("e0");
    let e1 = b.edge(v1, v2, CurveInput::Line).expect("e1");
    let e2 = b.edge(v2, v0, CurveInput::Line).expect("e2");
    let face = b
        .face(
            SurfaceInput::Plane(frame([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0])),
            true,
            vec![FaceBoundInput::outer(vec![
                (e0, true),
                (e1, true),
                (e2, true),
            ])],
        )
        .expect("face");
    let body = b.solid(part, "plate body", vec![face]).expect("solid");

    let input = quad_mesh();
    b.mesh(part, "plate mesh", &input, Some(body.into()))
        .expect("mesh");
    let text = b.finish().expect("finish");

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    assert_eq!(model.tessellated_shape_representation_arena.items.len(), 1);
    assert_eq!(
        model.shape_definition_representation_arena.items.len(),
        2,
        "shape SDR + mesh SDR"
    );

    let scene = model.scene();
    let groups: Vec<_> = scene.all_mesh_groups().collect();
    assert_eq!(groups.len(), 1);
    let meshes = groups[0].meshes();
    assert_eq!(meshes.len(), 1);

    // Exact symmetry: points and 0-based triangles come back as fed, which
    // also pins the 1-based strip winding encoding.
    assert_eq!(meshes[0].points(), input.points);
    assert_eq!(meshes[0].triangles(), input.triangles);
    match meshes[0].normals() {
        MeshNormals::PerVertex(rows) => assert_eq!(rows, vec![[0.0, 0.0, 1.0]; 4]),
        other => panic!("expected per-vertex normals, got {other:?}"),
    }

    // The group links back to the b-rep solid it tessellates.
    let linked = groups[0].solid().expect("solid back-link");
    let solids: Vec<_> = scene.all_solids().collect();
    assert_eq!(solids.len(), 1);
    assert_eq!(linked.key(), solids[0].key());
}

#[test]
fn mesh_links_back_to_void_solid() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("hollow").expect("part");

    // Minimal void solid: one-face outer shell + one-face void shell.
    let mut tri = |z: f64| {
        let v0 = b.vertex([0.0, 0.0, z]).expect("v0");
        let v1 = b.vertex([1.0, 0.0, z]).expect("v1");
        let v2 = b.vertex([1.0, 1.0, z]).expect("v2");
        let e0 = b.edge(v0, v1, CurveInput::Line).expect("e0");
        let e1 = b.edge(v1, v2, CurveInput::Line).expect("e1");
        let e2 = b.edge(v2, v0, CurveInput::Line).expect("e2");
        b.face(
            SurfaceInput::Plane(frame([0.0, 0.0, z], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0])),
            true,
            vec![FaceBoundInput::outer(vec![
                (e0, true),
                (e1, true),
                (e2, true),
            ])],
        )
        .expect("face")
    };
    let outer = tri(0.0);
    let cavity = tri(0.5);
    let body = b
        .solid_with_voids(part, "hollow body", vec![outer], vec![vec![cavity]])
        .expect("void solid");

    b.mesh(part, "hollow mesh", &quad_mesh(), Some(body.into()))
        .expect("mesh");
    let text = b.finish().expect("finish");

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let groups: Vec<_> = scene.all_mesh_groups().collect();
    assert_eq!(groups.len(), 1);
    let linked = groups[0].solid().expect("solid back-link");
    assert!(matches!(linked.key(), step_io::EntityKey::BrepWithVoids(_)));
}

#[test]
fn standalone_mesh_with_uniform_normals() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("panel").expect("part");
    b.mesh(
        part,
        "panel mesh",
        &MeshInput {
            points: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            triangles: vec![[0, 1, 2]],
            normals: MeshNormalsInput::Uniform([0.0, 0.0, 1.0]),
        },
        None,
    )
    .expect("mesh");
    let text = b.finish().expect("finish");

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let groups: Vec<_> = scene.all_mesh_groups().collect();
    assert_eq!(groups.len(), 1);
    assert!(groups[0].solid().is_none(), "no b-rep link");
    let meshes = groups[0].meshes();
    assert_eq!(meshes[0].triangles(), vec![[0, 1, 2]]);
    assert!(matches!(
        meshes[0].normals(),
        MeshNormals::Uniform([0.0, 0.0, 1.0])
    ));
}
