//! Topology-group entity handlers (catalog `topology` group, 14 entities).
//!
//! Plan 4 migrates the Pass 5-1 ~ 5-8 family from `src/reader/topology.rs`
//! and `src/writer/buffer/topology.rs` into per-entity handler modules.
//! Each module impls [`crate::entities::SimpleEntityHandler`] and submits a
//! [`crate::entities::EntityHandlerEntry`] via
//! `#[linkme::distributed_slice(ENTITY_HANDLERS)]`.
//!
//! The Pass 5-N ordering is preserved through dedicated `PassLevel`
//! variants (`Pass5Vertex`, `Pass5Edge`, `Pass5OrientedEdge`,
//! `Pass5EdgeLoop`, `Pass5FaceBound`, `Pass5Face`, `Pass5Shell`,
//! `Pass5OrientedShell`, `Pass5Solid`).

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
pub mod vertex_point;
