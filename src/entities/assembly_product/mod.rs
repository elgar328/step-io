//! Assembly + product entity handlers (Pass 6-1 ~ 6-3, 6-7, 6-8). Each
//! handler impls [`crate::entities::SimpleEntityHandler`] and registers
//! via the `#[step_entity]` proc-macro attribute from `step_io_macros`.
//! The graph-aware variants (CDSR, `PRODUCT_DEFINITION_SHAPE` classifier)
//! receive `&EntityGraph` through the unified handler signature.

pub mod context_dependent_shape_representation;
pub mod design_context;
pub mod mechanical_context;
pub mod next_assembly_usage_occurrence;
pub mod product;
pub mod product_category;
pub mod product_category_relationship;
pub mod product_context;
pub mod product_definition;
pub mod product_definition_context;
pub mod product_definition_context_association;
pub mod product_definition_context_role;
pub mod product_definition_formation;
pub mod product_definition_formation_with_source;
pub mod product_definition_shape;
pub mod product_definition_with_associated_documents;
pub mod product_related_product_category;
pub(crate) mod shared;
