//! codegen crate library: exposes the generated schema-faithful module and the
//! merkle round-trip oracle to the `roundtrip` verifier binary.

pub mod merkle;

pub mod entity_normalize;

#[path = "generated/mod.rs"]
pub mod generated;

pub mod check;
