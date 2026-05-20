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
                DerivedDefinitionItem::PropertyDefinition(pid) => {
                    self.property_step_ids[pid.0 as usize]
                }
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
        // Defensive: silent skip when product chain wasn't emitted (e.g.,
        // kernel-built IR with `properties` populated but no assembly).
        // Reader symmetry — reader silent skips PDs whose target wasn't a
        // resolvable Product.
        let Some(&pdef_id) = self.product_def_ids.get(&prop.target) else {
            return 0;
        };

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

        // 3. PROPERTY_DEFINITION itself.
        let desc_attr = match &prop.description {
            Some(s) => Attribute::String(s.clone()),
            None => Attribute::Unset,
        };
        let pd = self.push_simple(
            "PROPERTY_DEFINITION",
            vec![
                Attribute::String(prop.name.clone()),
                desc_attr,
                Attribute::EntityRef(pdef_id),
            ],
        );

        // 4. PROPERTY_DEFINITION_REPRESENTATION binding the two.
        let _ = PropertyDefinitionRepresentationHandler::write(
            self,
            PropertyDefinitionRepresentationWriteInput { pd, repr },
        );
        pd
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
