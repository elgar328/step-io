//! B-rep authoring through `StepBuilder`: cube (planes + lines) and
//! cylinder (circles + cylindrical surface, shared/reused edges), read back
//! through the Scene.

use step_io::build::{CurveInput, FaceBoundInput, Frame, SurfaceInput, VoidShellNormals};
use step_io::generated::model::AdvancedFaceId;
use step_io::scene::geometry::{CurveKind, SurfaceKind};
use step_io::{EntityKey, StepBuilder, read};

fn frame(origin: [f64; 3], axis: [f64; 3], ref_dir: [f64; 3]) -> Frame {
    Frame {
        origin,
        axis,
        ref_dir,
    }
}

/// The 6 planar faces of an axis-aligned box (min corner + edge length),
/// closed into a shell by the caller. Corner index = x + 2y + 4z. With
/// `inward`, the normals point toward the box centre (a reversed shell, as a
/// kernel would hand back a cavity): each plane axis is negated and its loop
/// winding reversed to match.
fn box_faces(b: &mut StepBuilder, min: [f64; 3], size: f64, inward: bool) -> Vec<AdvancedFaceId> {
    let mut v = Vec::new();
    for z in 0..2 {
        for y in 0..2 {
            for x in 0..2 {
                v.push(
                    b.vertex([
                        min[0] + f64::from(x) * size,
                        min[1] + f64::from(y) * size,
                        min[2] + f64::from(z) * size,
                    ])
                    .expect("vertex"),
                );
            }
        }
    }
    let pairs = [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7),
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    let mut e = std::collections::HashMap::new();
    for (a_ix, b_ix) in pairs {
        let id = b.edge(v[a_ix], v[b_ix], CurveInput::Line).expect("edge");
        e.insert((a_ix, b_ix), id);
    }
    let edge = |a_ix: usize, b_ix: usize| {
        e.get(&(a_ix, b_ix))
            .map(|id| (*id, true))
            .or_else(|| e.get(&(b_ix, a_ix)).map(|id| (*id, false)))
            .expect("edge exists")
    };
    let [x0, y0, z0] = min;
    let faces_spec: [([usize; 4], Frame); 6] = [
        (
            [0, 2, 3, 1],
            frame([x0, y0, z0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
        ),
        (
            [4, 5, 7, 6],
            frame([x0, y0, z0 + size], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        ),
        (
            [0, 1, 5, 4],
            frame([x0, y0, z0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]),
        ),
        (
            [2, 6, 7, 3],
            frame([x0, y0 + size, z0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
        ),
        (
            [0, 4, 6, 2],
            frame([x0, y0, z0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ),
        (
            [1, 3, 7, 5],
            frame([x0 + size, y0, z0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ),
    ];
    let mut faces = Vec::new();
    for (loop_ixs, f) in faces_spec {
        let f = if inward {
            frame(f.origin, [-f.axis[0], -f.axis[1], -f.axis[2]], f.ref_dir)
        } else {
            f
        };
        let order = if inward {
            [loop_ixs[3], loop_ixs[2], loop_ixs[1], loop_ixs[0]]
        } else {
            loop_ixs
        };
        let edges = (0..4).map(|i| edge(order[i], order[(i + 1) % 4])).collect();
        faces.push(
            b.face(
                SurfaceInput::Plane(f),
                true,
                vec![FaceBoundInput::outer(edges)],
            )
            .expect("face"),
        );
    }
    faces
}

#[test]
#[allow(clippy::too_many_lines)]
fn cube_round_trips_and_reads_back() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("cube").expect("part");

    // 8 corners of the unit cube: index = x + 2y + 4z.
    let mut v = Vec::new();
    for z in 0..2 {
        for y in 0..2 {
            for x in 0..2 {
                v.push(
                    b.vertex([f64::from(x), f64::from(y), f64::from(z)])
                        .expect("vertex"),
                );
            }
        }
    }

    // 12 edges, keyed by their corner index pair (shared between faces).
    let pairs = [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7), // along X
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7), // along Y
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7), // along Z
    ];
    let mut e = std::collections::HashMap::new();
    for (a_ix, b_ix) in pairs {
        let id = b.edge(v[a_ix], v[b_ix], CurveInput::Line).expect("edge");
        e.insert((a_ix, b_ix), id);
    }
    let edge = |a_ix: usize, b_ix: usize| {
        e.get(&(a_ix, b_ix))
            .map(|id| (*id, true))
            .or_else(|| e.get(&(b_ix, a_ix)).map(|id| (*id, false)))
            .expect("edge exists")
    };

    // 6 faces: corner loop (counter-clockwise seen from outside) + plane.
    let faces_spec: [([usize; 4], Frame); 6] = [
        (
            [0, 2, 3, 1],
            frame([0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
        ), // bottom
        (
            [4, 5, 7, 6],
            frame([0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        ), // top
        (
            [0, 1, 5, 4],
            frame([0.0, 0.0, 0.0], [0.0, -1.0, 0.0], [1.0, 0.0, 0.0]),
        ), // front
        (
            [2, 6, 7, 3],
            frame([0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
        ), // back
        (
            [0, 4, 6, 2],
            frame([0.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ), // left
        (
            [1, 3, 7, 5],
            frame([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ), // right
    ];
    let mut faces = Vec::new();
    for (loop_ixs, f) in faces_spec {
        let edges = (0..4)
            .map(|i| edge(loop_ixs[i], loop_ixs[(i + 1) % 4]))
            .collect();
        faces.push(
            b.face(
                SurfaceInput::Plane(f),
                true,
                vec![FaceBoundInput::outer(edges)],
            )
            .expect("face"),
        );
    }
    b.solid(part, "cube body", faces).expect("solid");

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    // Promotion: the solid-bearing part writes an ABSR, not a plain SR.
    assert_eq!(
        model.advanced_brep_shape_representation_arena.items.len(),
        1
    );
    assert_eq!(model.shape_representation_arena.items.len(), 0);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    assert_eq!(solids.len(), 1);
    let faces: Vec<_> = solids[0].faces().collect();
    assert_eq!(faces.len(), 6);
    for face in &faces {
        assert!(matches!(face.surface().kind(), SurfaceKind::Plane(_)));
        let bounds: Vec<_> = face.bounds().collect();
        assert_eq!(bounds.len(), 1);
        assert!(bounds[0].is_outer());
        let edges: Vec<_> = bounds[0].oriented_edges().collect();
        assert_eq!(edges.len(), 4);
        for (edge, _forward) in edges {
            assert!(matches!(edge.curve().kind(), CurveKind::Line(_)));
        }
    }
}

#[test]
fn cylinder_round_trips_and_reads_back() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("pin").expect("part");

    let (radius, height) = (2.0, 5.0);
    let z_dir = [0.0, 0.0, 1.0];
    let x_dir = [1.0, 0.0, 0.0];

    // Seam vertices on the bottom and top circles.
    let v_bot = b.vertex([radius, 0.0, 0.0]).expect("v bot");
    let v_top = b.vertex([radius, 0.0, height]).expect("v top");

    // Full-circle edges (from == to) and the straight seam edge.
    let bottom = b
        .edge(
            v_bot,
            v_bot,
            CurveInput::Circle(frame([0.0; 3], z_dir, x_dir), radius),
        )
        .expect("bottom circle");
    let top = b
        .edge(
            v_top,
            v_top,
            CurveInput::Circle(frame([0.0, 0.0, height], z_dir, x_dir), radius),
        )
        .expect("top circle");
    let seam = b.edge(v_bot, v_top, CurveInput::Line).expect("seam");

    // Lateral face: the seam edge is used twice, in opposite directions.
    let lateral = b
        .face(
            SurfaceInput::Cylinder(frame([0.0; 3], z_dir, x_dir), radius),
            true,
            vec![FaceBoundInput::outer(vec![
                (bottom, true),
                (seam, true),
                (top, false),
                (seam, false),
            ])],
        )
        .expect("lateral face");
    let bottom_cap = b
        .face(
            SurfaceInput::Plane(frame([0.0; 3], [0.0, 0.0, -1.0], x_dir)),
            true,
            vec![FaceBoundInput::outer(vec![(bottom, false)])],
        )
        .expect("bottom cap");
    let top_cap = b
        .face(
            SurfaceInput::Plane(frame([0.0, 0.0, height], z_dir, x_dir)),
            true,
            vec![FaceBoundInput::outer(vec![(top, true)])],
        )
        .expect("top cap");
    b.solid(part, "pin body", vec![lateral, bottom_cap, top_cap])
        .expect("solid");

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    assert_eq!(solids.len(), 1);
    let faces: Vec<_> = solids[0].faces().collect();
    assert_eq!(faces.len(), 3);

    let mut cylindrical = 0;
    let mut planes = 0;
    for face in &faces {
        match face.surface().kind() {
            SurfaceKind::Cylindrical(_) => cylindrical += 1,
            SurfaceKind::Plane(_) => planes += 1,
            other => panic!("unexpected surface kind: {other:?}"),
        }
        for bound in face.bounds() {
            for (edge, _forward) in bound.oriented_edges() {
                assert!(matches!(
                    edge.curve().kind(),
                    CurveKind::Line(_) | CurveKind::Circle(_)
                ));
            }
        }
    }
    assert_eq!((cylindrical, planes), (1, 2));

    // The lateral loop reuses the seam edge twice: 4 oriented edges.
    let lateral_face = faces
        .iter()
        .find(|f| matches!(f.surface().kind(), SurfaceKind::Cylindrical(_)))
        .unwrap();
    let lateral_edges: Vec<_> = lateral_face
        .bounds()
        .next()
        .unwrap()
        .oriented_edges()
        .collect();
    assert_eq!(lateral_edges.len(), 4);
}

#[test]
fn empty_bounds_and_faces_are_rejected() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("plate").expect("part");

    let err = b
        .face(
            SurfaceInput::Plane(frame([0.0; 3], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0])),
            true,
            vec![],
        )
        .expect_err("a face without bounds is schema-invalid");
    assert!(matches!(
        err,
        step_io::AuthorError::Cardinality {
            entity: "ADVANCED_FACE",
            attribute: "bounds",
            ..
        }
    ));

    let err = b
        .solid(part, "empty", vec![])
        .expect_err("a shell without faces is schema-invalid");
    assert!(matches!(
        err,
        step_io::AuthorError::Cardinality {
            entity: "CLOSED_SHELL",
            attribute: "cfs_faces",
            ..
        }
    ));
}

#[test]
fn void_solid_round_trips_and_reads_back() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("hollow block").expect("part");

    // Outer 10-unit box with a 4-unit cubic cavity centred inside it. The
    // cavity is built as an ordinary outward box (normals toward the
    // surrounding material), so it is declared `TowardMaterial` and step-io
    // reverses it into the void.
    let outer = box_faces(&mut b, [0.0, 0.0, 0.0], 10.0, false);
    let cavity = box_faces(&mut b, [3.0, 3.0, 3.0], 4.0, false);
    b.solid_with_voids(
        part,
        "hollow body",
        outer,
        vec![cavity],
        VoidShellNormals::TowardMaterial,
    )
    .expect("void solid");

    let text = b.finish().expect("finish");
    assert!(
        text.contains("BREP_WITH_VOIDS"),
        "output should carry a BREP_WITH_VOIDS"
    );
    // TowardMaterial faces are reversed into the void: the oriented shell is `.F.`.
    assert!(
        text.lines()
            .any(|l| l.contains("ORIENTED_CLOSED_SHELL") && l.contains(".F.")),
        "TowardMaterial should reverse the void shell (.F.)"
    );

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    // Promotion: a void solid still writes an ABSR, not a plain SR.
    assert_eq!(
        model.advanced_brep_shape_representation_arena.items.len(),
        1
    );
    assert_eq!(model.shape_representation_arena.items.len(), 0);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    assert_eq!(solids.len(), 1, "the void solid surfaces in all_solids");
    let solid = solids[0];
    assert!(matches!(solid.key(), EntityKey::BrepWithVoids(_)));

    // Outer shell = 6 faces; one cavity, also 6 faces.
    assert_eq!(solid.faces().count(), 6);
    let voids = solid.voids();
    assert_eq!(voids.len(), 1);
    assert_eq!(voids[0].len(), 6);

    // Every face round-trips back to this solid.
    for face in solid.faces() {
        assert_eq!(face.solid().expect("owning solid").key(), solid.key());
    }
    for face in &voids[0] {
        assert_eq!(face.solid().expect("owning solid").key(), solid.key());
    }

    // The part's definition surfaces the void solid too.
    let def = scene
        .all_product_definitions()
        .next()
        .expect("a product definition");
    let product_solids: Vec<_> = def.solids().collect();
    assert_eq!(product_solids.len(), 1);
    assert!(matches!(
        product_solids[0].key(),
        EntityKey::BrepWithVoids(_)
    ));
}

#[test]
fn void_shell_away_from_material_emits_t() {
    let mut b = StepBuilder::new().expect("builder");
    let part = b.part("hollow block").expect("part");

    // The cavity is authored the way a kernel hands one back: a reversed shell
    // whose normals point into the void (away from the material), so it is
    // declared `AwayFromMaterial` and step-io keeps it as authored.
    let outer = box_faces(&mut b, [0.0, 0.0, 0.0], 10.0, false);
    let cavity = box_faces(&mut b, [3.0, 3.0, 3.0], 4.0, true);
    b.solid_with_voids(
        part,
        "hollow body",
        outer,
        vec![cavity],
        VoidShellNormals::AwayFromMaterial,
    )
    .expect("void solid");

    let text = b.finish().expect("finish");
    // As-authored: the oriented shell is `.T.`, and no void shell is reversed.
    assert!(
        text.lines()
            .any(|l| l.contains("ORIENTED_CLOSED_SHELL") && l.contains(".T.")),
        "AwayFromMaterial should keep the void shell as authored (.T.)"
    );
    assert!(
        !text
            .lines()
            .any(|l| l.contains("ORIENTED_CLOSED_SHELL") && l.contains(".F.")),
        "no void shell should be reversed"
    );

    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    let scene = model.scene();
    let solid = scene.all_solids().next().expect("the void solid");
    assert!(matches!(solid.key(), EntityKey::BrepWithVoids(_)));
    let voids = solid.voids();
    assert_eq!(voids.len(), 1);
    assert_eq!(voids[0].len(), 6);
}

#[test]
fn manifold_and_void_solids_coexist_in_all_solids() {
    let mut b = StepBuilder::new().expect("builder");
    let solid_part = b.part("solid block").expect("part");
    let plain = box_faces(&mut b, [0.0, 0.0, 0.0], 5.0, false);
    b.solid(solid_part, "plain body", plain).expect("solid");

    let void_part = b.part("hollow block").expect("part");
    let outer = box_faces(&mut b, [0.0, 0.0, 0.0], 10.0, false);
    let cavity = box_faces(&mut b, [3.0, 3.0, 3.0], 4.0, false);
    b.solid_with_voids(
        void_part,
        "hollow body",
        outer,
        vec![cavity],
        VoidShellNormals::TowardMaterial,
    )
    .expect("void solid");

    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);

    let scene = model.scene();
    let solids: Vec<_> = scene.all_solids().collect();
    assert_eq!(solids.len(), 2, "both solids surface");
    assert_eq!(
        solids
            .iter()
            .filter(|s| matches!(s.key(), EntityKey::ManifoldSolidBrep(_)))
            .count(),
        1
    );
    assert_eq!(
        solids
            .iter()
            .filter(|s| matches!(s.key(), EntityKey::BrepWithVoids(_)))
            .count(),
        1
    );
}
