//! Topology-group entity handlers (catalog `topology` group). Each handler impls
//! [`crate::entities::SimpleEntityHandler`] and registers via the
//! `#[step_entity]` proc-macro attribute from `step_io_macros`.
//!
//! The `VERTEX_POINT` handler sits in `entities/geometry/` per the ir.toml
//! blueprint (`vertex_point` is also a `geometric_representation_item`).

pub mod advanced_face;
pub mod brep_with_voids;
pub mod closed_shell;
pub mod edge_curve;
pub mod edge_loop;
pub mod face_bound;
pub mod face_outer_bound;
pub mod face_surface;
pub mod manifold_solid_brep;
pub mod open_shell;
pub mod oriented_closed_shell;
pub mod oriented_edge;
pub mod vertex_loop;
