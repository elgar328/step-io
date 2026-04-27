//! Assembly entity converters (Pass 6).
//!
//! Pass 6-1 ~ 6-5 (Phase A) walk `PRODUCT`, `PRODUCT_DEFINITION`,
//! `PRODUCT_DEFINITION_FORMATION` (incl. AP203's `_WITH_SPECIFIED_SOURCE`
//! variant), `ADVANCED_BREP_SHAPE_REPRESENTATION` and
//! `SHAPE_DEFINITION_REPRESENTATION` to populate `Arena<Product>` and
//! classify each product as a solid leaf or an empty group.
//!
//! Pass 6-6 ~ 6-8 (Phase B) wire up the tree:
//! - 6-6: `ITEM_DEFINED_TRANSFORMATION` → `Transform3d` map.
//! - 6-7: `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION` + complex `RRWT` →
//!   per-NAUO transform map.
//! - 6-8: `NEXT_ASSEMBLY_USAGE_OCCURRENCE` → `Instance` pushed into the
//!   parent product's `Group`.
//!
//! Root resolution lives in `ReaderContext::finalize_assembly`.

use super::ReaderContext;
use crate::ir::assembly::{Instance, ProductContent};
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph, RawEntity};

impl ReaderContext {
    // ------------------------------------------------------------------
    // Pass 6-7: CONTEXT_DEPENDENT_SHAPE_REPRESENTATION + complex RRWT
    // ------------------------------------------------------------------
    //
    // `CDSR(rr_complex_ref, pdef_shape_ref)`. The first attr points at a
    // complex entity bundling three parts — only the one that carries a
    // transformation ref (RRWT) is of interest here.

    pub(super) fn convert_context_dependent_shape_representation(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            2,
            entity_id,
            "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION",
        )?;
        let rr_ref = read_entity_ref(attrs, 0, entity_id, "representation_relation")?;
        let pdef_shape_ref = read_entity_ref(attrs, 1, entity_id, "represented_product_relation")?;

        // Only NAUO-tagged CDSRs — product-level CDSRs skip silently.
        let Some(&nauo_ref) = self.pdef_shape_to_nauo.get(&pdef_shape_ref) else {
            return Ok(());
        };

        // Look up the RR complex. Must have all three part types.
        let Some(RawEntity::Complex { parts, .. }) = graph.get(rr_ref) else {
            return Ok(());
        };
        if !super::has_all_parts(
            parts,
            &[
                "REPRESENTATION_RELATIONSHIP",
                "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
                "SHAPE_REPRESENTATION_RELATIONSHIP",
            ],
        ) {
            return Ok(());
        }
        let rrwt_attrs = super::require_part_attrs(
            parts,
            "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
            rr_ref,
        )?;
        let transform_ref = read_entity_ref(rrwt_attrs, 0, rr_ref, "transform_operator")?;
        let Some(&transform) = self.transform_map.get(&transform_ref) else {
            return Err(ConvertError::MissingReference {
                from: rr_ref,
                to: transform_ref,
                field_name: "transform_operator",
            });
        };
        self.nauo_transform_map.insert(nauo_ref, transform);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 6-8: NEXT_ASSEMBLY_USAGE_OCCURRENCE → Instance push
    // ------------------------------------------------------------------

    pub(super) fn convert_next_assembly_usage_occurrence(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 6, entity_id, "NEXT_ASSEMBLY_USAGE_OCCURRENCE")?;
        let occurrence_id = read_string_or_unset(attrs, 0, entity_id, "id")?.to_owned();
        let occurrence_name = read_string_or_unset(attrs, 1, entity_id, "name")?.to_owned();
        // attrs[2] = description, attrs[5] = reference_designator — ignored.
        let relating_pdef = read_entity_ref(attrs, 3, entity_id, "relating_pdef")?;
        let related_pdef = read_entity_ref(attrs, 4, entity_id, "related_pdef")?;

        let parent_pid = self.resolve_product_by_pdef(entity_id, relating_pdef, "relating_pdef")?;
        let child_pid = self.resolve_product_by_pdef(entity_id, related_pdef, "related_pdef")?;

        let Some(&transform) = self.nauo_transform_map.get(&entity_id) else {
            self.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: String::from("NEXT_ASSEMBLY_USAGE_OCCURRENCE with no transform found"),
            });
            return Ok(());
        };

        match &mut self.assembly_products[parent_pid].content {
            ProductContent::Group(instances) => {
                instances.push(Instance {
                    child: child_pid,
                    transform,
                    occurrence_id,
                    occurrence_name,
                });
            }
            ProductContent::Solid(_)
            | ProductContent::SurfaceBody(_)
            | ProductContent::Wireframe(_) => {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: String::from(
                        "NEXT_ASSEMBLY_USAGE_OCCURRENCE parent is a geometry leaf, not a Group",
                    ),
                });
            }
        }
        Ok(())
    }
}

/// If `pdef_shape_ref` refers to a `PRODUCT_DEFINITION_SHAPE` whose
/// `definition` attribute targets a `PRODUCT_DEFINITION`, return that
/// target's STEP id. Otherwise return `None` (`NAUO`-tagged, missing, or
/// malformed `PDEF_SHAPE`). Called from `passes.rs` during the pre-pass
/// that populates `pdef_shape_to_pdef`.
pub(super) fn pdef_shape_to_pdef_ref(
    graph: &crate::parser::entity::EntityGraph,
    pdef_shape_ref: u64,
) -> Option<u64> {
    pdef_shape_target(graph, pdef_shape_ref, "PRODUCT_DEFINITION")
}

/// Like [`pdef_shape_to_pdef_ref`] but for `NAUO`-tagged
/// `PRODUCT_DEFINITION_SHAPE`s. Returns the NAUO's STEP id.
pub(super) fn pdef_shape_to_nauo_ref(
    graph: &crate::parser::entity::EntityGraph,
    pdef_shape_ref: u64,
) -> Option<u64> {
    pdef_shape_target(graph, pdef_shape_ref, "NEXT_ASSEMBLY_USAGE_OCCURRENCE")
}

fn pdef_shape_target(
    graph: &crate::parser::entity::EntityGraph,
    pdef_shape_ref: u64,
    expected_target_type: &str,
) -> Option<u64> {
    let entity = graph.get(pdef_shape_ref)?;
    let attrs = match entity {
        RawEntity::Simple {
            name, attributes, ..
        } if name == "PRODUCT_DEFINITION_SHAPE" => attributes.as_slice(),
        _ => return None,
    };
    // PDEF_SHAPE attr[2] = definition (PRODUCT_DEFINITION or NAUO).
    let def_ref = match attrs.get(2) {
        Some(Attribute::EntityRef(n)) => *n,
        _ => return None,
    };
    match graph.get(def_ref)? {
        RawEntity::Simple { name, .. } if name == expected_target_type => Some(def_ref),
        _ => None,
    }
}
