//! Product chain and assembly tree emission.
//!
//! The single entry point `emit_product_chain_if_eligible` drives emission of
//! the entire product / assembly section. It walks every `Product` in the IR
//! and emits the surrounding PRODUCT / PRODUCT_DEFINITION chain, SR or ABSR
//! plus SDR for each. Then, for every `Group` product, it emits the per-
//! `Instance` NAUO + ITEM_DEFINED_TRANSFORMATION + complex RRWT + CDSR bundle.
//!
//! Single-part files (one Solid product) flow through the same code path:
//! the product loop runs once, and the instance loop finds no Groups so it
//! emits nothing extra — output matches the W-B.1 single-part format.

#![allow(clippy::doc_markdown)]

use std::collections::HashMap;

use super::WriteBuffer;
use crate::ir::PropertyDefinitionId;
use crate::ir::arena::Arena;
use crate::ir::representation_item::RepresentationItemRef;
use crate::ir::shape_rep::Representation;
use crate::ir::{
    GeometryLeaf, Instance, Product, ProductCategoryId, ProductId, Transform3d, WireframeContent,
    WireframeReprKind,
};
use crate::parser::entity::Attribute;
use crate::parser::schema::{SchemaClass, StepSchema};
use crate::writer::WriteError;
use crate::writer::entity::{WriterBody, WriterEntity};

