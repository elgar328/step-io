//! Visualization entity handlers (Pass 7-1 ~ 7-10). Each handler impls
//! [`crate::entities::SimpleEntityHandler`] and registers via the
//! `#[step_entity]` proc-macro attribute from `step_io_macros`. The 10
//! entities form a chain: leaf colour / transparent → fill / surface
//! style aggregations → side / usage / assignment wrappers → styled item.
//! The MDGPR top-level wrapper (Pass 7-11) sits in `entities/shape_rep/`
//! because its dispatch boundary is `representation`.

pub mod camera_image;
pub mod camera_model_d3;
pub mod camera_model_variants;
pub mod camera_usage;
pub mod colour_rgb;
pub mod composite_text;
pub mod context_dependent_over_riding_styled_item;
pub mod curve_style;
pub mod draughting_pre_defined_colour;
pub mod draughting_pre_defined_curve_font;
pub mod fill_area_style;
pub mod fill_area_style_colour;
pub mod invisibility;
pub mod over_riding_styled_item;
pub mod point_style;
pub mod pre_defined_curve_font;
pub mod pre_defined_marker;
pub mod pre_defined_symbol;
pub mod pre_defined_terminator_symbol;
pub mod presentation_layer_assignment;
pub mod presentation_representation;
pub mod presentation_size;
pub mod presentation_style_assignment;
pub mod presentation_style_by_context;
pub mod presented_item;
pub mod styled_item;
pub mod surface_side_style;
pub mod surface_style_boundary;
pub mod surface_style_fill_area;
pub mod surface_style_parameter_line;
pub mod surface_style_rendering;
pub mod surface_style_rendering_with_properties;
pub mod surface_style_transparent;
pub mod surface_style_usage;
pub mod symbol;
pub mod symbol_colour;
pub mod symbol_style;
pub mod text_literal;
pub mod text_style_for_defined_font;
pub mod text_style_with_box_characteristics;
pub mod view_volume;
