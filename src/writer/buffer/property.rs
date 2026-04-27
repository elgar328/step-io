//! Property emission — `PROPERTY_DEFINITION` + `REPRESENTATION` +
//! `PROPERTY_DEFINITION_REPRESENTATION` chain for user-defined attributes.
//!
//! Recursive emit per [`Property`] — each property emits its measure items,
//! a wrapping `REPRESENTATION`, the `PROPERTY_DEFINITION` itself, and the
//! `PROPERTY_DEFINITION_REPRESENTATION` that ties them together. Mirrors
//! the visualization emit pattern.

use super::WriteBuffer;
use crate::ir::id::UnitContextId;
use crate::ir::property::{MeasureKind, Property, PropertyMeasure};
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

        // 1. Emit MEASURE items.
        let item_refs: Vec<u64> = prop
            .items
            .iter()
            .map(|m| self.emit_property_measure(m, prop.context))
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
        self.push_simple(
            "PROPERTY_DEFINITION_REPRESENTATION",
            vec![Attribute::EntityRef(pd), Attribute::EntityRef(repr)],
        );
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
        };
        let unit_ref = self.resolve_property_unit_ref(ctx, m.kind);
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

    /// Pick the cached unit-leaf STEP id matching this measure's kind. Falls
    /// back to the property's `context` `UnitContext` when set, or — if the
    /// kernel-built IR omitted the context — the first emitted `UnitContext`.
    /// The caches are populated by the unit-emit pass that ran earlier in
    /// `emit_all`, so the ids are always present once any unit context has
    /// been emitted.
    fn resolve_property_unit_ref(&self, ctx: Option<UnitContextId>, kind: MeasureKind) -> u64 {
        // We don't currently model which UnitContext a property's units came
        // from in the leaf-id maps (those are keyed by IR fields, not by
        // ctx id). Look up the unit by IR fields of the chosen ctx.
        let ctx_id = ctx.unwrap_or(UnitContextId(0));
        let ctx_idx = ctx_id.0 as usize;
        let units = &self.model.units[ctx_id];
        match kind {
            MeasureKind::Length => *self
                .length_unit_ids
                .get(&(units.length, units.length_cbu_wrapped))
                .or_else(|| self.unit_context_ids.get(ctx_idx))
                .expect("length unit emitted with the unit context"),
            MeasureKind::PlaneAngle => *self
                .angle_unit_ids
                .get(&(units.plane_angle, units.plane_angle_cbu_wrapped))
                .or_else(|| self.unit_context_ids.get(ctx_idx))
                .expect("angle unit emitted with the unit context"),
            MeasureKind::SolidAngle => *self
                .solid_angle_unit_ids
                .get(&units.solid_angle)
                .or_else(|| self.unit_context_ids.get(ctx_idx))
                .expect("solid-angle unit emitted with the unit context"),
        }
    }
}