/// Context entity ids reused by every product in a given file. The
/// APPLICATION_CONTEXT id itself is only needed inside
/// `emit_application_context` (to wire PRODUCT_CONTEXT / PDEF_CONTEXT), so
/// it isn't carried forward here.
#[derive(Clone, Copy)]
pub(crate) struct AssemblyContextIds {
    pub(crate) product_ctx: u64,
    pub(crate) pdef_ctx: u64,
}

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_product_chain_if_eligible(
        &mut self,
    ) -> Result<(), WriteError> {
        // No emitted contexts → silent skip. Kernel-built IR with no units
        // and no `Product.geometry_context` falls in here; the alternative
        // (synthesising a default context) breaks round-trip equality
        // because the synthesised entries reappear on re-read while the
        // original IR has none.
        if self.unit_context_ids.is_empty() {
            return Ok(());
        }
        let Some(assembly) = self.model.assembly.as_ref() else {
            return Ok(());
        };
        if assembly.products.is_empty() {
            return Ok(());
        }
        // Clone out of `self.model` so the pass is free to call `&mut self`
        // methods without keeping a borrow on the model.
        let schema = self.model.metadata.schema.clone();
        let products = assembly.products.clone();
        self.emit_assembly_chain(&products, &schema)?;
        self.emit_pdca_cluster();
        self.emit_pdr_cluster();
        Ok(())
    }

    /// Emit `PRODUCT_DEFINITION_RELATIONSHIP` + `MAKE_FROM_USAGE_OPTION`
    /// arena entries after the assembly chain has populated
    /// `product_def_ids` and units pass has populated `mwu_step_ids`.
    fn emit_pdr_cluster(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::assembly_product::make_from_usage_option::{
            MakeFromUsageOptionHandler, MakeFromUsageOptionWriteInput,
        };
        use crate::entities::assembly_product::product_definition_relationship::{
            ProductDefinitionRelationshipHandler, ProductDefinitionRelationshipWriteInput,
        };
        use crate::ir::ProductDefinitionRelationship;

        let Some(assembly) = self.model.assembly.clone() else {
            return;
        };
        for entry in assembly.product_definition_relationships.iter() {
            match entry {
                ProductDefinitionRelationship::Plain(plain) => {
                    let (Some(&relating_step), Some(&related_step)) = (
                        self.product_def_ids.get(&plain.relating),
                        self.product_def_ids.get(&plain.related),
                    ) else {
                        continue;
                    };
                    let _ = ProductDefinitionRelationshipHandler::write(
                        self,
                        ProductDefinitionRelationshipWriteInput {
                            plain: plain.clone(),
                            relating_pdef_step: relating_step,
                            related_pdef_step: related_step,
                        },
                    );
                }
                ProductDefinitionRelationship::MakeFrom(mfu) => {
                    let (Some(&relating_step), Some(&related_step)) = (
                        self.product_def_ids.get(&mfu.relating),
                        self.product_def_ids.get(&mfu.related),
                    ) else {
                        continue;
                    };
                    let quantity_step = self.step_id(mfu.quantity);
                    let _ = MakeFromUsageOptionHandler::write(
                        self,
                        MakeFromUsageOptionWriteInput {
                            mfu: mfu.clone(),
                            relating_pdef_step: relating_step,
                            related_pdef_step: related_step,
                            quantity_step,
                        },
                    );
                }
            }
        }
    }

    /// Emit `PRODUCT_DEFINITION_CONTEXT_ROLE` + `PRODUCT_DEFINITION_CONTEXT_ASSOCIATION`
    /// after the assembly chain has been written. Requires
    /// `product_def_ids` (populated by `emit_assembly_chain`) and
    /// `pdc_step_ids` (populated by `emit_application_context`).
    fn emit_pdca_cluster(&mut self) {
        let Some(assembly) = self.model.assembly.clone() else {
            return;
        };
        let mut pdcr_step_ids: Vec<u64> =
            Vec::with_capacity(assembly.product_definition_context_roles.len());
        for r in assembly.product_definition_context_roles.iter() {
            let desc_attr = match &r.description {
                Some(d) => Attribute::String(d.clone()),
                None => Attribute::Unset,
            };
            let id = self.push_simple(
                "PRODUCT_DEFINITION_CONTEXT_ROLE",
                vec![Attribute::String(r.name.clone()), desc_attr],
            );
            pdcr_step_ids.push(id);
        }
        for a in assembly.product_definition_context_associations.iter() {
            let Some(&pdef_step) = self.product_def_ids.get(&a.definition) else {
                continue;
            };
            let pdc_step = self.pdc_step_ids[a.frame_of_reference.0 as usize];
            let role_step = pdcr_step_ids[a.role.0 as usize];
            let _ = self.push_simple(
                "PRODUCT_DEFINITION_CONTEXT_ASSOCIATION",
                vec![
                    Attribute::EntityRef(pdef_step),
                    Attribute::EntityRef(pdc_step),
                    Attribute::EntityRef(role_step),
                ],
            );
        }
    }

    /// Resolve a product's `geometry_context` to a STEP entity id, or
    /// `None` when the IR has no explicit context for this product
    /// (metadata-only product with no SHAPE_DEFINITION_REPRESENTATION
    /// in source). Callers treat `None` as "skip SR + SDR emission so
    /// round-trip preserves the absence of a shape representation".
    fn resolve_product_ctx(&self, product: &Product) -> Option<u64> {
        let id = product.geometry_context?;
        Some(self.unit_context_ids[id.0 as usize])
    }

    #[allow(clippy::too_many_lines)] // sequential per-product chain emit
    fn emit_assembly_chain(
        &mut self,
        products: &Arena<Product>,
        schema: &StepSchema,
    ) -> Result<(), WriteError> {
        let fallback_ctx = self.emit_application_context(schema);

        // Emit PRODUCT chain + shape representation + SDR for every product;
        // collect each product's PDEF and SR entity ids for later instance
        // wiring. PDEF ids land in `self.product_def_ids` so the property
        // emitter (which runs after this pass) can resolve a
        // `Property.target` ProductId to a STEP ref.
        let mut product_sr: HashMap<ProductId, u64> = HashMap::new();

        for (pid, product) in products.iter_with_ids() {
            // Per-product context: use the IR-recorded PC/PDC when present
            // (multi-product AP203 corpus), otherwise fall back to the
            // shared synthesised context.
            let product_ctx = product
                .product_context
                .map_or(fallback_ctx.product_ctx, |id| {
                    self.pc_step_ids[id.0 as usize]
                });
            let pdef_ctx_step = product
                .pdef_context
                .map_or(fallback_ctx.pdef_ctx, |id| self.pdc_step_ids[id.0 as usize]);
            let per_product_ctx = AssemblyContextIds {
                product_ctx,
                pdef_ctx: pdef_ctx_step,
            };
            let prod_entity = self.emit_product(product, &per_product_ctx);
            self.product_step_ids.insert(pid, prod_entity);
            // Document-style products — `pdef_context = None && geometry_context
            // = None`, typically `category.kind = "document"` observed in NIST
            // AP242 PMI fixtures. Source had only the bare PRODUCT entity (no
            // FORMATION / PDEF / SDR), so skip emitting the synthetic chain;
            // otherwise round-trip would re-introduce a PDEF and break IR
            // idempotency (`pdef_context` goes from `None` → `Some(PDC[0])`).
            //
            // plm SELECT items that target this product still need a step ref,
            // so record the PRODUCT step id in `product_def_ids` — the lookup
            // map's value is the "best ref" for this product (PDEF when the
            // chain exists, PRODUCT otherwise). Reader-side symmetry is via
            // `entities/plm/mod.rs::resolve_date_time_item` chain 3, which
            // walks `product_arena_map` for direct PRODUCT refs.
            //
            // Kernel-built IR keeps `geometry_context = Some(_)` (the adapter
            // sets it when constructing geometry) so it bypasses this branch
            // and gets the full chain as before.
            if product.pdef_context.is_none() && product.geometry_context.is_none() {
                // Document-style product: no PDEF/SDR chain. But if the source
                // gave it a formation (e.g. an ASME-standard reference linked
                // via DOCUMENT_PRODUCT_EQUIVALENCE), emit that formation so it
                // round-trips. `product_def_ids` stays the PRODUCT ref (the PD
                // chain is still absent — keeps the pdef_context None→None
                // idempotency); DPE reaches the formation via its own cache.
                if product.formation.is_some() {
                    let _ = self.emit_formation(prod_entity, product);
                }
                self.product_def_ids.insert(pid, prod_entity);
                continue;
            }
            let formation = self.emit_formation(prod_entity, product);
            // Resolve documentation_ids (DocumentId -> DOCUMENT step id, filled
            // by the document prepass that runs before the product chain) so
            // the WITH_ASSOCIATED_DOCUMENTS subtype round-trips when present.
            let doc_refs: Vec<u64> = product
                .associated_documents
                .iter()
                .map(|d| self.step_id(d))
                .collect();
            // Reader-built products carry the canonical PD id/description in the
            // arena (the legacy synthesis hardcoded "design"/""). Kernel-built
            // products (`pdef = None`) keep the synthesized values.
            let (pd_id, pd_desc) = match product.pdef {
                Some(pdid) => {
                    let pd = &self
                        .model
                        .assembly
                        .as_ref()
                        .expect("assembly present when Product.pdef is set")
                        .product_definitions[pdid];
                    (pd.id.clone(), pd.description.clone())
                }
                None => ("design".to_owned(), String::new()),
            };
            let pdef = self.emit_pdef(
                &pd_id,
                &pd_desc,
                formation,
                per_product_ctx.pdef_ctx,
                &doc_refs,
            );
            self.product_def_ids.insert(pid, pdef);
        }
        // Now that every product's PDEF step id is cached in
        // `product_def_ids`, emit the `property_definitions` arena —
        // arena-driven PD/PDS emit fills `product_def_shape_ids` for the
        // second loop's SR/SDR emit (and the PMI consumer cluster runs
        // even later in `emit_all`, so its SHAPE_ASPECT.of_shape refs
        // also see filled slots). The arena's source-#N order is
        // preserved across the two loops because the writer never
        // interleaves user property emits between PDS arena entries.
        self.emit_property_definitions_pds_only();

        // Second loop: SR + SDR for every geometry-bearing product.
        // Document-style products (`pdef_context = None && geometry_context
        // = None`) already short-circuited above; metadata-only products
        // (`geometry_context = None`) skip SR/SDR here for source-mirror.
        for (pid, product) in products.iter_with_ids() {
            if product.pdef_context.is_none() && product.geometry_context.is_none() {
                continue;
            }
            let Some(unit_ctx) = self.resolve_product_ctx(product) else {
                continue;
            };
            let sr = if let Some(rep_id) = product.representation_id {
                // Reader-built IR: the representation was pre-emitted in
                // arena order by `emit_representations_pre_pass`. Reuse the
                // cached step id rather than re-emitting inline.
                let geo_sr = self.step_id(rep_id);
                match product.outer_representation_id {
                    // Fusion 360 / CATIA indirect form: the outer plain SR
                    // wrapper is a separate pre-emitted arena entry. The
                    // linking SHAPE_REPRESENTATION_RELATIONSHIP is now
                    // emitted from the `representation_relationships`
                    // arena (phase srr-unify-b), so we only need to point
                    // the SDR at the outer SR here.
                    Some(outer_id) => self.step_id(outer_id),
                    None => geo_sr,
                }
            } else {
                // Hand/kernel-built IR: the `representations` arena is
                // unpopulated, so emit the representation inline from
                // `ProductContent` as before.
                match &product.geometry {
                    Some(GeometryLeaf::Solid(solid)) => {
                        let solid_refs: Vec<u64> = solid
                            .ids
                            .iter()
                            .map(|sid| self.emit_solid(*sid))
                            .collect::<Result<_, _>>()?;
                        let absr = self.emit_absr(product, solid_refs, unit_ctx)?;
                        self.wrap_indirect_sr_if_set(product, absr, unit_ctx)?
                    }
                    Some(GeometryLeaf::SurfaceBody(sbody)) => {
                        let mssr = self.emit_mssr(product, &sbody.ids, unit_ctx)?;
                        self.wrap_indirect_sr_if_set(product, mssr, unit_ctx)?
                    }
                    Some(GeometryLeaf::Wireframe(wf)) => {
                        let gb_sr = self.emit_wireframe_representation(product, wf, unit_ctx)?;
                        self.wrap_indirect_sr_if_set(product, gb_sr, unit_ctx)?
                    }
                    None => self.emit_group_shape_representation(product, unit_ctx)?,
                }
            };
            product_sr.insert(pid, sr);
            // PDS was emitted by `emit_property_definitions_if_set`; fetch
            // its cached step id (every geometry-bearing product has one
            // mirrored from the assembly chain — see
            // `product_definition_shape::read`).
            let Some(&pdef_shape) = self.product_def_shape_ids.get(&pid) else {
                continue;
            };
            self.emit_sdr(pdef_shape, sr);
        }

        // Emit per-instance bundles for every Group product.
        for (parent_pid, parent) in products.iter_with_ids() {
            if parent.instances.is_empty() {
                continue;
            }
            // Document-style Group products skip the PDEF chain above, so
            // their entry in `product_def_ids` is absent. Such products
            // never have instances; skip them here for symmetry.
            let Some(&parent_pdef) = self.product_def_ids.get(&parent_pid) else {
                continue;
            };
            // No SR for this parent → no shape to anchor instances on.
            // Reached when the parent is a metadata-only Group([]) whose
            // SR emission was skipped above.
            let Some(&parent_sr) = product_sr.get(&parent_pid) else {
                continue;
            };
            // Clone product_def_ids for emit_instance_bundle so it can
            // borrow `&self` via emit_* methods without overlapping borrows.
            let product_def = self.product_def_ids.clone();
            for inst in &parent.instances {
                if !product_sr.contains_key(&inst.child) {
                    // Child product has no SR (geometry_context: None
                    // — metadata-only). A CDSR/RRWT bundle can't point
                    // into nothing; surface this instead of panicking
                    // on the missing map entry.
                    return Err(WriteError::DanglingId {
                        detail: format!(
                            "Instance child ProductId({}) has no SR (geometry_context: None)",
                            inst.child.0
                        ),
                    });
                }
                self.emit_instance_bundle(inst, parent_pdef, parent_sr, &product_def, &product_sr)?;
            }
        }

        Ok(())
    }

    // ----------------------------------------------------------------
    // Representation pre-emit
    // ----------------------------------------------------------------

    /// Emit every geometry representation in `representations` arena order,
    /// filling `representation_step_ids` so the product chain can resolve
    /// each `RepresentationId` to a cached step id. `MDGPR` entries are
    /// skipped — they depend on `STYLED_ITEM`s and are emitted later by
    /// `emit_visualization_if_set`, which appends their slots. The arena is
    /// `[geometry reprs, MDGPR]`, so the geometry reprs
    /// form a contiguous prefix and the appended `MDGPR` slots stay aligned
    /// with their `RepresentationId`s.
    /// Emit the `unitless_contexts` arena (phase unitless-context) as
    /// `(GRC PRC REP_CONTEXT)` complex MI entities. Called from
    /// `emit_pools` after `emit_units_pool_if_set` so the unit-bearing
    /// GUAC entities are already emitted before the unitless variants.
    pub(in crate::writer::buffer) fn emit_unitless_contexts(&mut self) -> Result<(), WriteError> {
        use crate::entities::shape_rep::parametric_representation_context::ParametricRepresentationContextHandler;
        use crate::entities::shape_rep::representation_context::RepresentationContextHandler;
        use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
        let entries: Vec<_> = self
            .model
            .shape_rep
            .unitless_contexts
            .iter_with_ids()
            .map(|(__id, v)| (__id, v.clone()))
            .collect();
        for (__id, uc) in entries {
            // `Some(dim)` → GRC+PRC+RC complex; `None` → plain simple
            // REPRESENTATION_CONTEXT.
            let step_id = if uc.coordinate_space_dimension.is_some() {
                ParametricRepresentationContextHandler::write(self, uc)?
            } else {
                RepresentationContextHandler::write(self, uc)?
            };
            self.set_step_id(__id, step_id);
        }
        Ok(())
    }

    /// Emit the `Representation::DraughtingModel` arena entries (phase
    /// draughting-model). Called from `emit_pools` after the visualization
    /// / annotation / callout passes so every `items` ref cache
    /// (styled_item / ao / draughting_callout / representation_item /
    /// per-geometry placement) is populated. Appends to
    /// `representation_step_ids` in arena id order — the MDGPR entries
    /// have already been pushed by the visualization pass and the
    /// DraughtingModel entries follow them.
    pub(in crate::writer::buffer) fn emit_draughting_models(&mut self) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::draughting_model::DraughtingModelHandler;
        self.assert_phase(
            super::EmitPhase::Annotations,
            "emit_draughting_models (indexes annotation_occurrence/draughting_callout step ids)",
        );
        let reprs = self.model.shape_rep.representations.clone();
        for (id, repr) in reprs.iter_with_ids() {
            if let Representation::DraughtingModel(dm) = repr {
                let step_id = DraughtingModelHandler::write(self, dm.clone())?;
                self.set_step_id(id, step_id);
            }
        }
        Ok(())
    }

    /// Emit the `Representation::TessellatedShapeRepresentation` arena
    /// entries (phase tsr). Called from `emit_pools` after
    /// `emit_draughting_models` so `representation_step_ids` is appended
    /// in arena order (Mdgpr → DM → TSR — all three delayed-emit
    /// variants sit at the arena tail by reader pass scheduling).
    /// TSR `items` resolve through `emit_tessellated_item_ref`, which
    /// reads the tessellation step-id caches that `emit_tessellation`
    /// (called earlier in `emit_pools`) populated.
    pub(in crate::writer::buffer) fn emit_tessellated_shape_representations(
        &mut self,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::tessellated_shape_representation::TessellatedShapeRepresentationHandler;
        let reprs = self.model.shape_rep.representations.clone();
        for (id, repr) in reprs.iter_with_ids() {
            if let Representation::TessellatedShapeRepresentation(tsr) = repr {
                let step_id = TessellatedShapeRepresentationHandler::write(self, tsr.clone())?;
                self.set_step_id(id, step_id);
            }
        }
        Ok(())
    }

    /// Emit the `Representation::ConstructiveGeometry` arena entries
    /// (phase cgr). Called from `emit_pools` after
    /// `emit_tessellated_shape_representations` so
    /// `representation_step_ids` is appended in arena order
    /// (Mdgpr → DM → TSR → CGR). CGR `items` resolve through
    /// `emit_representation_item_ref` (geometry / topology / placement
    /// caches all populated by earlier emit passes).
    pub(in crate::writer::buffer) fn emit_constructive_geometry_representations(
        &mut self,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::constructive_geometry_representation::ConstructiveGeometryRepresentationHandler;
        let reprs = self.model.shape_rep.representations.clone();
        for (id, repr) in reprs.iter_with_ids() {
            if let Representation::ConstructiveGeometry(cgr) = repr {
                let step_id = ConstructiveGeometryRepresentationHandler::write(self, cgr.clone())?;
                self.set_step_id(id, step_id);
            }
        }
        Ok(())
    }

    /// Emit the `Representation::ShapeRepresentationWithParameters` arena
    /// entries (phase srwp). Called after every other delayed-emit
    /// Representation pass so `representation_step_ids` keeps arena order
    /// (Mdgpr → DM → TSR → CGR → SRWP).
    pub(in crate::writer::buffer) fn emit_shape_representation_with_parameters(
        &mut self,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::srwp::ShapeRepresentationWithParametersHandler;
        let reprs = self.model.shape_rep.representations.clone();
        for (id, repr) in reprs.iter_with_ids() {
            if let Representation::ShapeRepresentationWithParameters(srwp) = repr {
                let step_id = ShapeRepresentationWithParametersHandler::write(self, srwp.clone())?;
                self.set_step_id(id, step_id);
            }
        }
        Ok(())
    }

    /// Emit the `RepresentationRelationship` arena (phase cgrr). Called
    /// from `emit_pools` after every delayed-emit Representation pass
    /// (Mdgpr / DM / TSR / CGR) so `rep_1` / `rep_2` resolve through a
    /// fully populated `representation_step_ids` cache.
    pub(in crate::writer::buffer) fn emit_representation_relationships(
        &mut self,
    ) -> Result<(), WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::cgr_relationship::ConstructiveGeometryRepresentationRelationshipHandler;
        use crate::ir::shape_rep::RepresentationRelationship;
        let rrs: Vec<_> = self
            .model
            .shape_rep
            .representation_relationships
            .iter()
            .cloned()
            .collect();
        for rr in rrs {
            match rr {
                RepresentationRelationship::Itself(data) => {
                    use crate::entities::shape_rep::representation_relationship::RepresentationRelationshipHandler;
                    let r1 = self.step_id(data.rep_1);
                    let r2 = self.step_id(data.rep_2);
                    if r1 == 0 || r2 == 0 {
                        continue;
                    }
                    RepresentationRelationshipHandler::write(self, data)?;
                }
                RepresentationRelationship::ConstructiveGeometryRepresentationRelationship(
                    cgrr,
                ) => {
                    ConstructiveGeometryRepresentationRelationshipHandler::write(self, cgrr)?;
                }
                RepresentationRelationship::MechanicalDesignAndDraughtingRelationship(mddr) => {
                    use crate::entities::shape_rep::mddr::MechanicalDesignAndDraughtingRelationshipHandler;
                    let r1 = self.step_id(mddr.rep_1);
                    let r2 = self.step_id(mddr.rep_2);
                    if r1 == 0 || r2 == 0 {
                        continue;
                    }
                    MechanicalDesignAndDraughtingRelationshipHandler::write(self, mddr)?;
                }
                RepresentationRelationship::ShapeRepresentationRelationship(srr) => {
                    let r1 = self.step_id(srr.rep_1);
                    let r2 = self.step_id(srr.rep_2);
                    if r1 == 0 || r2 == 0 {
                        continue;
                    }
                    let _ = self.emit_simple_srr(r1, r2);
                }
                RepresentationRelationship::RepresentationRelationshipWithTransformation(_) => {
                    // Assembly placement complexes are emitted inline by
                    // `emit_instance_bundle` (interleaved with their NAUO / CDSR /
                    // IDT). Skip here so they are not double-emitted.
                }
            }
        }
        Ok(())
    }

    pub(in crate::writer::buffer) fn emit_representations_pre_pass(
        &mut self,
    ) -> Result<(), WriteError> {
        self.assert_phase(
            super::EmitPhase::Representations,
            "emit_representations_pre_pass (indexes geometry/topology/representation_item step ids)",
        );
        let reprs = self.model.shape_rep.representations.clone();
        for (id, repr) in reprs.iter_with_ids() {
            // Mdgpr (visualization pass) and DraughtingModel (post-callout
            // pass) are emitted later — their `items` refs depend on
            // step-id caches that this pre-pass cannot populate.
            if matches!(
                repr,
                Representation::Mdgpr(_)
                    | Representation::DraughtingModel(_)
                    | Representation::TessellatedShapeRepresentation(_)
                    | Representation::ConstructiveGeometry(_)
                    | Representation::ShapeRepresentationWithParameters(_)
            ) {
                continue;
            }
            // An assembly ABSR whose items include a MAPPED_ITEM forward-
            // references entities emitted by `emit_mapped_items` (which runs
            // after this pre-pass). Reserve its step id here so backward-refs
            // (e.g. SDR.used_representation) resolve, and emit the body later
            // in `emit_deferred_assembly_absr`.
            if let Representation::AdvancedBrep(r) = repr {
                if r.items
                    .iter()
                    .any(|it| matches!(it, RepresentationItemRef::MappedItem(_)))
                {
                    let reserved = self.fresh();
                    self.set_step_id(id, reserved);
                    self.deferred_assembly_absr_ids.push((id, reserved));
                    continue;
                }
            }
            let step_id = self.emit_representation(repr)?;
            self.set_step_id(id, step_id);
        }
        Ok(())
    }

    /// Build the `[name, items, context]` attrs of an
    /// `ADVANCED_BREP_SHAPE_REPRESENTATION` from its arena items, preserving
    /// source order. Shared by the pre-pass (solid ABSR) and the deferred pass
    /// (assembly ABSR whose MAPPED_ITEM items emit later).
    fn advanced_brep_attrs(
        &mut self,
        r: &crate::ir::shape_rep::AdvancedBrepRepr,
    ) -> Result<Vec<Attribute>, WriteError> {
        let mut items = Vec::with_capacity(r.items.len());
        for it in &r.items {
            items.push(Attribute::EntityRef(
                self.emit_representation_item_ref(*it)?,
            ));
        }
        Ok(vec![
            Attribute::String(r.name.clone()),
            Attribute::List(items),
            self.repr_context_attr(r.context),
        ])
    }

    /// Emit the bodies of assembly ABSRs whose step ids were reserved in
    /// `emit_representations_pre_pass`. Runs after `emit_mapped_items` so the
    /// MAPPED_ITEM items resolve to non-zero step ids.
    pub(in crate::writer::buffer) fn emit_deferred_assembly_absr(
        &mut self,
    ) -> Result<(), WriteError> {
        let deferred = std::mem::take(&mut self.deferred_assembly_absr_ids);
        let reprs = self.model.shape_rep.representations.clone();
        for (rid, reserved) in deferred {
            if let Representation::AdvancedBrep(r) = &reprs[rid] {
                let attrs = self.advanced_brep_attrs(r)?;
                self.push_simple_with_id(reserved, "ADVANCED_BREP_SHAPE_REPRESENTATION", attrs);
            }
        }
        Ok(())
    }

    /// Emit one geometry representation purely from its arena data — the
    /// `name`, `context`, coordinate frame and geometry are taken from the
    /// `Representation` variant, with no `Product` dependency. Sub-entities
    /// (solids, SBSM, geometric curve sets) are cache-hit or freshly
    /// emitted as needed.
    #[allow(clippy::too_many_lines)]
    fn emit_representation(&mut self, repr: &Representation) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::geometry::geometric_curve_set::{
            CurveSetWriteInput, GeometricCurveSetHandler,
        };
        use crate::entities::geometry::geometric_set::GeometricSetHandler;
        use crate::entities::geometry::shell_based_surface_model::ShellBasedSurfaceModelHandler;

        match repr {
            Representation::AdvancedBrep(r) => {
                let attrs = self.advanced_brep_attrs(r)?;
                Ok(self.push_simple("ADVANCED_BREP_SHAPE_REPRESENTATION", attrs))
            }
            Representation::ManifoldSurface(r) => {
                let mut items = Vec::with_capacity(2);
                if let Some(frame) = r.ref_frame {
                    items.push(Attribute::EntityRef(self.emit_axis2_placement_3d(frame)?));
                }
                // Route SBSM emit through the unified GRI arena cache so
                // an SBSM also referenced from a STYLED_ITEM doesn't
                // emit twice. `sbsm_ids` is populated when the source
                // file came through the reader; kernel-built IR can
                // leave it empty, in which case we fall back to inline
                // emit from the flattened `shells`.
                if r.sbsm_ids.is_empty() {
                    let sbsm = ShellBasedSurfaceModelHandler::write(self, r.shells.clone())?;
                    items.push(Attribute::EntityRef(sbsm));
                } else {
                    for sbsm_id in &r.sbsm_ids {
                        let step_id = self.emit_representation_item_ref(
                            crate::ir::representation_item::RepresentationItemRef::GeometricRepresentationItem(
                                *sbsm_id,
                            ),
                        )?;
                        items.push(Attribute::EntityRef(step_id));
                    }
                }
                Ok(self.push_simple(
                    "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
                    vec![
                        Attribute::String(r.name.clone()),
                        Attribute::List(items),
                        self.repr_context_attr(r.context),
                    ],
                ))
            }
            Representation::Plain(r) => {
                let mut items = Vec::with_capacity(1);
                if let Some(frame) = r.frame {
                    items.push(Attribute::EntityRef(self.emit_axis2_placement_3d(frame)?));
                }
                Ok(self.push_simple(
                    "SHAPE_REPRESENTATION",
                    vec![
                        Attribute::String(r.name.clone()),
                        Attribute::List(items),
                        self.repr_context_attr(r.context),
                    ],
                ))
            }
            Representation::Wireframe(r) => {
                let mut items = Vec::with_capacity(2);
                if let Some(frame) = r.ref_frame {
                    items.push(Attribute::EntityRef(self.emit_axis2_placement_3d(frame)?));
                }
                // Route GCS/GS emits through the unified GRI arena cache
                // so a curve set also referenced from a STYLED_ITEM
                // doesn't emit twice. `gcs_ids` is populated when the
                // source file came through the reader; kernel-built IR
                // can leave it empty, in which case we fall back to a
                // single inline emit from the flattened content.
                if r.gcs_ids.is_empty() {
                    let set_input = CurveSetWriteInput {
                        curves: r.content.curves.clone(),
                        points: r.content.points.clone(),
                    };
                    let set_ref = if r.content.points.is_empty() {
                        GeometricCurveSetHandler::write(self, set_input)?
                    } else {
                        GeometricSetHandler::write(self, set_input)?
                    };
                    items.push(Attribute::EntityRef(set_ref));
                } else {
                    for gcs_id in &r.gcs_ids {
                        let set_ref = self.emit_representation_item_ref(
                            crate::ir::representation_item::RepresentationItemRef::GeometricRepresentationItem(
                                *gcs_id,
                            ),
                        )?;
                        items.push(Attribute::EntityRef(set_ref));
                    }
                }
                let name = match r.content.repr_kind {
                    WireframeReprKind::Surface => {
                        "GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION"
                    }
                    WireframeReprKind::Wireframe => {
                        "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION"
                    }
                };
                Ok(self.push_simple(
                    name,
                    vec![
                        Attribute::String(r.name.clone()),
                        Attribute::List(items),
                        self.repr_context_attr(r.context),
                    ],
                ))
            }
            Representation::Mdgpr(_) => {
                unreachable!("MDGPR is emitted by the visualization pass, not the pre-emit pass")
            }
            Representation::DraughtingModel(_) => {
                unreachable!(
                    "DRAUGHTING_MODEL is emitted by emit_draughting_models, not the pre-emit pass"
                )
            }
            Representation::TessellatedShapeRepresentation(_) => {
                unreachable!(
                    "TESSELLATED_SHAPE_REPRESENTATION is emitted by emit_tessellated_shape_representations, not the pre-emit pass"
                )
            }
            Representation::ConstructiveGeometry(_) => {
                unreachable!(
                    "CONSTRUCTIVE_GEOMETRY_REPRESENTATION is emitted by emit_constructive_geometry_representations, not the pre-emit pass"
                )
            }
            Representation::ShapeRepresentationWithParameters(_) => {
                unreachable!(
                    "SHAPE_REPRESENTATION_WITH_PARAMETERS is emitted by emit_shape_representation_with_parameters, not the pre-emit pass"
                )
            }
            Representation::ShapeDimensionRepresentation(r) => {
                use crate::entities::shape_rep::shape_dimension_representation::ShapeDimensionRepresentationHandler;
                ShapeDimensionRepresentationHandler::write(self, r.clone())
            }
        }
    }

    /// Resolve a representation's `Option<RepresentationContextRef>` to
    /// its `context_of_items` attribute. `Unitful` indexes the cached
    /// `REPRESENTATION_CONTEXT` step ids populated by the unit pass;
    /// `Unitless` indexes the GRC+PRC complex step ids populated by the
    /// `emit_unitless_contexts` pass; `None` emits `Unset`.
    pub(crate) fn repr_context_attr(
        &self,
        context: Option<crate::ir::shape_rep::RepresentationContextRef>,
    ) -> Attribute {
        use crate::ir::shape_rep::RepresentationContextRef as R;
        match context {
            Some(R::Unitful(id)) => Attribute::EntityRef(self.unit_context_ids[id.0 as usize]),
            Some(R::Unitless(id)) => Attribute::EntityRef(self.step_id(id)),
            None => Attribute::Unset,
        }
    }

    // ----------------------------------------------------------------
    // Context
    // ----------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    fn emit_application_context(&mut self, schema: &StepSchema) -> AssemblyContextIds {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::plm::application_context::ApplicationContextHandler;
        use crate::entities::plm::application_protocol_definition::{
            ApplicationProtocolDefinitionHandler, ApplicationProtocolDefinitionWriteInput,
        };
        use crate::ir::plm::ApplicationContext;
        use crate::ir::{ProductContext, ProductDefinitionContext};
        let has_ac = self
            .model
            .plm
            .as_ref()
            .is_some_and(|p| !p.application_contexts.is_empty());
        let has_pc_pdc = self.model.assembly.as_ref().is_some_and(|a| {
            !a.product_contexts.is_empty() || !a.product_definition_contexts.is_empty()
        });
        if has_ac || has_pc_pdc {
            // 1) Emit all IR AC entries, caching step ids by IR index.
            let plm = self.model.plm.clone().unwrap_or_default();
            self.ac_step_ids = Vec::with_capacity(plm.application_contexts.len());
            for ac in plm.application_contexts.iter() {
                let id = ApplicationContextHandler::write(self, ac.clone())
                    .expect("AC write only pushes one simple entity");
                self.ac_step_ids.push(id);
            }
            // If IR has no AC but does have PC/PDC, synthesise one AC for
            // PC.frame_of_reference to point at.
            if self.ac_step_ids.is_empty() {
                let (desc, status, name, year) = apd_info(schema);
                let id = ApplicationContextHandler::write(
                    self,
                    ApplicationContext {
                        application: desc.into(),
                    },
                )
                .expect("AC write only pushes one simple entity");
                self.ac_step_ids.push(id);
                let _ = ApplicationProtocolDefinitionHandler::write(
                    self,
                    ApplicationProtocolDefinitionWriteInput {
                        status: status.into(),
                        application_interpreted_model_schema_name: name.into(),
                        application_protocol_year: year,
                        application: id,
                    },
                )
                .expect("APD write only pushes one simple entity");
            }
            // 2) Emit all APD entries (refs AC via cache). APD is top-level
            // (no consumer), so its step id is not cached.
            for apd in plm.application_protocol_definitions.iter() {
                let ac_step = self.ac_step_ids[apd.application.0 as usize];
                let _ = ApplicationProtocolDefinitionHandler::write(
                    self,
                    ApplicationProtocolDefinitionWriteInput {
                        status: apd.status.clone(),
                        application_interpreted_model_schema_name: apd
                            .application_interpreted_model_schema_name
                            .clone(),
                        application_protocol_year: apd.application_protocol_year,
                        application: ac_step,
                    },
                )
                .expect("APD write only pushes one simple entity");
            }
            // 3) Emit all PC entries (refs AC via cache).
            let assembly = self.model.assembly.clone().unwrap_or_default();
            self.pc_step_ids = Vec::with_capacity(assembly.product_contexts.len());
            for pc in assembly.product_contexts.iter() {
                // ref resolved here (orchestrator owns the ac cache); lift +
                // generated serialize handle the L1 shape / emit.
                let d = pc.data();
                let ac_step = self.ac_step_ids[d.frame_of_reference.0 as usize];
                let id = match pc {
                    ProductContext::Itself(_) => {
                        crate::early::serialize::serialize_product_context(
                            self,
                            &crate::early::lift::lift_product_context(d, ac_step),
                        )
                    }
                    ProductContext::Mechanical(_) => {
                        crate::early::serialize::serialize_mechanical_context(
                            self,
                            &crate::early::lift::lift_mechanical_context(d, ac_step),
                        )
                    }
                };
                self.pc_step_ids.push(id);
            }
            // 4) Emit all PDC entries (refs AC via cache).
            self.pdc_step_ids = Vec::with_capacity(assembly.product_definition_contexts.len());
            for pdc in assembly.product_definition_contexts.iter() {
                let d = pdc.data();
                let ac_step = self.ac_step_ids[d.frame_of_reference.0 as usize];
                let id = match pdc {
                    ProductDefinitionContext::Itself(_) => {
                        crate::early::serialize::serialize_product_definition_context(
                            self,
                            &crate::early::lift::lift_product_definition_context(d, ac_step),
                        )
                    }
                    ProductDefinitionContext::Design(_) => {
                        crate::early::serialize::serialize_design_context(
                            self,
                            &crate::early::lift::lift_design_context(d, ac_step),
                        )
                    }
                };
                self.pdc_step_ids.push(id);
            }
            // Fallback PC/PDC for products without explicit context.
            let product_ctx = if let Some(&id) = self.pc_step_ids.first() {
                id
            } else {
                self.push_simple(
                    "PRODUCT_CONTEXT",
                    vec![
                        Attribute::String(String::new()),
                        Attribute::EntityRef(self.ac_step_ids[0]),
                        Attribute::String("mechanical".into()),
                    ],
                )
            };
            let pdef_ctx = if let Some(&id) = self.pdc_step_ids.first() {
                id
            } else {
                self.push_simple(
                    "PRODUCT_DEFINITION_CONTEXT",
                    vec![
                        Attribute::String("part definition".into()),
                        Attribute::EntityRef(self.ac_step_ids[0]),
                        Attribute::String("design".into()),
                    ],
                )
            };
            return AssemblyContextIds {
                product_ctx,
                pdef_ctx,
            };
        }
        // Fallback: synthesise from schema class (no IR context entries).
        let (desc, status, name, year) = apd_info(schema);
        let app_ctx = ApplicationContextHandler::write(
            self,
            ApplicationContext {
                application: desc.into(),
            },
        )
        .expect("AC write only pushes one simple entity");
        let _ = ApplicationProtocolDefinitionHandler::write(
            self,
            ApplicationProtocolDefinitionWriteInput {
                status: status.into(),
                application_interpreted_model_schema_name: name.into(),
                application_protocol_year: year,
                application: app_ctx,
            },
        )
        .expect("APD write only pushes one simple entity");
        let product_ctx = self.push_simple(
            "PRODUCT_CONTEXT",
            vec![
                Attribute::String(String::new()),
                Attribute::EntityRef(app_ctx),
                Attribute::String("mechanical".into()),
            ],
        );
        let pdef_ctx = self.push_simple(
            "PRODUCT_DEFINITION_CONTEXT",
            vec![
                Attribute::String("part definition".into()),
                Attribute::EntityRef(app_ctx),
                Attribute::String("design".into()),
            ],
        );
        AssemblyContextIds {
            product_ctx,
            pdef_ctx,
        }
    }

    // ----------------------------------------------------------------
    // Per-product emitters
    // ----------------------------------------------------------------

    pub(crate) fn emit_product(&mut self, product: &Product, ctx: &AssemblyContextIds) -> u64 {
        use crate::entities::SimpleEntityHandler;
        crate::entities::assembly_product::product::ProductHandler::write(
            self,
            (product.clone(), *ctx),
        )
        .expect("PRODUCT write only pushes one simple entity")
    }

    /// Emit this product's `PRODUCT_DEFINITION_FORMATION`. When the product
    /// links an arena formation (reader path), emit it faithfully — preserving
    /// the version `id` / `description` / `make_or_buy` — and cache its step id
    /// in `product_definition_formation_step_ids`. Otherwise (kernel-built IR,
    /// empty arena) synthesise a bare formation, picking the subtype by the
    /// `formation_with_source` mirror flag, exactly as before.
    pub(crate) fn emit_formation(&mut self, prod_entity: u64, product: &Product) -> u64 {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::assembly_product::product_definition_formation::{
            ProductDefinitionFormationHandler, ProductDefinitionFormationWriteInput,
        };
        use crate::entities::assembly_product::product_definition_formation_with_source::{
            ProductDefinitionFormationWithSourceHandler,
            ProductDefinitionFormationWithSourceWriteInput,
        };
        use crate::ir::assembly::ProductDefinitionFormation;

        if let Some(fid) = product.formation {
            let formation = self
                .model
                .assembly
                .as_ref()
                .expect("product.formation set implies an assembly")
                .product_definition_formations[fid]
                .clone();
            let step = match formation {
                ProductDefinitionFormation::Itself(d) => ProductDefinitionFormationHandler::write(
                    self,
                    ProductDefinitionFormationWriteInput {
                        id: d.id,
                        description: d.description,
                        prod_entity,
                    },
                ),
                ProductDefinitionFormation::WithSpecifiedSource(s) => {
                    ProductDefinitionFormationWithSourceHandler::write(
                        self,
                        ProductDefinitionFormationWithSourceWriteInput {
                            id: s.inherited.id,
                            description: s.inherited.description,
                            prod_entity,
                            make_or_buy: s.make_or_buy,
                        },
                    )
                }
            }
            .expect("formation write only pushes one simple entity");
            self.set_step_id(fid, step);
            return step;
        }

        // Synthesis fallback — kernel-built IR with no arena formation.
        if product.formation_with_source {
            ProductDefinitionFormationWithSourceHandler::write(
                self,
                ProductDefinitionFormationWithSourceWriteInput {
                    id: String::new(),
                    description: String::new(),
                    prod_entity,
                    make_or_buy: "NOT_KNOWN".into(),
                },
            )
            .expect("PDF_WITH_SOURCE write only pushes one simple entity")
        } else {
            ProductDefinitionFormationHandler::write(
                self,
                ProductDefinitionFormationWriteInput {
                    id: String::new(),
                    description: String::new(),
                    prod_entity,
                },
            )
            .expect("PDF write only pushes one simple entity")
        }
    }

    /// Emit the `product_categories` arena (phase pc-unify-a). Iterates
    /// arena order, emitting PC `Itself` and PRPC variants in turn.
    /// Slot id of every emit is cached in `product_category_step_ids`
    /// for the PCR emitter to consume.
    // `idx` is an arena index, which fits u32 by the arena's own overflow
    // guard — the `idx as u32` id reconstruction is safe.
    #[allow(clippy::cast_possible_truncation)]
    pub(in crate::writer::buffer) fn emit_product_categories_arena(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::assembly_product::product_category::{
            ProductCategoryHandler, ProductCategoryWriteInput,
        };
        use crate::entities::assembly_product::product_related_product_category::{
            ProductRelatedProductCategoryHandler, ProductRelatedProductCategoryWriteInput,
        };
        use crate::ir::assembly::ProductCategory;

        let pool = self.model.assembly.as_ref();
        let Some(pool) = pool else { return };
        let pcs: Vec<ProductCategory> = pool.product_categories.iter().cloned().collect();
        for (idx, pc) in pcs.iter().enumerate() {
            let step = match pc {
                ProductCategory::Itself(data) => ProductCategoryHandler::write(
                    self,
                    ProductCategoryWriteInput {
                        name: data.name.clone(),
                        description: data.description.clone(),
                    },
                )
                .expect("PC write only pushes one simple entity"),
                ProductCategory::ProductRelatedProductCategory(data) => {
                    let product_refs: Vec<u64> = data
                        .products
                        .iter()
                        .filter_map(|pid| self.product_step_ids.get(pid).copied())
                        .collect();
                    if product_refs.is_empty() {
                        continue;
                    }
                    ProductRelatedProductCategoryHandler::write(
                        self,
                        ProductRelatedProductCategoryWriteInput {
                            kind: data.name.clone(),
                            kind_description: data.description.clone(),
                            product_refs,
                        },
                    )
                    .expect("PRPC write only pushes one simple entity")
                }
            };
            self.set_step_id(ProductCategoryId(idx as u32), step);
        }
    }

    /// Emit the `product_category_relationships` arena (phase pc-unify-a).
    /// Runs after `emit_product_categories_arena` so the PC + PRPC step
    /// ids are cached.
    pub(in crate::writer::buffer) fn emit_product_category_relationships_arena(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::assembly_product::product_category_relationship::{
            ProductCategoryRelationshipHandler, ProductCategoryRelationshipWriteInput,
        };
        let pool = self.model.assembly.as_ref();
        let Some(pool) = pool else { return };
        let pcrs: Vec<_> = pool
            .product_category_relationships
            .iter()
            .cloned()
            .collect();
        for pcr in pcrs {
            let pc = self.step_id(pcr.category);
            let prpc = self.step_id(pcr.sub_category);
            if pc == 0 || prpc == 0 {
                continue;
            }
            let _ = ProductCategoryRelationshipHandler::write(
                self,
                ProductCategoryRelationshipWriteInput {
                    pc_ref: pc,
                    prpc_ref: prpc,
                },
            );
        }
    }

    /// Emit the product definition for one product. With `doc_refs` empty this
    /// is a plain `PRODUCT_DEFINITION`; otherwise the source used the
    /// `_WITH_ASSOCIATED_DOCUMENTS` subtype and we re-emit it with the resolved
    /// `DOCUMENT` step ids in `documentation_ids`.
    pub(crate) fn emit_pdef(
        &mut self,
        id: &str,
        description: &str,
        formation: u64,
        pdef_ctx: u64,
        doc_refs: &[u64],
    ) -> u64 {
        use crate::entities::SimpleEntityHandler;
        if doc_refs.is_empty() {
            use crate::entities::assembly_product::product_definition::{
                ProductDefinitionHandler, ProductDefinitionWriteInput,
            };
            ProductDefinitionHandler::write(
                self,
                ProductDefinitionWriteInput {
                    id: id.to_owned(),
                    description: description.to_owned(),
                    formation,
                    pdef_ctx,
                },
            )
            .expect("PRODUCT_DEFINITION write only pushes one simple entity")
        } else {
            use crate::entities::assembly_product::product_definition_with_associated_documents::{
                ProductDefinitionWithAssociatedDocumentsHandler,
                ProductDefinitionWithAssociatedDocumentsWriteInput,
            };
            ProductDefinitionWithAssociatedDocumentsHandler::write(
                self,
                ProductDefinitionWithAssociatedDocumentsWriteInput {
                    id: id.to_owned(),
                    description: description.to_owned(),
                    formation,
                    pdef_ctx,
                    documentation: doc_refs.to_vec(),
                },
            )
            .expect("PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS write pushes one simple entity")
        }
    }

    pub(crate) fn emit_mssr(
        &mut self,
        product: &Product,
        shells: &[crate::ir::ShellId],
        unit_ctx: u64,
    ) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::manifold_surface_shape_representation::{
            ManifoldSurfaceShapeRepresentationHandler, ManifoldSurfaceShapeRepresentationWriteInput,
        };
        ManifoldSurfaceShapeRepresentationHandler::write(
            self,
            ManifoldSurfaceShapeRepresentationWriteInput {
                product: product.clone(),
                shells: shells.to_vec(),
                unit_ctx,
            },
        )
    }

    /// Emit a wireframe `SHAPE_REPRESENTATION` (`GBWSR` / `GBSSR`) plus its
    /// `GEOMETRIC_(CURVE_)SET` of items. Picks the `..._SURFACE_...` cousin
    /// when `repr_kind == Surface` (CATIA convention) and uses
    /// `GEOMETRIC_SET` when loose points coexist with curves; otherwise
    /// emits the more common `..._WIREFRAME_...` + `GEOMETRIC_CURVE_SET`.
    pub(crate) fn emit_wireframe_representation(
        &mut self,
        product: &Product,
        wf: &WireframeContent,
        unit_ctx: u64,
    ) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::gbssr::GbssrHandler;
        use crate::entities::shape_rep::gbwsr::{GbwsrHandler, WireframeRepresentationWriteInput};

        let input = WireframeRepresentationWriteInput {
            product: product.clone(),
            wireframe: wf.clone(),
            unit_ctx,
        };
        match wf.repr_kind {
            WireframeReprKind::Surface => GbssrHandler::write(self, input),
            WireframeReprKind::Wireframe => GbwsrHandler::write(self, input),
        }
    }

    pub(crate) fn emit_absr(
        &mut self,
        product: &Product,
        solid_refs: Vec<u64>,
        unit_ctx: u64,
    ) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::advanced_brep_shape_representation::{
            AdvancedBrepShapeRepresentationHandler, AdvancedBrepShapeRepresentationWriteInput,
        };
        AdvancedBrepShapeRepresentationHandler::write(
            self,
            AdvancedBrepShapeRepresentationWriteInput {
                product: product.clone(),
                solid_refs,
                unit_ctx,
            },
        )
    }

    /// Emit a `SHAPE_REPRESENTATION` for a Group product — no geometry,
    /// just the product's coordinate reference frame so the entity is
    /// well-formed.
    pub(crate) fn emit_group_shape_representation(
        &mut self,
        product: &Product,
        unit_ctx: u64,
    ) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::shape_representation::{
            ShapeRepresentationHandler, ShapeRepresentationWriteInput,
        };
        ShapeRepresentationHandler::write(
            self,
            ShapeRepresentationWriteInput {
                product: product.clone(),
                unit_ctx,
            },
        )
    }

    pub(crate) fn emit_sdr(&mut self, pdef_shape: u64, sr: u64) -> u64 {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::property::shape_definition_representation::{
            ShapeDefinitionRepresentationHandler, ShapeDefinitionRepresentationWriteInput,
        };
        ShapeDefinitionRepresentationHandler::write(
            self,
            ShapeDefinitionRepresentationWriteInput {
                pdef_shape,
                shape_rep: sr,
            },
        )
        .expect("SDR write only pushes one simple entity")
    }

    /// Wrap a geometry-carrying SR (ABSR / MSSR) in the Fusion 360 / CATIA
    /// indirect form when the source file used it: emit a plain
    /// `SHAPE_REPRESENTATION` holding only `outer_sr_frame`, link it to the
    /// geometry SR via `SHAPE_REPRESENTATION_RELATIONSHIP`, and return the
    /// plain SR id so the SDR points there. With `outer_sr_frame == None`
    /// (the default) the geometry SR is returned unchanged.
    fn wrap_indirect_sr_if_set(
        &mut self,
        product: &Product,
        geo_sr: u64,
        unit_ctx: u64,
    ) -> Result<u64, WriteError> {
        let Some(outer_frame) = product.outer_sr_frame else {
            return Ok(geo_sr);
        };
        let outer_axis = self.emit_axis2_placement_3d(outer_frame)?;
        let plain_sr = self.push_simple(
            "SHAPE_REPRESENTATION",
            vec![
                Attribute::String(String::new()),
                Attribute::List(vec![Attribute::EntityRef(outer_axis)]),
                Attribute::EntityRef(unit_ctx),
            ],
        );
        let _ = self.emit_simple_srr(plain_sr, geo_sr);
        Ok(plain_sr)
    }

    pub(crate) fn emit_simple_srr(&mut self, rep_1: u64, rep_2: u64) -> u64 {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::shape_rep::shape_representation_relationship::{
            ShapeRepresentationRelationshipHandler, ShapeRepresentationRelationshipWriteInput,
        };
        ShapeRepresentationRelationshipHandler::write(
            self,
            ShapeRepresentationRelationshipWriteInput { rep_1, rep_2 },
        )
        .expect("SRR write only pushes one simple entity")
    }

    // ----------------------------------------------------------------
    // Per-instance emitters
    // ----------------------------------------------------------------

    // `slot` is an arena index, which fits u32 by the arena's own overflow
    // guard — the `slot as u32` id reconstruction is safe.
    #[allow(clippy::cast_possible_truncation)]
    fn emit_instance_bundle(
        &mut self,
        inst: &Instance,
        parent_pdef: u64,
        parent_sr: u64,
        product_def: &HashMap<ProductId, u64>,
        product_sr: &HashMap<ProductId, u64>,
    ) -> Result<(), WriteError> {
        let child_pdef = product_def[&inst.child];
        let child_sr = product_sr[&inst.child];
        let idt = self.emit_item_defined_transformation(inst.transform)?;
        let nauo = self.emit_nauo(inst, parent_pdef, child_pdef);
        // Reader-built IR materialises the NAUO-owned PDS in the
        // `property_definitions` arena. Emit its body here (it needs the NAUO
        // step id), preserving the source name/description, and fill the arena
        // slot so the dependent centroid PD resolves it. Kernel-built IR (no
        // arena slot) synthesises the PDS as before.
        let arena_pds = inst
            .acu
            .and_then(|a| self.nauo_pds_arena_slot.get(&a).cloned());
        let nauo_pds = match arena_pds {
            Some((slot, name, description)) => {
                let step = self.push_simple(
                    "PRODUCT_DEFINITION_SHAPE",
                    vec![
                        Attribute::String(name),
                        Attribute::String(description),
                        Attribute::EntityRef(nauo),
                    ],
                );
                self.set_step_id(PropertyDefinitionId(slot as u32), step);
                step
            }
            None => self.emit_nauo_owned_pds(nauo),
        };
        // Extra placement SDRs: some exporters link the NAUO-owned PDS to one or
        // more standalone placement SHAPE_REPRESENTATIONs (besides the CDSR).
        // Re-emit one SDR per entry, all sharing the same `nauo_pds` the CDSR
        // uses (so every SDR references the one placement PDS, as in the source).
        for &sr_id in &inst.placement_representation {
            let sr_step = self.step_id(sr_id);
            if sr_step != 0 {
                self.emit_sdr(nauo_pds, sr_step);
            }
        }
        // Reader-built IR materialises the placement as a
        // `RepresentationRelationshipWithTransformation` arena entry; emit the
        // complex from its faithful `rep_1`/`rep_2`/`name`/`description` and
        // record the step id so `style_context` can reference it. Kernel-built
        // IR (`transform_rr == None`) synthesises the complex from `parent_sr` /
        // `child_sr` as before.
        let rrwt = match inst.transform_rr {
            Some(rrid) => {
                use crate::ir::shape_rep::RepresentationRelationship;
                let (name, description, p_step, c_step) = match &self
                    .model
                    .shape_rep
                    .representation_relationships[rrid]
                {
                    RepresentationRelationship::RepresentationRelationshipWithTransformation(d) => {
                        let p = match self.step_id(d.rep_1) {
                            0 => parent_sr,
                            s => s,
                        };
                        let c = match self.step_id(d.rep_2) {
                            0 => child_sr,
                            s => s,
                        };
                        (d.name.clone(), d.description.clone(), p, c)
                    }
                    _ => (String::new(), String::new(), parent_sr, child_sr),
                };
                let rrwt = self.emit_rrwt_complex(&name, &description, p_step, c_step, idt);
                self.set_step_id(rrid, rrwt);
                rrwt
            }
            None => self.emit_rrwt_complex("", "", parent_sr, child_sr, idt),
        };
        let _cdsr = {
            use crate::entities::SimpleEntityHandler;
            use crate::entities::assembly_product::context_dependent_shape_representation::{
                ContextDependentShapeRepresentationHandler,
                ContextDependentShapeRepresentationWriteInput,
            };
            ContextDependentShapeRepresentationHandler::write(
                self,
                ContextDependentShapeRepresentationWriteInput { rrwt, nauo_pds },
            )?
        };
        Ok(())
    }

    pub(crate) fn emit_item_defined_transformation(
        &mut self,
        transform: Transform3d,
    ) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        crate::entities::shape_rep::item_defined_transformation::ItemDefinedTransformationHandler::write(
            self,
            transform,
        )
    }

    /// Emit the instance's `NEXT_ASSEMBLY_USAGE_OCCURRENCE`. The sole NAUO emit
    /// point (called once per bundle), so the NAUO appears exactly when its
    /// placement bundle does — no orphan / double emit. Reader-built instances
    /// (`acu == Some`) emit from the canonical `assembly_component_usage` arena
    /// entry (real id / name / description / reference_designator); kernel-built
    /// instances (`acu == None`) synthesise from the `Instance` fields as before
    /// (no description / reference_designator to preserve → empty / unset).
    pub(crate) fn emit_nauo(&mut self, inst: &Instance, parent_pdef: u64, child_pdef: u64) -> u64 {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::assembly_product::next_assembly_usage_occurrence::{
            NextAssemblyUsageOccurrenceHandler, NextAssemblyUsageOccurrenceWriteInput,
        };
        let input = if let Some(acuid) = inst.acu {
            let acu = &self
                .model
                .assembly
                .as_ref()
                .expect("assembly present when Instance.acu is set")
                .assembly_component_usages[acuid];
            NextAssemblyUsageOccurrenceWriteInput {
                id: acu.id.clone(),
                name: acu.name.clone(),
                description: acu.description.clone(),
                reference_designator: acu.reference_designator.clone(),
                relating: parent_pdef,
                related: child_pdef,
            }
        } else {
            NextAssemblyUsageOccurrenceWriteInput {
                id: inst.occurrence_id.clone(),
                name: inst.occurrence_name.clone(),
                description: String::new(),
                reference_designator: None,
                relating: parent_pdef,
                related: child_pdef,
            }
        };
        NextAssemblyUsageOccurrenceHandler::write(self, input)
            .expect("NAUO write only pushes one simple entity")
    }

    /// NAUO-owned `PRODUCT_DEFINITION_SHAPE` — its `definition` points at the
    /// NAUO rather than a PRODUCT_DEFINITION. Uses the OCCT convention of
    /// `'Placement'` / `'Placement of an item'` for name/description so diffs
    /// against FreeCAD fixtures stay small.
    fn emit_nauo_owned_pds(&mut self, nauo: u64) -> u64 {
        self.push_simple(
            "PRODUCT_DEFINITION_SHAPE",
            vec![
                Attribute::String("Placement".into()),
                Attribute::String("Placement of an item".into()),
                Attribute::EntityRef(nauo),
            ],
        )
    }

    fn emit_rrwt_complex(
        &mut self,
        name: &str,
        description: &str,
        parent_sr: u64,
        child_sr: u64,
        idt: u64,
    ) -> u64 {
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex {
                parts: vec![
                    (
                        "REPRESENTATION_RELATIONSHIP".into(),
                        vec![
                            Attribute::String(name.to_owned()),
                            Attribute::String(description.to_owned()),
                            Attribute::EntityRef(parent_sr),
                            Attribute::EntityRef(child_sr),
                        ],
                    ),
                    (
                        "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION".into(),
                        vec![Attribute::EntityRef(idt)],
                    ),
                    ("SHAPE_REPRESENTATION_RELATIONSHIP".into(), vec![]),
                ],
            },
        });
        n
    }

    // ----------------------------------------------------------------
    // Low-level
    // ----------------------------------------------------------------

    pub(crate) fn push_simple(&mut self, name: &str, attrs: Vec<Attribute>) -> u64 {
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: name.into(),
                attrs,
            },
        });
        n
    }

    /// Push a complex (multi-part) entity: `parts` is the ordered
    /// `(part_name, attrs)` list rendered as `( PART1(..) PART2(..) … )`.
    pub(crate) fn push_complex(&mut self, parts: Vec<(String, Vec<Attribute>)>) -> u64 {
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex { parts },
        });
        n
    }

    /// Push a simple entity under a previously-reserved id (from `fresh()`),
    /// rather than minting a new one. Lets a body emit after the referrers
    /// that already used its reserved id (STEP forward references).
    pub(crate) fn push_simple_with_id(&mut self, id: u64, name: &str, attrs: Vec<Attribute>) {
        self.entities.push(WriterEntity {
            id,
            body: WriterBody::Simple {
                name: name.into(),
                attrs,
            },
        });
    }

    /// Push a complex (multi-part) entity under a previously-reserved id — the
    /// complex analogue of [`Self::push_simple_with_id`], for reserve-then-fill
    /// emit paths (e.g. the units pool's pre-reserved `NamedUnit` ids).
    pub(crate) fn push_complex_with_id(&mut self, id: u64, parts: Vec<(String, Vec<Attribute>)>) {
        self.entities.push(WriterEntity {
            id,
            body: WriterBody::Complex { parts },
        });
    }
}

