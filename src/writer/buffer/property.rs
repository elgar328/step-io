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
use crate::ir::id::UnitContextId;
use crate::ir::property::{
    MeasureKind, Property, PropertyItem, PropertyMeasure, PropertyMeasureUnit,
};
use crate::ir::shape_rep::DescriptiveItem;
use crate::parser::entity::Attribute;

impl WriteBuffer<'_> {
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
        self.emit_general_properties(&pool);
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
    fn emit_general_properties(&mut self, pool: &crate::ir::PropertyPool) {
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
                IdAttributeItem::ShapeAspect(sa_id) => {
                    let Some(&step) = self.shape_aspect_step_ids.get(sa_id.0 as usize) else {
                        continue;
                    };
                    if step == 0 {
                        continue; // SA emit skipped — its target didn't resolve.
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
                PropertyItem::Measure(m) => self.emit_property_measure(m, prop.context),
                PropertyItem::Descriptive(d) => self.emit_descriptive_item(d.clone()),
            })
            .collect();

        // 2. REPRESENTATION wrapping the items.
        let ctx_attr = match prop.context {
            Some(id) => Attribute::EntityRef(self.unit_context_ids[id.0 as usize]),
            None => Attribute::Unset,
        };
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
    pub(in crate::writer::buffer) fn emit_property_definitions_if_set(&mut self) {
        use crate::ir::property::{CharacterizedDefinition, PropertyDefinition};
        let pool_owned = self.model.properties.clone();
        let pool = match pool_owned.as_ref() {
            Some(p) if !p.property_definitions.is_empty() => p,
            _ => {
                // Hand/kernel-built IR with no `property_definitions` arena
                // — fall back to one PDS per geometry-bearing product so
                // the SDR / PMI consumers still see a populated
                // `product_def_shape_ids` cache. Re-read fills the arena
                // (reader's PDS handler unconditionally mirrors into it);
                // this fallback only affects the *first* write of a hand-
                // built IR, where the user did not pre-populate the arena.
                self.emit_pds_fallback_from_product_chain();
                return;
            }
        };
        self.property_definition_step_ids = vec![0; pool.property_definitions.len()];
        for (idx, pd) in pool.property_definitions.iter_with_ids() {
            let (entity_name, data) = match pd {
                PropertyDefinition::Itself(data) => ("PROPERTY_DEFINITION", data),
                PropertyDefinition::ProductDefinitionShape(pds) => {
                    ("PRODUCT_DEFINITION_SHAPE", &pds.inherited)
                }
            };
            let CharacterizedDefinition::ProductDefinition(product_id) = data.definition;
            let Some(&pdef_step) = self.product_def_ids.get(&product_id) else {
                continue; // product chain not emitted — leave slot 0
            };
            let step = self.push_simple(
                entity_name,
                vec![
                    Attribute::String(data.name.clone()),
                    Attribute::String(data.description.clone()),
                    Attribute::EntityRef(pdef_step),
                ],
            );
            self.property_definition_step_ids[idx.0 as usize] = step;
            if matches!(pd, PropertyDefinition::ProductDefinitionShape(_)) {
                // Mirror into the legacy PMI consumer cache. Multiple PDS
                // per product would overwrite — observed corpora carry one
                // product-targeted PDS per product, so this matches the
                // existing assembly-chain behaviour.
                self.product_def_shape_ids.insert(product_id, step);
            }
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

    pub(crate) fn emit_property_measure(
        &mut self,
        m: &PropertyMeasure,
        ctx: Option<UnitContextId>,
    ) -> u64 {
        let typed_name = match m.kind {
            MeasureKind::Length => "LENGTH_MEASURE",
            MeasureKind::PlaneAngle => "PLANE_ANGLE_MEASURE",
            MeasureKind::SolidAngle => "SOLID_ANGLE_MEASURE",
            MeasureKind::PositiveRatio => "POSITIVE_RATIO_MEASURE",
            MeasureKind::Mass => "MASS_MEASURE",
            MeasureKind::Area => "AREA_MEASURE",
            MeasureKind::Volume => "VOLUME_MEASURE",
        };
        let unit_ref = self
            .resolve_explicit_unit_ref(m.unit_ref)
            .unwrap_or_else(|| self.resolve_property_unit_ref(ctx, m.kind));
        self.push_simple(
            "MEASURE_REPRESENTATION_ITEM",
            vec![
                Attribute::String(m.name.clone()),
                Attribute::Typed {
                    type_name: typed_name.into(),
                    value: Box::new(Attribute::Real(m.value)),
                },
                Attribute::EntityRef(unit_ref),
            ],
        )
    }

    /// Resolve an explicit [`PropertyMeasureUnit`] to its emitted STEP id.
    /// `None` falls through to the legacy context-based lookup.
    fn resolve_explicit_unit_ref(&self, unit_ref: Option<PropertyMeasureUnit>) -> Option<u64> {
        match unit_ref? {
            PropertyMeasureUnit::Named(id) => self.named_unit_step_ids.get(id.0 as usize).copied(),
            PropertyMeasureUnit::Derived(id) => {
                self.derived_unit_step_ids.get(id.0 as usize).copied()
            }
        }
    }

    /// Pick the unit-leaf STEP id matching this measure's kind and the
    /// property's `UnitContext`. The `unit_leaf_ids` vec is populated by
    /// the units emit pass that runs before properties in `emit_all`, so
    /// indexing is always safe — upstream guards (assembly skip on empty
    /// `unit_context_ids`, property skip on missing `product_def_ids`)
    /// ensure this path is only reached when units are present.
    ///
    /// `PositiveRatio` reaches this path only for kernel-built IR without
    /// an explicit `unit_ref` — falls back to the length leaf (lossy).
    fn resolve_property_unit_ref(&self, ctx: Option<UnitContextId>, kind: MeasureKind) -> u64 {
        let ctx_idx = ctx.unwrap_or(UnitContextId(0)).0 as usize;
        let (length, angle, solid) = self.unit_leaf_ids[ctx_idx];
        match kind {
            MeasureKind::Length
            | MeasureKind::PositiveRatio
            | MeasureKind::Mass
            | MeasureKind::Area
            | MeasureKind::Volume => length,
            MeasureKind::PlaneAngle => angle,
            MeasureKind::SolidAngle => solid,
        }
    }
}
