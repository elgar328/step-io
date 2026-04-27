//! Property entity handlers (Pass 8-1 ~ 8-2).
//!
//! Plan 7 migrates `MEASURE_REPRESENTATION_ITEM` and `PROPERTY_DEFINITION`
//! into per-entity handlers. `PROPERTY_DEFINITION_REPRESENTATION` stays
//! hand-rolled in `src/reader/property.rs` because its read body needs
//! `&EntityGraph` to walk the bound REPRESENTATION (a generic entity name
//! shared with MDGPR / SR — a per-pass map would conflate them). See the
//! `DOMAIN_TBD` marker on `convert_property_definition_representation`
//! for the Plan 7+ IR Roadmap follow-up.

pub mod measure_representation_item;
pub mod property_definition;
