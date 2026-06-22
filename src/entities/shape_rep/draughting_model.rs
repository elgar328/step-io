//! `DRAUGHTING_MODEL` handler — 2-layer.
//!
//! `representation` subtype carrying a `set_select` of
//! `draughting_model_item_select` members. The plain `DRAUGHTING_MODEL(name,
//! items, context)` form is read here (`bind` + `lower_draughting_model`,
//! always `Form::Simple`); the complex-MI forms `(CHARACTERIZED_OBJECT ...
//! DRAUGHTING_MODEL ...)` are read by `characterized_object_complex`.
//!
//! `write` (driven by `emit_draughting_models` after the item-ref caches are
//! populated) dispatches `dm.form` 4-way through lift + generated serialize:
//! `Simple` → `serialize_draughting_model`; the three complex forms →
//! `serialize_characterized_object_complex(_with_id)`. The complex forms emit
//! the `CHARACTERIZED_OBJECT` part as `(*, *)` and reuse the step id reserved
//! for the CO facet (so a PD forward-ref resolves) — the standalone CO pass
//! dedups it.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{DraughtingModel, DraughtingModelForm};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DraughtingModelHandler;

#[step_entity(name = "DRAUGHTING_MODEL")]
impl SimpleEntityHandler for DraughtingModelHandler {
    type WriteInput = DraughtingModel;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "DRAUGHTING_MODEL")?;
        let early = bind::bind_draughting_model(entity_id, attrs)?;
        lower::lower_draughting_model(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dm: DraughtingModel) -> Result<u64, WriteError> {
        let mut items = Vec::with_capacity(dm.items.len());
        for item in dm.items {
            items.push(buf.emit_representation_item_ref(item)?);
        }
        let Attribute::EntityRef(ctx_step) = buf.repr_context_attr(dm.context) else {
            unreachable!("DRAUGHTING_MODEL context guaranteed resolved by lower → EntityRef")
        };
        match dm.form {
            DraughtingModelForm::Simple => {
                let early = lift::lift_draughting_model(dm.name, items, ctx_step);
                Ok(serialize::serialize_draughting_model(buf, &early))
            }
            // Complex MI: emit under the id reserved for the CO facet (so a PD
            // targeting the CHARACTERIZED_OBJECT forward-refs this shared id);
            // fall back to a fresh id when nothing reserved it. The standalone
            // CO pass dedups the inline CO.
            form @ (DraughtingModelForm::Characterized(co_id)
            | DraughtingModelForm::CharacterizedShapeTessellated(co_id)) => {
                let reserved = buf.step_id(co_id);
                let n = if reserved != 0 { reserved } else { buf.fresh() };
                let early = lift::lift_characterized_object_complex(form, dm.name, items, ctx_step);
                serialize::serialize_characterized_object_complex_with_id(buf, n, &early);
                Ok(n)
            }
            DraughtingModelForm::ShapeTessellated => {
                let early = lift::lift_characterized_object_complex(
                    DraughtingModelForm::ShapeTessellated,
                    dm.name,
                    items,
                    ctx_step,
                );
                Ok(serialize::serialize_characterized_object_complex(
                    buf, &early,
                ))
            }
        }
    }
}