/// Per-AP `APPLICATION_CONTEXT` description and
/// `APPLICATION_PROTOCOL_DEFINITION` attributes:
/// `(description, status, schema_name, year)`.
///
/// `Ap242Is` falls back to AP214 IS because no fixture guides it yet.
/// `Unknown` also falls back here: step-io cannot fabricate
/// application_context metadata (description, year, schema name) for
/// schemas it doesn't recognise. The FILE_SCHEMA header still carries
/// the original schema string verbatim (see `writer::header`), so a
/// round-tripped Unknown file has an honest FILE_SCHEMA but
/// AP214-flavoured APD — a best-effort partial round-trip.
fn apd_info(schema: &StepSchema) -> (&'static str, &'static str, &'static str, i64) {
    match schema.class() {
        Some(SchemaClass::Ap203) => (
            "configuration controlled 3D designs of mechanical parts and assemblies",
            "international standard",
            "config_control_design",
            1994,
        ),
        Some(SchemaClass::Ap214Cd) => (
            "core data for automotive mechanical design processes",
            "committee draft",
            "automotive_design",
            1997,
        ),
        Some(SchemaClass::Ap214Dis) => (
            "core data for automotive mechanical design processes",
            "draft international standard",
            "automotive_design",
            1998,
        ),
        Some(SchemaClass::Ap242Dis) => (
            "Managed model based 3d engineering",
            "international standard",
            "ap242_managed_model_based_3d_engineering",
            2013,
        ),
        // AP214 IS, AP242 IS placeholder, and Unknown (no class) all use
        // the AP214 IS fallback — see the comment above.
        None | Some(SchemaClass::Ap214Is | SchemaClass::Ap242Is) => (
            "core data for automotive mechanical design processes",
            "international standard",
            "automotive_design",
            2000,
        ),
    }
}
