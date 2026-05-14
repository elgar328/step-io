//! Topology-group entity handlers (catalog `topology` group).
//!
//! Plan 4 migrates the Pass 5-1 ~ 5-8 family from `src/reader/topology.rs`
//! and `src/writer/buffer/topology.rs` into per-entity handler modules.
//! Each module impls [`crate::entities::SimpleEntityHandler`] and submits a
//! [`crate::entities::EntityHandlerEntry`] via
//! `#[linkme::distributed_slice(ENTITY_HANDLERS)]`.
//!
//! The Pass 5-N ordering is preserved through dedicated `PassLevel`
//! variants (`Pass5Edge`, `Pass5OrientedEdge`, `Pass5EdgeLoop`,
//! `Pass5FaceBound`, `Pass5Face`, `Pass5Shell`, `Pass5OrientedShell`,
//! `Pass5Solid`). `Pass5Vertex` lives here in ordering but the
//! `VERTEX_POINT` handler sits in `entities/geometry/` per the ir.toml
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
