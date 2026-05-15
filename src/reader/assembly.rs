//! Assembly entity helpers that did not migrate into `entities/` modules.
//!
//! After Plan 6 the file holds two hand-rolled pieces only:
//!
//! - `convert_context_dependent_shape_representation` — Pass 6-7 needs
//!   `&EntityGraph` to resolve the RR-complex sub-entity, which the
//!   `EntityHandlerEntry::Simple` / `Complex` reader signatures do not
//!   carry. See the `DOMAIN_TBD` marker on the function for the
//!   Plan 7+ IR Roadmap follow-up.
//! - `pdef_shape_to_pdef_ref` / `pdef_shape_to_nauo_ref` —
//!   `PRODUCT_DEFINITION_SHAPE` is classified via a graph traversal
//!   from `passes.rs` (no `convert_*` body, so the handler trait is
//!   inappropriate). The classifier itself is the marker site in
//!   `passes.rs`.
//!
//! Everything else moved into `entities/assembly_product/` (PRODUCT
//! chain) and `entities/shape_rep/` (shape representations + IDT).

use super::ReaderContext;
use crate::ir::attr::{check_count, read_entity_ref};
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

    // DOMAIN_TBD: catalog shape_rep O but the read body needs &EntityGraph
    // (RR-complex sub-entity resolution). Handler trait API 보존을 위해
    // hand-rolled 유지. Plan 7+ IR Roadmap 시 graph 접근 일반화 검토.
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
}

/// Accepted entity-type names for the `definition` slot of a
/// `PRODUCT_DEFINITION_SHAPE` when classified as product-bearing.
/// `PRODUCT_DEFINITION` is the base type; `PRODUCT_DEFINITION_WITH_
/// ASSOCIATED_DOCUMENTS` is its AP203 / AP242 subtype that step-io's
/// reader treats identically (the extra `documentation_ids` attribute
/// is dropped at read).
const PDEF_TARGET_NAMES: &[&str] = &[
    "PRODUCT_DEFINITION",
    "PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS",
];

/// If `pdef_shape_ref` refers to a `PRODUCT_DEFINITION_SHAPE` whose
/// `definition` attribute targets a `PRODUCT_DEFINITION` (or any of its
/// supported subtypes), return that target's STEP id. Otherwise return
/// `None` (`NAUO`-tagged, missing, or malformed `PDEF_SHAPE`).
pub(super) fn pdef_shape_to_pdef_ref(
    graph: &crate::parser::entity::EntityGraph,
    pdef_shape_ref: u64,
) -> Option<u64> {
    pdef_shape_target_in(graph, pdef_shape_ref, PDEF_TARGET_NAMES)
}

/// Like [`pdef_shape_to_pdef_ref`] but for `NAUO`-tagged
/// `PRODUCT_DEFINITION_SHAPE`s. Returns the NAUO's STEP id.
pub(super) fn pdef_shape_to_nauo_ref(
    graph: &crate::parser::entity::EntityGraph,
    pdef_shape_ref: u64,
) -> Option<u64> {
    pdef_shape_target_in(graph, pdef_shape_ref, &["NEXT_ASSEMBLY_USAGE_OCCURRENCE"])
}

fn pdef_shape_target_in(
    graph: &crate::parser::entity::EntityGraph,
    pdef_shape_ref: u64,
    accepts: &[&str],
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
        RawEntity::Simple { name, .. } if accepts.iter().any(|t| *t == name) => Some(def_ref),
        _ => None,
    }
}
