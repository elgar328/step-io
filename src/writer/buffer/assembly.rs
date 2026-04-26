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
struct AssemblyContextIds {
    product_ctx: u64,
    pdef_ctx: u64,
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

    /// Resolve a product's `geometry_context` to a STEP entity id, falling
    /// back to the first emitted context when the IR has no explicit ref.
    /// Caller is responsible for skipping product emission entirely when
    /// `unit_context_ids` is empty (see `emit_product_chain_if_eligible`).
    fn resolve_product_ctx(&self, product: &Product) -> u64 {
        if let Some(id) = product.geometry_context {
            return self.unit_context_ids[id.0 as usize];
        }
        *self
            .unit_context_ids
            .first()
            .expect("emit_product_chain_if_eligible bails out when no contexts exist")
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
            let pdef_shape = self.emit_pdef_shape(pdef);
            self.product_def_ids.insert(pid, pdef);
            self.emit_product_category_chain(product, prod_entity);

            let unit_ctx = self.resolve_product_ctx(product);
            let sr = match &product.content {
                ProductContent::Solid(sid) => {
                    let solid_ref = self.emit_solid(*sid)?;
                    let absr = self.emit_absr(product, solid_ref, unit_ctx)?;
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
            let parent_sr = product_sr[&parent_pid];
            // Clone product_def_ids for emit_instance_bundle so it can
            // borrow `&self` via emit_* methods without overlapping borrows.
            let product_def = self.product_def_ids.clone();
            for inst in instances {
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

    fn emit_product(&mut self, product: &Product, ctx: &AssemblyContextIds) -> u64 {
        self.push_simple(
            "PRODUCT",
            vec![
                Attribute::String(product.id.clone()),
                Attribute::String(product.name.clone()),
                Attribute::String(product.description.clone().unwrap_or_default()),
                Attribute::List(vec![Attribute::EntityRef(ctx.product_ctx)]),
            ],
        )
    }

    fn emit_formation(&mut self, prod_entity: u64, product: &Product) -> u64 {
        // The `_WITH_SPECIFIED_SOURCE` subtype is selected by the loyalty
        // flag alone — AP203 readers always set it to `true` (the schema
        // mandates the subtype), AP214/242 set it only when the source
        // file used the subtype (e.g. some CATIA exports). The subtype
        // carries an extra `source` enum which we hardcode to
        // `.NOT_KNOWN.` (the only value seen in real fixtures).
        let name = if product.formation_with_source {
            "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE"
        } else {
            "PRODUCT_DEFINITION_FORMATION"
        };
        let mut attrs = vec![
            Attribute::String(String::new()),
            Attribute::String(String::new()),
            Attribute::EntityRef(prod_entity),
        ];
        if product.formation_with_source {
            attrs.push(Attribute::Enum("NOT_KNOWN".into()));
        }
        self.push_simple(name, attrs)
    }

    /// Emit the `PRODUCT_CATEGORY` chain for a product. No-op when the IR
    /// has `category = None` (kernel-built IR or AP214 CD minimal). Always
    /// emits `PRODUCT_RELATED_PRODUCT_CATEGORY`; emits `PRODUCT_CATEGORY` +
    /// `PRODUCT_CATEGORY_RELATIONSHIP` only when the source file had them
    /// (`category.root.is_some()`).
    fn emit_product_category_chain(&mut self, product: &Product, prod_entity: u64) {
        let Some(category) = &product.category else {
            return;
        };
        let prpc_desc = category
            .kind_description
            .as_ref()
            .map_or(Attribute::Unset, |s| Attribute::String(s.clone()));
        let prpc = self.push_simple(
            "PRODUCT_RELATED_PRODUCT_CATEGORY",
            vec![
                Attribute::String(category.kind.clone()),
                prpc_desc,
                Attribute::List(vec![Attribute::EntityRef(prod_entity)]),
            ],
        );
        if let Some(root) = &category.root {
            let pc_desc = root
                .description
                .as_ref()
                .map_or(Attribute::Unset, |s| Attribute::String(s.clone()));
            let pc = self.push_simple(
                "PRODUCT_CATEGORY",
                vec![Attribute::String(root.name.clone()), pc_desc],
            );
            let _pcr = self.push_simple(
                "PRODUCT_CATEGORY_RELATIONSHIP",
                vec![
                    Attribute::String(" ".into()),
                    Attribute::String(" ".into()),
                    Attribute::EntityRef(pc),
                    Attribute::EntityRef(prpc),
                ],
            );
        }
    }

    fn emit_pdef(&mut self, formation: u64, pdef_ctx: u64) -> u64 {
        self.push_simple(
            "PRODUCT_DEFINITION",
            vec![
                Attribute::String("design".into()),
                Attribute::String(String::new()),
                Attribute::EntityRef(formation),
                Attribute::EntityRef(pdef_ctx),
            ],
        )
    }

    fn emit_pdef_shape(&mut self, pdef: u64) -> u64 {
        self.push_simple(
            "PRODUCT_DEFINITION_SHAPE",
            vec![
                Attribute::String(String::new()),
                Attribute::String(String::new()),
                Attribute::EntityRef(pdef),
            ],
        )
    }

    fn emit_shell_based_surface_model(
        &mut self,
        shells: &[crate::ir::ShellId],
    ) -> Result<u64, WriteError> {
        let mut shell_refs = Vec::with_capacity(shells.len());
        for &s in shells {
            shell_refs.push(Attribute::EntityRef(self.emit_shell(s)?));
        }
        Ok(self.push_simple(
            "SHELL_BASED_SURFACE_MODEL",
            vec![
                Attribute::String(String::new()),
                Attribute::List(shell_refs),
            ],
        ))
    }

    fn emit_mssr(
        &mut self,
        product: &Product,
        shells: &[crate::ir::ShellId],
        unit_ctx: u64,
    ) -> Result<u64, WriteError> {
        let axis_ref = self.emit_axis2_placement_3d(product.shape_ref_frame)?;
        let sbsm_ref = self.emit_shell_based_surface_model(shells)?;
        Ok(self.push_simple(
            "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
            vec![
                Attribute::String(String::new()),
                Attribute::List(vec![
                    Attribute::EntityRef(axis_ref),
                    Attribute::EntityRef(sbsm_ref),
                ]),
                Attribute::EntityRef(unit_ctx),
            ],
        ))
    }

    /// Emit a wireframe `SHAPE_REPRESENTATION` (`GBWSR` / `GBSSR`) plus its
    /// `GEOMETRIC_(CURVE_)SET` of items. Picks the `..._SURFACE_...` cousin
    /// when `repr_kind == Surface` (CATIA convention) and uses
    /// `GEOMETRIC_SET` when loose points coexist with curves; otherwise
    /// emits the more common `..._WIREFRAME_...` + `GEOMETRIC_CURVE_SET`.
    fn emit_wireframe_representation(
        &mut self,
        product: &Product,
        wf: &WireframeContent,
        unit_ctx: u64,
    ) -> Result<u64, WriteError> {
        let axis_ref = self.emit_axis2_placement_3d(product.shape_ref_frame)?;
        let mut item_refs = Vec::with_capacity(wf.curves.len() + wf.points.len());
        for &cid in &wf.curves {
            item_refs.push(Attribute::EntityRef(self.emit_curve(cid)?));
        }
        for &pid in &wf.points {
            item_refs.push(Attribute::EntityRef(self.emit_point(pid)?));
        }
        let set_name = if wf.points.is_empty() {
            "GEOMETRIC_CURVE_SET"
        } else {
            "GEOMETRIC_SET"
        };
        let set_ref = self.push_simple(
            set_name,
            vec![Attribute::String(String::new()), Attribute::List(item_refs)],
        );
        let repr_name = match wf.repr_kind {
            WireframeReprKind::Surface => "GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION",
            WireframeReprKind::Wireframe => "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION",
        };
        Ok(self.push_simple(
            repr_name,
            vec![
                Attribute::String(String::new()),
                Attribute::List(vec![
                    Attribute::EntityRef(axis_ref),
                    Attribute::EntityRef(set_ref),
                ]),
                Attribute::EntityRef(unit_ctx),
            ],
        ))
    }

    fn emit_absr(
        &mut self,
        product: &Product,
        solid_ref: u64,
        unit_ctx: u64,
    ) -> Result<u64, WriteError> {
        let axis_ref = self.emit_axis2_placement_3d(product.shape_ref_frame)?;
        Ok(self.push_simple(
            "ADVANCED_BREP_SHAPE_REPRESENTATION",
            vec![
                Attribute::String(String::new()),
                Attribute::List(vec![
                    Attribute::EntityRef(axis_ref),
                    Attribute::EntityRef(solid_ref),
                ]),
                Attribute::EntityRef(unit_ctx),
            ],
        ))
    }

    /// Emit a `SHAPE_REPRESENTATION` for a Group product — no geometry,
    /// just the product's coordinate reference frame so the entity is
    /// well-formed.
    fn emit_group_shape_representation(
        &mut self,
        product: &Product,
        unit_ctx: u64,
    ) -> Result<u64, WriteError> {
        let axis_ref = self.emit_axis2_placement_3d(product.shape_ref_frame)?;
        Ok(self.push_simple(
            "SHAPE_REPRESENTATION",
            vec![
                Attribute::String(String::new()),
                Attribute::List(vec![Attribute::EntityRef(axis_ref)]),
                Attribute::EntityRef(unit_ctx),
            ],
        ))
    }

    fn emit_sdr(&mut self, pdef_shape: u64, sr: u64) -> u64 {
        self.push_simple(
            "SHAPE_DEFINITION_REPRESENTATION",
            vec![Attribute::EntityRef(pdef_shape), Attribute::EntityRef(sr)],
        )
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

    fn emit_simple_srr(&mut self, rep_1: u64, rep_2: u64) -> u64 {
        self.push_simple(
            "SHAPE_REPRESENTATION_RELATIONSHIP",
            vec![
                Attribute::String("SRR".into()),
                Attribute::String("None".into()),
                Attribute::EntityRef(rep_1),
                Attribute::EntityRef(rep_2),
            ],
        )
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
        let _cdsr = self.emit_cdsr(rrwt, nauo_pds);
        Ok(())
    }

    fn emit_item_defined_transformation(
        &mut self,
        transform: Transform3d,
    ) -> Result<u64, WriteError> {
        let source = self.emit_axis2_placement_3d(transform.source)?;
        let target = self.emit_axis2_placement_3d(transform.target)?;
        Ok(self.push_simple(
            "ITEM_DEFINED_TRANSFORMATION",
            vec![
                Attribute::String(String::new()),
                Attribute::String(String::new()),
                Attribute::EntityRef(source),
                Attribute::EntityRef(target),
            ],
        ))
    }

    fn emit_nauo(&mut self, inst: &Instance, parent_pdef: u64, child_pdef: u64) -> u64 {
        self.push_simple(
            "NEXT_ASSEMBLY_USAGE_OCCURRENCE",
            vec![
                Attribute::String(inst.occurrence_id.clone()),
                Attribute::String(inst.occurrence_name.clone()),
                Attribute::String(String::new()),
                Attribute::EntityRef(parent_pdef),
                Attribute::EntityRef(child_pdef),
                Attribute::Unset,
            ],
        )
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

    fn emit_cdsr(&mut self, rrwt: u64, nauo_pds: u64) -> u64 {
        self.push_simple(
            "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION",
            vec![Attribute::EntityRef(rrwt), Attribute::EntityRef(nauo_pds)],
        )
    }

    // ----------------------------------------------------------------
    // Low-level
    // ----------------------------------------------------------------

    pub(in crate::writer::buffer) fn push_simple(
        &mut self,
        name: &str,
        attrs: Vec<Attribute>,
    ) -> u64 {
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
