//! Visualization entity handlers (Pass 7-1 ~ 7-11).
//!
//! Plan 7 migrates the bespoke `run_pass!` blocks in `run_assembly_passes`
//! (`src/reader/passes.rs` line 276~293) into per-entity handlers. The
//! 11 entities form a chain: leaf colour / transparent → fill / surface
//! style aggregations → side / usage / assignment wrappers → styled item
//! → MDGPR. Stubs land in stage C1 and gain bodies in stages C2~C4.

pub mod colour_rgb;
pub mod fill_area_style;
pub mod fill_area_style_colour;
pub mod mdgpr;
pub mod presentation_style_assignment;
pub mod styled_item;
pub mod surface_side_style;
pub mod surface_style_fill_area;
pub mod surface_style_rendering_with_properties;
pub mod surface_style_transparent;
pub mod surface_style_usage;
