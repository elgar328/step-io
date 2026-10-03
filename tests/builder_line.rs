//! Straight edges through `StepBuilder`: what the written `LINE` carries — the
//! direction and the `VECTOR` magnitude — read back from the file.

use step_io::build::CurveInput;
use step_io::generated::model::{CurveRef, DirectionRef, StepModel, VectorRef};
use step_io::{StepBuilder, read};

/// One edge from `from` to `to` over `curve`, written and read back.
fn one_edge(from: [f64; 3], to: [f64; 3], curve: CurveInput) -> StepModel {
    let mut b = StepBuilder::new().expect("builder");
    let v0 = b.vertex(from).expect("vertex");
    let v1 = b.vertex(to).expect("vertex");
    b.edge(v0, v1, curve).expect("edge");
    let text = b.finish().expect("finish");
    let (model, report) = read(text.as_bytes()).expect("re-read");
    assert!(report.dropped.is_empty(), "drops: {:?}", report.dropped);
    model
}

/// A vector's bit patterns — the propositions here are bit-for-bit.
fn bits(v: [f64; 3]) -> [u64; 3] {
    v.map(f64::to_bits)
}

/// The written line's direction ratios and `VECTOR` magnitude.
fn line_of(model: &StepModel) -> ([f64; 3], f64) {
    assert_eq!(model.line_arena.items.len(), 1, "one edge, one LINE");
    let line = &model.line_arena.items[0];
    let VectorRef::Vector(v) = &line.dir else {
        panic!("a plain VECTOR")
    };
    let vector = model.vector_arena.get(v.0);
    let DirectionRef::Direction(d) = &vector.orientation else {
        panic!("a plain DIRECTION")
    };
    let r = &model.direction_arena.get(d.0).direction_ratios;
    ([r[0], r[1], r[2]], vector.magnitude)
}

/// `CurveInput::Line` derives the direction from the two vertex positions and
/// writes a unit magnitude — not the edge's length, which is only a parameter
/// scale. The edge is still the segment between its vertices.
#[test]
fn a_derived_line_has_unit_magnitude_and_the_vertices_bound_it() {
    let (from, to) = ([0.1, 0.2, 0.3], [1.3, 2.7, 1.9]);
    let model = one_edge(from, to, CurveInput::Line);
    let (dir, magnitude) = line_of(&model);
    assert_eq!(
        magnitude.to_bits(),
        1.0_f64.to_bits(),
        "the magnitude is the parameter scale, 1"
    );

    let delta = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let len = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    assert_eq!(
        bits(dir),
        bits([delta[0] / len, delta[1] / len, delta[2] / len])
    );

    let scene = model.scene();
    let edges: Vec<_> = scene.all_edges().collect();
    assert_eq!(edges.len(), 1);
    let segment = edges[0].to_nurbs().expect("a line edge is a segment");
    assert_eq!(segment.control_points, vec![from, to]);
    let CurveRef::Line(_) = model.edge_curve_arena.items[0].edge_geometry else {
        panic!("the edge's geometry is the LINE")
    };
}
