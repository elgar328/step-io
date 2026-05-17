//! Visualization entity handlers (Pass 7-1 ~ 7-10). Each handler impls
//! [`crate::entities::SimpleEntityHandler`] and registers via the
//! `#[step_entity]` proc-macro attribute from `step_io_macros`. The 10
//! entities form a chain: leaf colour / transparent → fill / surface
//! style aggregations → side / usage / assignment wrappers → styled item.
//! The MDGPR top-level wrapper (Pass 7-11) sits in `entities/shape_rep/`
//! because its dispatch boundary is `representation`.

pub mod colour_rgb;
pub mod draughting_pre_defined_colour;
pub mod fill_area_style;
pub mod fill_area_style_colour;
pub mod presentation_style_assignment;
pub mod styled_item;
pub mod surface_side_style;
pub mod surface_style_fill_area;
pub mod surface_style_rendering_with_properties;
pub mod surface_style_transparent;
pub mod surface_style_usage;
