//! Property emission — `PROPERTY_DEFINITION` + `REPRESENTATION` +
//! `PROPERTY_DEFINITION_REPRESENTATION` chain for user-defined attributes.
//!
//! Recursive emit per [`Property`] — each property emits its measure items,
//! a wrapping `REPRESENTATION`, the `PROPERTY_DEFINITION` itself, and the
//! `PROPERTY_DEFINITION_REPRESENTATION` that ties them together. Mirrors
//! the visualization emit pattern.

use super::WriteBuffer;
use crate::entities::SimpleEntityHandler;
use crate::entities::property::property_definition_representation::{
    PropertyDefinitionRepresentationHandler, PropertyDefinitionRepresentationWriteInput,
};
use crate::ir::property::{Property, PropertyItem, PropertyMeasureUnit};
use crate::ir::shape_rep::DescriptiveItem;
use crate::parser::entity::Attribute;

impl WriteBuffer<'_> {
    /// Emit PD-based `SHAPE_DEFINITION_REPRESENTATION`s from the
    /// `shape_definition_representations` arena (phase sdr-arena-1). Must run
    /// after `emit_property_definitions_non_pds` (the `definition` PD's step
    /// id) and `emit_representations_pre_pass` (the `used_representation`).
    pub(in crate::writer::buffer) fn emit_sdr_links(&mut self) {
        let Some(pool) = self.model.properties.clone() else {
            return;
        };
        for link in pool.shape_definition_representations.iter() {
            let def_step = match link.definition {
                crate::ir::property::SdrDefinition::PropertyDefinition(id) => self
                    .property_definition_step_ids
                    .get(id.0 as usize)
                    .copied()
                    .unwrap_or(0),
                crate::ir::property::SdrDefinition::ShapeAspect(id) => self
                    .shape_aspect_step_ids
                    .get(id.0 as usize)
                    .copied()
                    .unwrap_or(0),
            };
            let sr_step = self
                .representation_step_ids
                .get(link.used_representation.0 as usize)
                .copied()
                .unwrap_or(0);
            if def_step == 0 || sr_step == 0 {
                continue;
            }
            self.emit_sdr(def_step, sr_step);
        }
    }

    /// Emit `PROPERTY_DEFINITION_REPRESENTATION`s from the
    /// `property_definition_representations` arena — those whose
    /// `used_representation` is an already-modelled representation. References
    /// the existing PD and representation step ids (no fresh REPRESENTATION).
    /// Same ordering constraints as [`Self::emit_sdr_links`]. Mirrors it.
    pub(in crate::writer::buffer) fn emit_pdr_links(&mut self) {
        let Some(pool) = self.model.properties.clone() else {
            return;
        };
        for link in pool.property_definition_representations.iter() {
            let pd = self
                .property_definition_step_ids
                .get(link.definition.0 as usize)
                .copied()
                .unwrap_or(0);
            let repr = self
                .representation_step_ids
                .get(link.used_representation.0 as usize)
                .copied()
                .unwrap_or(0);
            if pd == 0 || repr == 0 {
                continue;
            }
            let _ = PropertyDefinitionRepresentationHandler::write(
                self,
                PropertyDefinitionRepresentationWriteInput { pd, repr },
            );
        }
    }

    pub(in crate::writer::buffer) fn emit_properties_if_set(&mut self) {
        let Some(pool) = self.model.properties.clone() else {
            return;
        };
        self.property_step_ids = vec![0; pool.properties.len()];
        for (id, prop) in pool.properties.iter_with_ids() {
            let pd = self.emit_property(prop);
            self.property_step_ids[id.0 as usize] = pd;
        }
        self.emit_name_attributes(&pool);
        self.emit_description_attributes(&pool);
        self.emit_id_attributes(&pool);
        // emit_general_properties already ran before
        // emit_property_definitions_non_pds (general_property_step_ids filled).
        self.emit_general_property_associations(&pool);
        self.emit_dimensional_characteristic_representations(&pool);
    }

    /// Emit `DIMENSIONAL_CHARACTERISTIC_REPRESENTATION` entries (phase
    /// sdr-dcr). Depends on `dimensional_size_step_ids` /
    /// `dimensional_location_step_ids` (filled by the dimensional emits)
    /// and `representation_step_ids` (filled by
    /// `emit_representations_pre_pass`).
    fn emit_dimensional_characteristic_representations(
        &mut self,
        pool: &crate::ir::property::PropertyPool,
    ) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::property::dimensional_characteristic_representation::DimensionalCharacteristicRepresentationHandler;
        for dcr in pool.dimensional_characteristic_representations.iter() {
            let _ = DimensionalCharacteristicRepresentationHandler::write(self, dcr.clone());
        }
    }

    /// Emit every `GENERAL_PROPERTY` in arena order, caching step ids in
    /// `general_property_step_ids` for the GPA emitter.
    pub(in crate::writer::buffer) fn emit_general_properties(
        &mut self,
        pool: &crate::ir::PropertyPool,
    ) {
        use crate::entities::property::general_property::GeneralPropertyHandler;
        self.general_property_step_ids = vec![0; pool.general_properties.len()];
        for (id, gp) in pool.general_properties.iter_with_ids() {
            let step = GeneralPropertyHandler::write(self, gp.clone())
                .expect("GENERAL_PROPERTY write only pushes one simple entity");
            self.general_property_step_ids[id.0 as usize] = step;
        }
    }

    /// Emit every `GENERAL_PROPERTY_ASSOCIATION`, resolving `base_definition`
    /// and `derived_definition` through the cached step ids. A reference
    /// whose target was not emitted (0 slot) is skipped — symmetric with the
    /// reader, which drops a GPA whose refs do not resolve.
    fn emit_general_property_associations(&mut self, pool: &crate::ir::PropertyPool) {
        use crate::entities::property::general_property_association::{
            GeneralPropertyAssociationHandler, GeneralPropertyAssociationWriteInput,
        };
        use crate::ir::DerivedDefinitionItem;
        for gpa in pool.general_property_associations.iter() {
            let base_step = self.general_property_step_ids[gpa.base_definition.0 as usize];
            let derived_step = match gpa.derived_definition {
                DerivedDefinitionItem::PropertyDefinition(pd_id) => self
                    .property_definition_step_ids
                    .get(pd_id.0 as usize)
                    .copied()
                    .unwrap_or(0),
            };
            if base_step == 0 || derived_step == 0 {
                continue;
            }
            let _ = GeneralPropertyAssociationHandler::write(
                self,
                GeneralPropertyAssociationWriteInput {
                    gpa: gpa.clone(),
                    base_step,
                    derived_step,
                },
            );
        }
    }

    fn emit_id_attributes(&mut self, pool: &crate::ir::PropertyPool) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::property::id_attribute::{IdAttributeHandler, IdAttributeWriteInput};
        use crate::ir::IdAttributeItem;
        for attr in pool.id_attributes.iter() {
            let item_step = match attr.identified_item {
                IdAttributeItem::ShapeAspect(sa_ref) => {
                    let step = self.emit_shape_aspect_ref(sa_ref);
                    if step == 0 {
                        continue; // shape-aspect emit skipped — target didn't resolve.
                    }
                    step
                }
                IdAttributeItem::PropertyDefinition(pd_id) => {
                    let step = self
                        .property_definition_step_ids
                        .get(pd_id.0 as usize)
                        .copied()
                        .unwrap_or(0);
                    if step == 0 {
                        continue;
                    }
                    step
                }
                IdAttributeItem::Group(g_id) => {
                    let Some(&step) = self.plm_group_step_ids.get(g_id.0 as usize) else {
                        continue;
                    };
                    step
                }
                IdAttributeItem::Address(a_id) => {
                    let Some(&step) = self.plm_address_step_ids.get(a_id.0 as usize) else {
                        continue;
                    };
                    step
                }
                IdAttributeItem::ApplicationContext(ac_id) => {
                    let Some(&step) = self.ac_step_ids.get(ac_id.0 as usize) else {
                        continue;
                    };
                    step
                }
            };
            let _ = IdAttributeHandler::write(
                self,
                IdAttributeWriteInput {
                    attr: attr.clone(),
                    item_step,
                },
            );
        }
    }

    fn emit_name_attributes(&mut self, pool: &crate::ir::PropertyPool) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::property::name_attribute::{
            NameAttributeHandler, NameAttributeWriteInput,
        };
        use crate::ir::NameAttributeItem;
        for attr in pool.name_attributes.iter() {
            let item_step = match attr.named_item {
                NameAttributeItem::ProductDefinition(pid) => {
                    let Some(&step) = self.product_def_ids.get(&pid) else {
                        continue;
                    };
                    step
                }
                NameAttributeItem::DerivedUnit(du_id) => {
                    let Some(&step) = self.derived_unit_step_ids.get(du_id.0 as usize) else {
                        continue;
                    };
                    step
                }
            };
            let _ = NameAttributeHandler::write(
                self,
                NameAttributeWriteInput {
                    attr: attr.clone(),
                    item_step,
                },
            );
        }
    }

    fn emit_description_attributes(&mut self, pool: &crate::ir::PropertyPool) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::property::description_attribute::{
            DescriptionAttributeHandler, DescriptionAttributeWriteInput,
        };
        use crate::ir::DescriptionAttributeItem;
        for attr in pool.description_attributes.iter() {
            let item_step = match attr.described_item {
                DescriptionAttributeItem::PersonAndOrganization(pao_id) => {
                    let Some(&step) = self.plm_p_and_o_step_ids.get(pao_id.0 as usize) else {
                        continue;
                    };
                    step
                }
            };
            let _ = DescriptionAttributeHandler::write(
                self,
                DescriptionAttributeWriteInput {
                    attr: attr.clone(),
                    item_step,
                },
            );
        }
    }

    /// Emit a property's PD + REPRESENTATION + PDR chain. Returns the
    /// `PROPERTY_DEFINITION` step id, or 0 when the product chain was not
    /// emitted (the caller leaves a 0 slot in `property_step_ids`).
    fn emit_property(&mut self, prop: &Property) -> u64 {
        // PD is emitted by `emit_property_definitions_if_set` (arena-driven)
        // — here we only fetch its cached step id, then emit the items, the
        // wrapping REPRESENTATION, and the PDR that ties them together. A
        // 0 slot means the PD's product chain wasn't emitted (defensive
        // for kernel-built IR); the GPA emitter skips 0 slots downstream.
        let pd = self
            .property_definition_step_ids
            .get(prop.definition.0 as usize)
            .copied()
            .unwrap_or(0);
        if pd == 0 {
            return 0;
        }

        // 1. Emit items (mixed MEASURE / DESCRIPTIVE in source order).
        let item_refs: Vec<u64> = prop
            .items
            .iter()
            .map(|item| match item {
                PropertyItem::Descriptive(d) => self.emit_descriptive_item(d.clone()),
                PropertyItem::MeasureItem(id) => self.representation_item_step_ids[id.0 as usize],
            })
            .collect();

        // 2. REPRESENTATION wrapping the items.
        let ctx_attr = self.repr_context_attr(prop.context);
        let repr = self.push_simple(
            "REPRESENTATION",
            vec![
                Attribute::String(prop.representation_name.clone()),
                Attribute::List(item_refs.into_iter().map(Attribute::EntityRef).collect()),
                ctx_attr,
            ],
        );

        // 3. PROPERTY_DEFINITION_REPRESENTATION binding the (already
        // emitted) PD and the new REPRESENTATION.
        let _ = PropertyDefinitionRepresentationHandler::write(
            self,
            PropertyDefinitionRepresentationWriteInput { pd, repr },
        );
        pd
    }

    /// Emit the schema-faithful `property_definitions` arena — the writer's
    /// sole source for `PROPERTY_DEFINITION` and `PRODUCT_DEFINITION_SHAPE`
    /// entities. Arena order matches source `#N` (reader pushes in
    /// BTreeMap-driven dispatch order) so re-reading the output populates
    /// an identical arena. Also fills the `product_def_shape_ids` cache for
    /// PMI consumers (`SHAPE_ASPECT.of_shape` etc.) — replaces the legacy
    /// assembly-chain inline PDS emit. Runs after `emit_product_chain` (so
    /// `product_def_ids` is filled) and before `emit_pmi_if_set` (which
    /// consumes `product_def_shape_ids`).
    /// First half of the PD orchestrator. Emits only the
    /// `PRODUCT_DEFINITION_SHAPE` arena slots; fills
    /// `property_definition_step_ids` (zero-init) and mirrors each PDS
    /// into `product_def_shape_ids`. The non-PDS slots are deferred to
    /// [`emit_property_definitions_non_pds`] which must run after
    /// `emit_pmi_if_set` (so `shape_aspect_step_ids` is populated for
    /// Pattern B targets). Splitting the pass threads the dependency
    /// needle: SR/SDR + SA emit read `product_def_shape_ids` between the
    /// two halves.
    pub(in crate::writer::buffer) fn emit_property_definitions_pds_only(&mut self) {
        use crate::ir::property::{CharacterizedDefinition, PropertyDefinition};
        let pool_owned = self.model.properties.clone();
        let pool = match pool_owned.as_ref() {
            Some(p) if !p.property_definitions.is_empty() => p,
            _ => {
                self.emit_pds_fallback_from_product_chain();
                return;
            }
        };
        self.property_definition_step_ids = vec![0; pool.property_definitions.len()];
        for (idx, pd) in pool.property_definitions.iter_with_ids() {
            let PropertyDefinition::ProductDefinitionShape(pds) = pd else {
                continue;
            };
            let data = &pds.inherited;
            // A NAUO-owned PDS is emitted by the assembly chain (it needs the
            // freshly-minted NAUO step id). Record its arena slot + source
            // name/desc here; `emit_instance_bundle` emits the body and fills the
            // step-id slot. Leaving the slot 0 until then keeps reader/writer
            // symmetric: a skipped instance ⇒ slot 0 ⇒ dependent centroid PD
            // dropped (no dangling reference).
            if let CharacterizedDefinition::ProductDefinitionRelationship(acu_id) = data.definition
            {
                self.nauo_pds_arena_slot.insert(
                    acu_id,
                    (idx.0 as usize, data.name.clone(), data.description.clone()),
                );
                continue;
            }
            let CharacterizedDefinition::ProductDefinition(product_id) = data.definition else {
                continue;
            };
            let Some(&pdef_step) = self.product_def_ids.get(&product_id) else {
                continue;
            };
            let step = self.push_simple(
                "PRODUCT_DEFINITION_SHAPE",
                vec![
                    Attribute::String(data.name.clone()),
                    Attribute::String(data.description.clone()),
                    Attribute::EntityRef(pdef_step),
                ],
            );
            self.property_definition_step_ids[idx.0 as usize] = step;
            self.product_def_shape_ids.insert(product_id, step);
        }
    }

    /// Second half of the PD orchestrator. Emits only the
    /// `PROPERTY_DEFINITION` (Itself variant) arena slots; both
    /// `ProductDefinition` and `ShapeAspect` targets resolve here.
    /// Preserves slots filled by [`emit_property_definitions_pds_only`].
    #[allow(clippy::too_many_lines)]
    pub(in crate::writer::buffer) fn emit_property_definitions_non_pds(&mut self) {
        use crate::ir::property::{CharacterizedDefinition, PropertyDefinition};
        let pool_owned = self.model.properties.clone();
        let Some(pool) = pool_owned.as_ref() else {
            return;
        };
        if pool.property_definitions.is_empty() {
            return;
        }
        if self.property_definition_step_ids.len() != pool.property_definitions.len() {
            self.property_definition_step_ids
                .resize(pool.property_definitions.len(), 0);
        }
        for (idx, pd) in pool.property_definitions.iter_with_ids() {
            let PropertyDefinition::Itself(data) = pd else {
                continue;
            };
            let target_step = match data.definition {
                CharacterizedDefinition::ProductDefinition(product_id) => {
                    let Some(&s) = self.product_def_ids.get(&product_id) else {
                        continue;
                    };
                    s
                }
                // An `Itself` PD never carries a `ProductDefinitionRelationship`
                // definition (that member only labels the NAUO-owned
                // `ProductDefinitionShape`, emitted by the assembly chain).
                CharacterizedDefinition::ProductDefinitionRelationship(_) => continue,
                CharacterizedDefinition::ShapeAspect(sa_ref) => {
                    let s = self.emit_shape_aspect_ref(sa_ref);
                    if s == 0 {
                        continue;
                    }
                    s
                }
                CharacterizedDefinition::DimensionalLocation(dl_id) => {
                    let s = self
                        .dimensional_location_step_ids
                        .get(dl_id.0 as usize)
                        .copied()
                        .unwrap_or(0);
                    if s == 0 {
                        continue;
                    }
                    s
                }
                CharacterizedDefinition::ProductDefinitionShape(pds_pd_id) => {
                    // PDS arena entry's step id was cached in Pass A
                    // (emit_property_definitions_pds_only). Slot 0 means the
                    // PDS itself failed to emit — skip this PD too.
                    let s = self
                        .property_definition_step_ids
                        .get(pds_pd_id.0 as usize)
                        .copied()
                        .unwrap_or(0);
                    if s == 0 {
                        continue;
                    }
                    s
                }
                CharacterizedDefinition::GeneralProperty(gp_id) => {
                    // GENERAL_PROPERTY step ids are filled by
                    // emit_general_properties, which must run before this pass.
                    let s = self
                        .general_property_step_ids
                        .get(gp_id.0 as usize)
                        .copied()
                        .unwrap_or(0);
                    if s == 0 {
                        continue;
                    }
                    s
                }
                CharacterizedDefinition::Document(doc_id) => {
                    // DOCUMENT_FILE step ids are filled by emit_documents_prepass,
                    // which must run before this pass.
                    let s = self
                        .plm_document_step_ids
                        .get(doc_id.0 as usize)
                        .copied()
                        .unwrap_or(0);
                    if s == 0 {
                        continue;
                    }
                    s
                }
                CharacterizedDefinition::CharacterizedItemWithinRepresentation(co_id) => {
                    // CIWR step ids are reserved by emit_characterized_objects_prepass
                    // before this pass; the CO body emits later under the reserved id.
                    // 0 = inline-DM CO (out of scope) → skip symmetrically.
                    let s = self
                        .characterized_object_step_ids
                        .get(co_id.0 as usize)
                        .copied()
                        .unwrap_or(0);
                    if s == 0 {
                        continue;
                    }
                    s
                }
                CharacterizedDefinition::CharacterizedObject(co_id) => {
                    // MBD draughting-model CO facet — reserved by
                    // emit_characterized_objects_prepass; the DM complex body
                    // emits later under this same shared id.
                    let s = self
                        .characterized_object_step_ids
                        .get(co_id.0 as usize)
                        .copied()
                        .unwrap_or(0);
                    if s == 0 {
                        continue;
                    }
                    s
                }
                CharacterizedDefinition::GeometricTolerance(gt_ref) => {
                    // geometric_tolerance(_with_datum_reference) step ids are
                    // filled by emit_geometric_tolerances(+_with_datum), moved
                    // before this pass.
                    use crate::ir::pmi::GeometricToleranceRef;
                    let s = match gt_ref {
                        GeometricToleranceRef::Plain(id) => {
                            self.geometric_tolerance_step_ids.get(id.0 as usize)
                        }
                        GeometricToleranceRef::WithDatumReference(id) => self
                            .geometric_tolerance_with_datum_reference_step_ids
                            .get(id.0 as usize),
                    }
                    .copied()
                    .unwrap_or(0);
                    if s == 0 {
                        continue;
                    }
                    s
                }
                CharacterizedDefinition::DimensionalSize(ds_id) => {
                    // dimensional_size step ids are filled by
                    // emit_dimensional_sizes, which runs before this pass.
                    let s = self
                        .dimensional_size_step_ids
                        .get(ds_id.0 as usize)
                        .copied()
                        .unwrap_or(0);
                    if s == 0 {
                        continue;
                    }
                    s
                }
            };
            let step = self.push_simple(
                "PROPERTY_DEFINITION",
                vec![
                    Attribute::String(data.name.clone()),
                    Attribute::String(data.description.clone()),
                    Attribute::EntityRef(target_step),
                ],
            );
            self.property_definition_step_ids[idx.0 as usize] = step;
        }
    }

    /// Emit one `PRODUCT_DEFINITION_SHAPE` per product whose PDEF was
    /// already cached in `product_def_ids`, mirroring the pre-phase-E'
    /// assembly chain behaviour. Used only when the IR carries no
    /// `property_definitions` arena (hand/kernel-built IR).
    fn emit_pds_fallback_from_product_chain(&mut self) {
        use crate::entities::SimpleEntityHandler;
        use crate::entities::assembly_product::product_definition_shape::{
            ProductDefinitionShapeHandler, ProductDefinitionShapeWriteInput,
        };
        let pdef_entries: Vec<(crate::ir::id::ProductId, u64)> = self
            .product_def_ids
            .iter()
            .map(|(pid, pdef)| (*pid, *pdef))
            .collect();
        for (pid, pdef) in pdef_entries {
            if let Ok(step) = ProductDefinitionShapeHandler::write(
                self,
                ProductDefinitionShapeWriteInput { pdef },
            ) {
                self.product_def_shape_ids.insert(pid, step);
            }
        }
    }

    /// Emit a `DESCRIPTIVE_REPRESENTATION_ITEM` for a property's
    /// descriptive item. Wraps the `shape_rep` handler so callers don't
    /// need to drag the trait import through every site.
    pub(crate) fn emit_descriptive_item(&mut self, item: DescriptiveItem) -> u64 {
        use crate::entities::SimpleEntityHandler;
        crate::entities::shape_rep::descriptive_representation_item::DescriptiveRepresentationItemHandler::write(self, item).expect("descriptive item emit is infallible")
    }

    /// Resolve an explicit [`PropertyMeasureUnit`] to its emitted STEP id.
    pub(in crate::writer::buffer) fn resolve_explicit_unit_ref(
        &self,
        unit_ref: Option<PropertyMeasureUnit>,
    ) -> Option<u64> {
        match unit_ref? {
            PropertyMeasureUnit::Named(id) => self.named_unit_step_ids.get(id.0 as usize).copied(),
            PropertyMeasureUnit::Derived(id) => {
                self.derived_unit_step_ids.get(id.0 as usize).copied()
            }
        }
    }
}
