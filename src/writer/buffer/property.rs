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
        for prop in &pool.properties {
            self.emit_property(prop);
        }
    }

    fn emit_property(&mut self, prop: &Property) {
        // Defensive: silent skip when product chain wasn't emitted (e.g.,
        // kernel-built IR with `properties` populated but no assembly).
        // Reader symmetry — reader silent skips PDs whose target wasn't a
        // resolvable Product.
        let Some(&pdef_id) = self.product_def_ids.get(&prop.target) else {
            return;
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
