//! Shape representation entity handlers (Pass 6-4 ~ 6-6, plus
//! `GLOBAL_UNIT_ASSIGNED_CONTEXT` at Pass 0-2, `MDGPR` at Pass 7-11, and
//! `SHAPE_ASPECT` at Pass 8-pre). Each handler impls
//! [`crate::entities::SimpleEntityHandler`] (or `ComplexEntityHandler`
//! for `GLOBAL_UNIT_ASSIGNED_CONTEXT`) and registers via the
//! `#[step_entity]` / `#[step_entity_complex]` proc-macro attributes
//! from `step_io_macros`.
//!
//! `MDGPR` (Pass 7-11) lives here despite being a visualization wrapper:
//! its dispatch boundary is `representation`, so the pool sits in
//! `shape_rep` per the ir.toml blueprint.

pub mod advanced_brep_shape_representation;
pub mod cgr_relationship;
pub mod characterized_item_within_representation;
pub mod compound_representation_item;
pub mod constructive_geometry_representation;
pub mod datum_system;
pub mod datum_target;
pub mod descriptive_representation_item;
pub mod draughting_model;
pub mod gbssr;
pub mod gbwsr;
pub mod geometric_item_specific_usage;
pub mod global_unit_assigned_context;
pub mod iiru;
pub mod item_defined_transformation;
pub mod manifold_surface_shape_representation;
pub mod mapped_item;
pub mod mddr;
pub mod mdgpr;
pub mod numeric_representation_item;
pub mod parametric_representation_context;
pub mod placed_datum_target_feature;
pub mod qualified_representation_item;
pub mod shape_aspect;
pub mod shape_aspect_relationship;
pub mod shape_aspect_subtypes;
pub mod shape_dimension_representation;
pub mod shape_representation;
pub mod shape_representation_relationship;
pub mod srwp;
pub mod tessellated_shape_representation;
pub mod tolerance_zone;
pub mod value_representation_item;
