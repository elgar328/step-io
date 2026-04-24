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
use crate::ir::{Instance, Product, ProductContent, ProductId, Transform3d};
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
        let Some(unit_ctx) = self.global_unit_context_id else {
            return Ok(());
        };
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
        self.emit_assembly_chain(&products, unit_ctx, &schema)?;
        Ok(())
    }

    fn emit_assembly_chain(
        &mut self,
        products: &[Product],
        unit_ctx: u64,
        schema: &StepSchema,
    ) -> Result<(), WriteError> {
        let ctx = self.emit_application_context(schema);

        // Emit PRODUCT chain + shape representation + SDR for every product;
        // collect each product's PDEF and SR entity ids for later instance
        // wiring.
        let mut product_def: HashMap<ProductId, u64> = HashMap::new();
        let mut product_sr: HashMap<ProductId, u64> = HashMap::new();

        #[allow(clippy::cast_possible_truncation)]
        for (idx, product) in products.iter().enumerate() {
            let pid = ProductId(idx as u32);
            let prod_entity = self.emit_product(product, &ctx);
            let formation = self.emit_formation(prod_entity, schema);
            let pdef = self.emit_pdef(formation, ctx.pdef_ctx);
            let pdef_shape = self.emit_pdef_shape(pdef);
            product_def.insert(pid, pdef);

            let sr = match &product.content {
                ProductContent::Solid(sid) => {
                    let solid_ref = self.emit_solid(*sid)?;
                    self.emit_absr(product, solid_ref, unit_ctx)?
                }
                ProductContent::SurfaceBody(shells) => self.emit_mssr(product, shells, unit_ctx)?,
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
            let parent_pdef = product_def[&parent_pid];
            let parent_sr = product_sr[&parent_pid];
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

    fn emit_formation(&mut self, prod_entity: u64, schema: &StepSchema) -> u64 {
        // AP203 uses PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE with
        // an extra `source` enum (set to `.NOT_KNOWN.` — IR doesn't track it);
        // other schemas use the plain form.
        let name = if schema.class() == Some(SchemaClass::Ap203) {
            "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE"
        } else {
            "PRODUCT_DEFINITION_FORMATION"
        };
        let mut attrs = vec![
            Attribute::String(String::new()),
            Attribute::String(String::new()),
            Attribute::EntityRef(prod_entity),
        ];
        if schema.class() == Some(SchemaClass::Ap203) {
            attrs.push(Attribute::Enum("NOT_KNOWN".into()));
        }
        self.push_simple(name, attrs)
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

    fn push_simple(&mut self, name: &str, attrs: Vec<Attribute>) -> u64 {
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
