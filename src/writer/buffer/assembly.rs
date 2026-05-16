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
use crate::ir::{
    Instance, Product, ProductContent, ProductId, Transform3d, WireframeContent, WireframeReprKind,
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
        let products: Vec<Product> = assembly.products.iter().cloned().collect();
        self.emit_assembly_chain(&products, &schema)?;
        Ok(())
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
        products: &[Product],
        schema: &StepSchema,
    ) -> Result<(), WriteError> {
        let ctx = self.emit_application_context(schema);

        // Emit PRODUCT chain + shape representation + SDR for every product;
        // collect each product's PDEF and SR entity ids for later instance
        // wiring. PDEF ids land in `self.product_def_ids` so the property
        // emitter (which runs after this pass) can resolve a
        // `Property.target` ProductId to a STEP ref.
        let mut product_sr: HashMap<ProductId, u64> = HashMap::new();

        #[allow(clippy::cast_possible_truncation)]
        for (idx, product) in products.iter().enumerate() {
            let pid = ProductId(idx as u32);
            let prod_entity = self.emit_product(product, &ctx);
            let formation = self.emit_formation(prod_entity, product);
            let pdef = self.emit_pdef(formation, ctx.pdef_ctx);
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
            let sr = match &product.content {
                ProductContent::Solid(sids) => {
                    let solid_refs: Vec<u64> = sids
                        .iter()
                        .map(|sid| self.emit_solid(*sid))
                        .collect::<Result<_, _>>()?;
                    let absr = self.emit_absr(product, solid_refs, unit_ctx)?;
                    self.wrap_indirect_sr_if_set(product, absr, unit_ctx)?
                }
                ProductContent::SurfaceBody(shells) => {
                    let mssr = self.emit_mssr(product, shells, unit_ctx)?;
                    self.wrap_indirect_sr_if_set(product, mssr, unit_ctx)?
                }
                ProductContent::Wireframe(wf) => {
                    let gb_sr = self.emit_wireframe_representation(product, wf, unit_ctx)?;
                    self.wrap_indirect_sr_if_set(product, gb_sr, unit_ctx)?
                }
                ProductContent::Group(_) => {
                    self.emit_group_shape_representation(product, unit_ctx)?
                }
            };
            product_sr.insert(pid, sr);
            self.emit_sdr(pdef_shape, sr);
        }

        // Emit per-instance bundles for every Group product.
        #[allow(clippy::cast_possible_truncation)]
        for (parent_idx, parent) in products.iter().enumerate() {
            let ProductContent::Group(instances) = &parent.content else {
                continue;
            };
            let parent_pid = ProductId(parent_idx as u32);
            let parent_pdef = self.product_def_ids[&parent_pid];
            // No SR for this parent → no shape to anchor instances on.
            // Reached when the parent is a metadata-only Group([]) whose
            // SR emission was skipped above.
            let Some(&parent_sr) = product_sr.get(&parent_pid) else {
                continue;
            };
            // Clone product_def_ids for emit_instance_bundle so it can
            // borrow `&self` via emit_* methods without overlapping borrows.
            let product_def = self.product_def_ids.clone();
            for inst in instances {
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
    // Context
    // ----------------------------------------------------------------

    fn emit_application_context(&mut self, schema: &StepSchema) -> AssemblyContextIds {
        let (desc, status, name, year) = apd_info(schema);
        let app_ctx = self.push_simple("APPLICATION_CONTEXT", vec![Attribute::String(desc.into())]);
        let _apd = self.push_simple(
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
