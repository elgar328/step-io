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
use crate::ir::assembly::{
    Instance, ProductContent, Transform3d, WireframeContent, WireframeReprKind,
};
use crate::ir::attr::{
    check_count, read_entity_ref, read_entity_ref_list, read_string, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph, RawEntity};

impl ReaderContext {
    // ------------------------------------------------------------------
    // Pass 6-2: PRODUCT_DEFINITION_FORMATION [+ _WITH_SPECIFIED_SOURCE]
    // ------------------------------------------------------------------
    //
    // Both subtypes share attr[2] = PRODUCT ref. The AP203 variant has an
    // extra `source` enum; we ignore everything except the product ref.

    pub(super) fn convert_product_definition_formation(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        self.convert_product_definition_formation_inner(entity_id, attrs, false)
    }

    pub(super) fn convert_product_definition_formation_with_source(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        self.convert_product_definition_formation_inner(entity_id, attrs, true)
    }

    fn convert_product_definition_formation_inner(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
        with_source: bool,
    ) -> Result<(), ConvertError> {
        // At least 3 attrs for base type; the `_WITH_SPECIFIED_SOURCE`
        // subtype has 4 (extra `source` enum like `.NOT_KNOWN.`). The
        // source enum is informational only — the loyalty flag on Product
        // captures the choice between subtypes.
        if attrs.len() < 3 {
            return Err(ConvertError::AttributeCount {
                entity_id,
                entity_name: "PRODUCT_DEFINITION_FORMATION".into(),
                expected: 3,
                actual: attrs.len(),
            });
        }
        let _id = read_string_or_unset(attrs, 0, entity_id, "id")?;
        let _description = read_string_or_unset(attrs, 1, entity_id, "description")?;
        let product_ref = read_entity_ref(attrs, 2, entity_id, "of_product")?;
        self.formation_to_product.insert(entity_id, product_ref);
        if with_source {
            if let Some(&pid) = self.product_arena_map.get(&product_ref) {
                self.assembly_products[pid].formation_with_source = true;
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 6-3: PRODUCT_DEFINITION
    // ------------------------------------------------------------------

    pub(super) fn convert_product_definition(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "PRODUCT_DEFINITION")?;
        // PRODUCT_DEFINITION.id is a fixed-value enum-like string ("design" /
        // "implementation"); keep strict. description is informal — accept `$`.
        let _id = read_string(attrs, 0, entity_id, "id")?;
        let _description = read_string_or_unset(attrs, 1, entity_id, "description")?;
        let formation_ref = read_entity_ref(attrs, 2, entity_id, "formation")?;
        // attrs[3] = frame_of_reference (PRODUCT_DEFINITION_CONTEXT) — ignored.

        let Some(&product_ref) = self.formation_to_product.get(&formation_ref) else {
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: formation_ref,
                field_name: "formation",
            });
        };
        self.pdef_to_product.insert(entity_id, product_ref);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 6-4: ADVANCED_BREP_SHAPE_REPRESENTATION
    // ------------------------------------------------------------------
    //
    // `ABSR('', items, context)` — `items` is a list of `REPRESENTATION_ITEM`
    // refs (axis placements, solids, …). We pick the first ref that is a
    // MANIFOLD_SOLID_BREP (i.e. present in `solid_map`) and bind it to the
    // ABSR id.

    pub(super) fn convert_advanced_brep_shape_representation(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "ADVANCED_BREP_SHAPE_REPRESENTATION")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let items = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        if let Some(&ctx_id) = self.context_id_map.get(&ctx_ref) {
            self.repr_context_map.insert(entity_id, ctx_id);
        }

        let solid_refs: Vec<u64> = items
            .iter()
            .filter(|r| self.solid_map.contains_key(r))
            .copied()
            .collect();
        // Pick the first AXIS2_PLACEMENT_3D in the items list as the coordinate
        // reference frame. In practice commercial CAD output places it first.
        if let Some(&placement_id) = items.iter().find_map(|r| self.placement_map.get(r)) {
            self.absr_ref_frame_map.insert(entity_id, placement_id);
        }
        match solid_refs.as_slice() {
            [] => {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: String::from(
                        "ADVANCED_BREP_SHAPE_REPRESENTATION without a MANIFOLD_SOLID_BREP item",
                    ),
                });
                Ok(())
            }
            [solid_ref, ..] => {
                if solid_refs.len() > 1 {
                    self.warnings.push(ConvertError::UnexpectedEntityForm {
                        entity_id,
                        detail: format!(
                            "ADVANCED_BREP_SHAPE_REPRESENTATION has {} MANIFOLD_SOLID_BREP items, using the first",
                            solid_refs.len()
                        ),
                    });
                }
                let solid_id = self.solid_map[solid_ref];
                self.absr_solid_map.insert(entity_id, solid_id);
                Ok(())
            }
        }
    }

    // ------------------------------------------------------------------
    // Pass 6-4b: SHELL_BASED_SURFACE_MODEL
    // ------------------------------------------------------------------
    //
    // `SBSM('', sbsm_boundary)` — `sbsm_boundary` is a list of `OPEN_SHELL`
    // / `CLOSED_SHELL` refs. We resolve each to a `ShellId` and store the
    // list for later MSSR flattening.

    pub(super) fn convert_shell_based_surface_model(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SHELL_BASED_SURFACE_MODEL")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let shell_refs = read_entity_ref_list(attrs, 1, entity_id, "sbsm_boundary")?;
        let shells: Vec<crate::ir::id::ShellId> = shell_refs
            .iter()
            .filter_map(|r| self.shell_map.get(r).copied())
            .collect();
        self.sbsm_shells_map.insert(entity_id, shells);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 6-4c: MANIFOLD_SURFACE_SHAPE_REPRESENTATION
    // ------------------------------------------------------------------
    //
    // `MSSR('', items, context)` — analogous to ABSR but with a
    // `SHELL_BASED_SURFACE_MODEL` in place of a `MANIFOLD_SOLID_BREP`.
    // An optional `AXIS2_PLACEMENT_3D` may appear in `items` as the
    // coordinate reference frame.

    pub(super) fn convert_manifold_surface_shape_representation(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let items = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        if let Some(&ctx_id) = self.context_id_map.get(&ctx_ref) {
            self.repr_context_map.insert(entity_id, ctx_id);
        }

        if let Some(&placement_id) = items.iter().find_map(|r| self.placement_map.get(r)) {
            self.mssr_ref_frame_map.insert(entity_id, placement_id);
        }

        let flattened: Vec<crate::ir::id::ShellId> = items
            .iter()
            .filter_map(|r| self.sbsm_shells_map.get(r))
            .flat_map(|shells| shells.iter().copied())
            .collect();
        self.mssr_shells_map.insert(entity_id, flattened);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 6-4f: GEOMETRIC_CURVE_SET / GEOMETRIC_SET
    // ------------------------------------------------------------------
    //
    // Both names share the same EXPRESS shape; `GEOMETRIC_CURVE_SET` is a
    // subtype restricting `items` to curves, while `GEOMETRIC_SET` allows
    // points and (rarely) surfaces too. We split the items into two
    // buckets — `curves` and `points` — using the populated curve / point
    // maps. Items that resolve to neither (e.g. a stray surface ref) are
    // silently skipped.

    pub(super) fn convert_geometric_curve_set(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "GEOMETRIC_CURVE_SET")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let mut curves = Vec::new();
        let mut points = Vec::new();
        for r in item_refs {
            if let Some(&cid) = self.curve_map.get(&r) {
                curves.push(cid);
            } else if let Some(&pid) = self.point_map.get(&r) {
                points.push(pid);
            }
        }
        self.curve_set_map.insert(entity_id, (curves, points));
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 6-4g: GEOMETRICALLY_BOUNDED_(WIREFRAME|SURFACE)_SHAPE_REPRESENTATION
    // ------------------------------------------------------------------
    //
    // Both wrappers share the same items shape: an axis placement (often
    // omitted by CATIA in the SURFACE flavour) plus one or more
    // GEOMETRIC_(CURVE_)SETs. We collapse the curve sets into a single
    // `WireframeContent` and remember the axis frame separately so the
    // SDR pass can populate `Product.shape_ref_frame`. The `repr_kind`
    // flag preserves which wrapper the source used.

    pub(super) fn convert_gbwsr(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        self.convert_wireframe_representation(entity_id, attrs, WireframeReprKind::Wireframe)
    }

    pub(super) fn convert_gbssr(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        self.convert_wireframe_representation(entity_id, attrs, WireframeReprKind::Surface)
    }

    fn convert_wireframe_representation(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
        repr_kind: WireframeReprKind,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            3,
            entity_id,
            "GEOMETRICALLY_BOUNDED_*_SHAPE_REPRESENTATION",
        )?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let items = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        if let Some(&ctx_id) = self.context_id_map.get(&ctx_ref) {
            self.repr_context_map.insert(entity_id, ctx_id);
        }

        if let Some(&placement_id) = items.iter().find_map(|r| self.placement_map.get(r)) {
            self.wireframe_ref_frame_map.insert(entity_id, placement_id);
        }
        let mut curves = Vec::new();
        let mut points = Vec::new();
        for r in &items {
            if let Some((c, p)) = self.curve_set_map.get(r) {
                curves.extend_from_slice(c);
                points.extend_from_slice(p);
            } else if let Some(&cid) = self.curve_map.get(r) {
                // Some producers attach curves directly without a wrapping
                // GEOMETRIC_CURVE_SET — accept that form too.
                curves.push(cid);
            }
        }
        self.wireframe_data_map.insert(
            entity_id,
            WireframeContent {
                curves,
                points,
                repr_kind,
            },
        );
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 6-4e: plain SHAPE_REPRESENTATION
    // ------------------------------------------------------------------
    //
    // `SHAPE_REPRESENTATION(name, items, context)`. Captures the first
    // `AXIS2_PLACEMENT_3D` from `items` so the SDR pass can attach it to
    // `Product.outer_sr_frame` when the Fusion 360 / CATIA indirection chain
    // (`SDR → plain SR → SRR → ABSR/MSSR`) is taken. Plain SRs that are not
    // part of an indirection chain (e.g. Group-product SRs) populate the map
    // harmlessly — entries are looked up only when SDR detects indirection.
    //
    // Subtypes (`ADVANCED_BREP_SHAPE_REPRESENTATION`,
    // `MANIFOLD_SURFACE_SHAPE_REPRESENTATION`) keep their own dispatch entries;
    // `run_pass!` exact-matches entity names so subtypes never reach this
    // converter.

    pub(super) fn convert_plain_shape_representation(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SHAPE_REPRESENTATION")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let items = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        if let Some(&ctx_id) = self.context_id_map.get(&ctx_ref) {
            self.repr_context_map.insert(entity_id, ctx_id);
        }

        if let Some(&placement_id) = items.iter().find_map(|r| self.placement_map.get(r)) {
            self.plain_sr_frame_map.insert(entity_id, placement_id);
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 6-4d: simple SHAPE_REPRESENTATION_RELATIONSHIP
    // ------------------------------------------------------------------
    //
    // `SRR(name, description, rep_1, rep_2)` — when one side resolves to a
    // known ABSR / MSSR, record the other (typically a plain SR) as
    // equivalent. SDR conversion later follows this mapping to reach the
    // geometry-carrying representation through the indirection
    // `SDR → plain SR → SRR → ABSR / MSSR` used by Fusion 360 and some
    // CATIA exports. Complex `_WITH_TRANSFORMATION` variants are consumed
    // by the CDSR / RRWT path instead.

    pub(super) fn convert_shape_representation_relationship(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "SHAPE_REPRESENTATION_RELATIONSHIP")?;
        // attrs[0] = name, attrs[1] = description — ignored.
        let rep_1 = read_entity_ref(attrs, 2, entity_id, "rep_1")?;
        let rep_2 = read_entity_ref(attrs, 3, entity_id, "rep_2")?;

        let r1_target = self.absr_solid_map.contains_key(&rep_1)
            || self.mssr_shells_map.contains_key(&rep_1)
            || self.wireframe_data_map.contains_key(&rep_1);
        let r2_target = self.absr_solid_map.contains_key(&rep_2)
            || self.mssr_shells_map.contains_key(&rep_2)
            || self.wireframe_data_map.contains_key(&rep_2);
        match (r1_target, r2_target) {
            (true, false) => {
                self.srr_equiv_map.insert(rep_2, rep_1);
            }
            (false, true) => {
                self.srr_equiv_map.insert(rep_1, rep_2);
            }
            (true, true) => {
                // Both sides are geometry-carrying reps (e.g. ABSR↔MSSR direct
                // relation). SDR can reach either directly — no indirection
                // needed. Silently skip.
            }
            (false, false) => {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "SHAPE_REPRESENTATION_RELATIONSHIP #{entity_id} connects two non-target representations — multi-hop SRR chains are not supported"
                    ),
                });
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 6-5: SHAPE_DEFINITION_REPRESENTATION
    // ------------------------------------------------------------------
    //
    // `SDR(pdef_shape, shape_rep)`. We care only about SDRs whose
    // `pdef_shape` points at a `PRODUCT_DEFINITION` (product-describing);
    // SDRs on NAUOs ("Placement of an item") belong to Phase B.
    //
    // If the referenced `shape_rep` is an ABSR we know the product is a
    // geometry leaf and set `content = Solid(...)`. Otherwise (e.g. a plain
    // `SHAPE_REPRESENTATION` holding only axis placements) the product is
    // an assembly/wrapper and keeps `Group(empty)`.

    pub(super) fn convert_shape_definition_representation(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SHAPE_DEFINITION_REPRESENTATION")?;
        let pdef_shape_ref = read_entity_ref(attrs, 0, entity_id, "definition")?;
        let shape_rep_ref = read_entity_ref(attrs, 1, entity_id, "used_representation")?;

        // Only consider SDRs where `pdef_shape.definition` is a
        // PRODUCT_DEFINITION. NAUO-tagged SDRs fall through to Phase B.
        //
        // The passes layer pre-computes `pdef_shape_ref → PRODUCT_DEFINITION
        // step id` into `pdef_to_product`'s lookup chain; if the pdef_shape
        // doesn't target a PRODUCT_DEFINITION we exit cleanly.
        let Some(pdef_ref) = self.pdef_shape_to_pdef.get(&pdef_shape_ref).copied() else {
            return Ok(());
        };
        let Some(&product_step_id) = self.pdef_to_product.get(&pdef_ref) else {
            // Unresolved PDEF — PDEF_SHAPE points at a PDEF we haven't
            // mapped. Warn and move on.
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: pdef_ref,
                field_name: "definition.definition",
            });
        };
        let Some(&pid) = self.product_arena_map.get(&product_step_id) else {
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: product_step_id,
                field_name: "definition.product",
            });
        };

        // Guard against a second SDR pinning the same product: keep the
        // first classification and warn on the duplicate.
        match &self.assembly_products[pid].content {
            ProductContent::Solid(_)
            | ProductContent::SurfaceBody(_)
            | ProductContent::Wireframe(_) => {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "duplicate SHAPE_DEFINITION_REPRESENTATION for product #{product_step_id}"
                    ),
                });
                return Ok(());
            }
            ProductContent::Group(instances) if !instances.is_empty() => {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "duplicate SHAPE_DEFINITION_REPRESENTATION for product #{product_step_id}"
                    ),
                });
                return Ok(());
            }
            ProductContent::Group(_) => {}
        }

        // Follow the Fusion 360 / CATIA indirection:
        //   SDR → plain SR → SHAPE_REPRESENTATION_RELATIONSHIP → ABSR / MSSR
        // Direct references fall through the map untouched.
        let effective_ref = self
            .srr_equiv_map
            .get(&shape_rep_ref)
            .copied()
            .unwrap_or(shape_rep_ref);

        // When the indirection chain was taken, attach the plain SR's frame
        // so the writer can re-emit the wrapper. `plain_sr_frame_map` may be
        // missing the entry if the plain SR had no axis item — in that case
        // outer_sr_frame stays None and the writer emits the direct form.
        if effective_ref != shape_rep_ref {
            if let Some(&plain_frame) = self.plain_sr_frame_map.get(&shape_rep_ref) {
                self.assembly_products[pid].outer_sr_frame = Some(plain_frame);
            }
        }

        let in_absr = self.absr_solid_map.contains_key(&effective_ref);
        let in_mssr = self.mssr_shells_map.contains_key(&effective_ref);
        if in_absr && in_mssr {
            self.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "shape_rep #{effective_ref} registered as both ABSR and MSSR — using ABSR"
                ),
            });
        }
        if let Some(&solid_id) = self.absr_solid_map.get(&effective_ref) {
            self.assembly_products[pid].content = ProductContent::Solid(solid_id);
        } else if let Some(shells) = self.mssr_shells_map.get(&effective_ref).cloned() {
            if shells.is_empty() {
                self.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "MSSR #{effective_ref} has no shells — product content left empty"
                    ),
                });
            } else {
                self.assembly_products[pid].content = ProductContent::SurfaceBody(shells);
            }
        } else if let Some(wf) = self.wireframe_data_map.get(&effective_ref).cloned() {
            self.assembly_products[pid].content = ProductContent::Wireframe(wf);
        }
        if let Some(&ref_frame) = self.absr_ref_frame_map.get(&effective_ref) {
            self.assembly_products[pid].shape_ref_frame = ref_frame;
        } else if let Some(&ref_frame) = self.mssr_ref_frame_map.get(&effective_ref) {
            self.assembly_products[pid].shape_ref_frame = ref_frame;
        } else if let Some(&ref_frame) = self.wireframe_ref_frame_map.get(&effective_ref) {
            self.assembly_products[pid].shape_ref_frame = ref_frame;
        }
        // Attach the unit / uncertainty context referenced by this product's
        // inner shape representation. Look up by the resolved ABSR/MSSR/etc.
        // entity, not the outer plain SR — the inner ctx is the geometry-side
        // one that downstream tooling cares about.
        if let Some(&ctx_id) = self.repr_context_map.get(&effective_ref) {
            self.assembly_products[pid].geometry_context = Some(ctx_id);
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 6-6: ITEM_DEFINED_TRANSFORMATION
    // ------------------------------------------------------------------

    pub(super) fn convert_item_defined_transformation(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "ITEM_DEFINED_TRANSFORMATION")?;
        // attrs[0] = name, attrs[1] = description — ignored.
        let source_ref = read_entity_ref(attrs, 2, entity_id, "transform_item_1")?;
        let target_ref = read_entity_ref(attrs, 3, entity_id, "transform_item_2")?;
        let source = self.resolve_placement(entity_id, source_ref, "transform_item_1")?;
        let target = self.resolve_placement(entity_id, target_ref, "transform_item_2")?;
        self.transform_map
            .insert(entity_id, Transform3d { source, target });
        Ok(())
    }

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
