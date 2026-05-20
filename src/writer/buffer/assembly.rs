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
use crate::ir::arena::Arena;
use crate::ir::shape_rep::Representation;
use crate::ir::{
    Instance, Product, ProductContent, ProductId, Transform3d, UnitContextId, WireframeContent,
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
        let schema = self.model.schema.clone();
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
                    let quantity_step = self.mwu_step_ids[mfu.quantity.0 as usize];
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
                self.emit_product_category_chain(product, prod_entity);
                self.product_def_ids.insert(pid, prod_entity);
                continue;
            }
            let formation = self.emit_formation(prod_entity, product);
            let pdef = self.emit_pdef(formation, per_product_ctx.pdef_ctx);
            let pdef_shape = {
                use crate::entities::SimpleEntityHandler;
                use crate::entities::assembly_product::product_definition_shape::{
                    ProductDefinitionShapeHandler, ProductDefinitionShapeWriteInput,
                };
                ProductDefinitionShapeHandler::write(
                    self,
                    ProductDefinitionShapeWriteInput { pdef },
                )?
            };
            self.product_def_ids.insert(pid, pdef);
            self.product_def_shape_ids.insert(pid, pdef_shape);
            self.emit_product_category_chain(product, prod_entity);

            // Metadata-only products (no SDR in source IR) carry
            // `geometry_context: None`. Skip SR + SDR emission for
            // them so the output mirrors the source absence; otherwise
            // the writer would attach a default context that re-reads
            // as `Some(UnitContextId(0))` and breaks IR equality.
            let Some(unit_ctx) = self.resolve_product_ctx(product) else {
                continue;
            };
            let sr = if let Some(rep_id) = product.representation_id {
                // Reader-built IR: the representation was pre-emitted in
                // arena order by `emit_representations_pre_pass`. Reuse the
                // cached step id rather than re-emitting inline.
                let geo_sr = self.representation_step_ids[rep_id.0 as usize];
                match product.outer_representation_id {
                    // Fusion 360 / CATIA indirect form: the outer plain SR
                    // wrapper is a separate pre-emitted arena entry. Link it
                    // to the geometry SR via SHAPE_REPRESENTATION_RELATIONSHIP
                    // and point the SDR at the outer SR.
                    Some(outer_id) => {
                        let outer_sr = self.representation_step_ids[outer_id.0 as usize];
                        let _ = self.emit_simple_srr(outer_sr, geo_sr);
                        outer_sr
                    }
                    None => geo_sr,
                }
            } else {
                // Hand/kernel-built IR: the `representations` arena is
                // unpopulated, so emit the representation inline from
                // `ProductContent` as before.
                match &product.content {
                    ProductContent::Solid(solid) => {
                        let solid_refs: Vec<u64> = solid
                            .ids
                            .iter()
                            .map(|sid| self.emit_solid(*sid))
                            .collect::<Result<_, _>>()?;
                        let absr = self.emit_absr(product, solid_refs, unit_ctx)?;
                        self.wrap_indirect_sr_if_set(product, absr, unit_ctx)?
                    }
                    ProductContent::SurfaceBody(sbody) => {
                        let mssr = self.emit_mssr(product, &sbody.ids, unit_ctx)?;
                        self.wrap_indirect_sr_if_set(product, mssr, unit_ctx)?
                    }
                    ProductContent::Wireframe(wf) => {
                        let gb_sr = self.emit_wireframe_representation(product, wf, unit_ctx)?;
                        self.wrap_indirect_sr_if_set(product, gb_sr, unit_ctx)?
                    }
                    ProductContent::Group(_) => {
                        self.emit_group_shape_representation(product, unit_ctx)?
                    }
                }
            };
            product_sr.insert(pid, sr);
            self.emit_sdr(pdef_shape, sr);
        }

        // Emit per-instance bundles for every Group product.
        for (parent_pid, parent) in products.iter_with_ids() {
            let ProductContent::Group(group) = &parent.content else {
                continue;
            };
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
            for inst in &group.instances {
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
    /// `[geometry reprs (Pass 6), MDGPR (Pass 7)]`, so the geometry reprs
    /// form a contiguous prefix and the appended `MDGPR` slots stay aligned
    /// with their `RepresentationId`s.
    pub(in crate::writer::buffer) fn emit_representations_pre_pass(
        &mut self,
    ) -> Result<(), WriteError> {
        let reprs = self.model.representations.clone();
        for repr in reprs.iter() {
            if let Representation::Mdgpr(_) = repr {
                continue;
            }
            let step_id = self.emit_representation(repr)?;
            self.representation_step_ids.push(step_id);
        }
        Ok(())
    }

    /// Emit one geometry representation purely from its arena data — the
    /// `name`, `context`, coordinate frame and geometry are taken from the
    /// `Representation` variant, with no `Product` dependency. Sub-entities
    /// (solids, SBSM, geometric curve sets) are cache-hit or freshly
    /// emitted as needed.
    fn emit_representation(&mut self, repr: &Representation) -> Result<u64, WriteError> {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::geometry::geometric_curve_set::{
            CurveSetWriteInput, GeometricCurveSetHandler,
        };
        use crate::entities::geometry::geometric_set::GeometricSetHandler;
        use crate::entities::geometry::shell_based_surface_model::ShellBasedSurfaceModelHandler;

        match repr {
            Representation::AdvancedBrep(r) => {
                let mut items = Vec::with_capacity(1 + r.solids.len());
                if let Some(frame) = r.ref_frame {
                    items.push(Attribute::EntityRef(self.emit_axis2_placement_3d(frame)?));
                }
                for sid in &r.solids {
                    items.push(Attribute::EntityRef(self.emit_solid(*sid)?));
                }
                Ok(self.push_simple(
                    "ADVANCED_BREP_SHAPE_REPRESENTATION",
                    vec![
                        Attribute::String(r.name.clone()),
                        Attribute::List(items),
                        self.repr_context_attr(r.context),
                    ],
                ))
            }
            Representation::ManifoldSurface(r) => {
                let mut items = Vec::with_capacity(2);
                if let Some(frame) = r.ref_frame {
                    items.push(Attribute::EntityRef(self.emit_axis2_placement_3d(frame)?));
                }
                let sbsm = ShellBasedSurfaceModelHandler::write(self, r.shells.clone())?;
                items.push(Attribute::EntityRef(sbsm));
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
        }
    }

    /// Resolve a representation's `Option<UnitContextId>` to its
    /// `context_of_items` attribute. `Some` indexes the cached
    /// `REPRESENTATION_CONTEXT` step ids; `None` emits `Unset`.
    fn repr_context_attr(&self, context: Option<UnitContextId>) -> Attribute {
        match context {
            Some(id) => Attribute::EntityRef(self.unit_context_ids[id.0 as usize]),
            None => Attribute::Unset,
        }
    }

    // ----------------------------------------------------------------
    // Context
    // ----------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    fn emit_application_context(&mut self, schema: &StepSchema) -> AssemblyContextIds {
        use crate::ir::{ProductContextKind, ProductDefinitionContextKind};
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
                let id = self.push_simple(
                    "APPLICATION_CONTEXT",
                    vec![Attribute::String(ac.application.clone())],
                );
                self.ac_step_ids.push(id);
            }
            // If IR has no AC but does have PC/PDC, synthesise one AC for
            // PC.frame_of_reference to point at.
            if self.ac_step_ids.is_empty() {
                let (desc, status, name, year) = apd_info(schema);
                let id =
                    self.push_simple("APPLICATION_CONTEXT", vec![Attribute::String(desc.into())]);
                self.ac_step_ids.push(id);
                let _ = self.push_simple(
                    "APPLICATION_PROTOCOL_DEFINITION",
                    vec![
                        Attribute::String(status.into()),
                        Attribute::String(name.into()),
                        Attribute::Integer(year),
                        Attribute::EntityRef(id),
                    ],
                );
            }
            // 2) Emit all APD entries (refs AC via cache).
            self.apd_step_ids = Vec::with_capacity(plm.application_protocol_definitions.len());
            for apd in plm.application_protocol_definitions.iter() {
                let ac_step = self.ac_step_ids[apd.application.0 as usize];
                let id = self.push_simple(
                    "APPLICATION_PROTOCOL_DEFINITION",
                    vec![
                        Attribute::String(apd.status.clone()),
                        Attribute::String(apd.application_interpreted_model_schema_name.clone()),
                        Attribute::Integer(apd.application_protocol_year),
                        Attribute::EntityRef(ac_step),
                    ],
                );
                self.apd_step_ids.push(id);
            }
            // 3) Emit all PC entries (refs AC via cache).
            let assembly = self.model.assembly.clone().unwrap_or_default();
            self.pc_step_ids = Vec::with_capacity(assembly.product_contexts.len());
            for pc in assembly.product_contexts.iter() {
                let entity_name = match pc.kind {
                    ProductContextKind::Plain => "PRODUCT_CONTEXT",
                    ProductContextKind::Mechanical => "MECHANICAL_CONTEXT",
                };
                let ac_step = self.ac_step_ids[pc.frame_of_reference.0 as usize];
                let id = self.push_simple(
                    entity_name,
                    vec![
                        Attribute::String(pc.name.clone()),
                        Attribute::EntityRef(ac_step),
                        Attribute::String(pc.discipline_type.clone()),
                    ],
                );
                self.pc_step_ids.push(id);
            }
            // 4) Emit all PDC entries (refs AC via cache).
            self.pdc_step_ids = Vec::with_capacity(assembly.product_definition_contexts.len());
            for pdc in assembly.product_definition_contexts.iter() {
                let entity_name = match pdc.kind {
                    ProductDefinitionContextKind::Plain => "PRODUCT_DEFINITION_CONTEXT",
                    ProductDefinitionContextKind::Design => "DESIGN_CONTEXT",
                };
                let ac_step = self.ac_step_ids[pdc.frame_of_reference.0 as usize];
                let id = self.push_simple(
                    entity_name,
                    vec![
                        Attribute::String(pdc.name.clone()),
                        Attribute::EntityRef(ac_step),
                        Attribute::String(pdc.life_cycle_stage.clone()),
                    ],
                );
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
        let app_ctx = self.push_simple("APPLICATION_CONTEXT", vec![Attribute::String(desc.into())]);
        let _ = self.push_simple(
            "APPLICATION_PROTOCOL_DEFINITION",
            vec![
                Attribute::String(status.into()),
                Attribute::String(name.into()),
                Attribute::Integer(year),
                Attribute::EntityRef(app_ctx),
            ],
        );
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

    pub(crate) fn emit_formation(&mut self, prod_entity: u64, product: &Product) -> u64 {
        // The `_WITH_SPECIFIED_SOURCE` subtype is selected by the loyalty
        // flag alone — AP203 readers always set it to `true` (the schema
        // mandates the subtype), AP214/242 set it only when the source
        // file used the subtype (e.g. some CATIA exports). The subtype
        // carries an extra `source` enum which the handler hardcodes to
        // `.NOT_KNOWN.` (the only value seen in real fixtures).
        use crate::entities::SimpleEntityHandler;
        use crate::entities::assembly_product::product_definition_formation::ProductDefinitionFormationHandler;
        use crate::entities::assembly_product::product_definition_formation_with_source::ProductDefinitionFormationWithSourceHandler;
        if product.formation_with_source {
            ProductDefinitionFormationWithSourceHandler::write(self, prod_entity)
                .expect("PDF_WITH_SOURCE write only pushes one simple entity")
        } else {
            ProductDefinitionFormationHandler::write(self, prod_entity)
                .expect("PDF write only pushes one simple entity")
        }
    }

    /// Emit the `PRODUCT_CATEGORY` chain for a product. No-op when the IR
    /// has `category = None` (kernel-built IR or AP214 CD minimal). Always
    /// emits `PRODUCT_RELATED_PRODUCT_CATEGORY`; emits `PRODUCT_CATEGORY` +
    /// `PRODUCT_CATEGORY_RELATIONSHIP` only when the source file had them
    /// (`category.root.is_some()`).
    fn emit_product_category_chain(&mut self, product: &Product, prod_entity: u64) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::assembly_product::product_category::{
            ProductCategoryHandler, ProductCategoryWriteInput,
        };
        use crate::entities::assembly_product::product_category_relationship::{
            ProductCategoryRelationshipHandler, ProductCategoryRelationshipWriteInput,
        };
        use crate::entities::assembly_product::product_related_product_category::{
            ProductRelatedProductCategoryHandler, ProductRelatedProductCategoryWriteInput,
        };

        let Some(category) = &product.category else {
            return;
        };
        let prpc = ProductRelatedProductCategoryHandler::write(
            self,
            ProductRelatedProductCategoryWriteInput {
                kind: category.kind.clone(),
                kind_description: category.kind_description.clone(),
                product_refs: vec![prod_entity],
            },
        )
        .expect("PRPC write only pushes one simple entity");
        if let Some(root) = &category.root {
            let pc = ProductCategoryHandler::write(
                self,
                ProductCategoryWriteInput {
                    name: root.name.clone(),
                    description: root.description.clone(),
                },
            )
            .expect("PRODUCT_CATEGORY write only pushes one simple entity");
            let _pcr = ProductCategoryRelationshipHandler::write(
                self,
                ProductCategoryRelationshipWriteInput {
                    pc_ref: pc,
                    prpc_ref: prpc,
                },
            )
            .expect("PCR write only pushes one simple entity");
        }
    }

    pub(crate) fn emit_pdef(&mut self, formation: u64, pdef_ctx: u64) -> u64 {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::assembly_product::product_definition::{
            ProductDefinitionHandler, ProductDefinitionWriteInput,
        };
        ProductDefinitionHandler::write(
            self,
            ProductDefinitionWriteInput {
                formation,
                pdef_ctx,
            },
        )
        .expect("PRODUCT_DEFINITION write only pushes one simple entity")
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
        let nauo_pds = self.emit_nauo_owned_pds(nauo);
        let rrwt = self.emit_rrwt_complex(parent_sr, child_sr, idt);
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

    pub(crate) fn emit_nauo(&mut self, inst: &Instance, parent_pdef: u64, child_pdef: u64) -> u64 {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::assembly_product::next_assembly_usage_occurrence::{
            NextAssemblyUsageOccurrenceHandler, NextAssemblyUsageOccurrenceWriteInput,
        };
        NextAssemblyUsageOccurrenceHandler::write(
            self,
            NextAssemblyUsageOccurrenceWriteInput {
                inst: inst.clone(),
                parent_pdef,
                child_pdef,
            },
        )
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

    fn emit_rrwt_complex(&mut self, parent_sr: u64, child_sr: u64, idt: u64) -> u64 {
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex {
                parts: vec![
                    (
                        "REPRESENTATION_RELATIONSHIP".into(),
                        vec![
                            Attribute::String(String::new()),
                            Attribute::String(String::new()),
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
