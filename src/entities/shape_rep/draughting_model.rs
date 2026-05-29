//! `DRAUGHTING_MODEL` handler — phase draughting-model.
//!
//! `representation` subtype carrying a `set_select` of
//! `draughting_model_item_select` members. Resolved through the generic
//! [`RepresentationItemRef`] helper; unresolved items are skipped on read
//! (symmetric on re-read). Complex-MI multi-inheritance forms
//! (`(CHARACTERIZED_OBJECT ... DRAUGHTING_MODEL ...)`) are not modelled
//! in this phase and silently dropped.
//!
//! Emit is delayed (Mdgpr pattern) — `emit_representations_pre_pass`
//! skips `DraughtingModel`; the actual write happens in
//! `emit_draughting_models` after every item-ref cache has been
//! populated.

use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{
    check_count, read_entity_ref_list, read_optional_entity_ref, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{DraughtingModel, Representation};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DraughtingModelHandler;

#[step_entity(name = "DRAUGHTING_MODEL", pass = Pass8DraughtingModel)]
impl SimpleEntityHandler for DraughtingModelHandler {
    type WriteInput = DraughtingModel;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "DRAUGHTING_MODEL")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        // `context_of_items` is optional on re-read: the writer emits `$`
        // when the source context was a non-modelled complex (e.g. a 2D
        // `GEOMETRIC_REPRESENTATION_CONTEXT` + `PARAMETRIC_REPRESENTATION_CONTEXT`
        // wrapper without `global_unit_assigned_context`). Accept both
        // forms so round-trip is symmetric.
        let ctx_ref_opt = read_optional_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        let context = ctx_ref_opt.and_then(|r| ctx.resolve_repr_context(r));
        if let Some(crate::ir::shape_rep::RepresentationContextRef::Unitful(ctx_id)) = context {
            ctx.repr_context_map.insert(entity_id, ctx_id);
        }
        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(item) = resolve_representation_item_ref(ctx, r) {
                items.push(item);
            }
        }
        if items.is_empty() {
            return Ok(());
        }
        let repr_id = ctx
            .representations
            .push(Representation::DraughtingModel(DraughtingModel {
                name,
                items,
                context,
                characterized_object_id: None,
            }));
        ctx.repr_id_map.insert(entity_id, repr_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dm: DraughtingModel) -> Result<u64, WriteError> {
        let mut item_refs = Vec::with_capacity(dm.items.len());
        for item in dm.items {
            let step = buf.emit_representation_item_ref(item)?;
            item_refs.push(Attribute::EntityRef(step));
        }
        let ctx_attr = buf.repr_context_attr(dm.context);
        if dm.characterized_object_id.is_some() {
            // Complex MI form (phase dm-complex-mi): four-part entity
            // wrapping CHARACTERIZED_OBJECT(*,*) + CHARACTERIZED_REPRESENTATION
            // + DRAUGHTING_MODEL + REPRESENTATION. The CO part is emitted
            // here inline; emit_characterized_objects skips the linked
            // arena entry via dedup so it doesn't surface twice.
            use crate::writer::entity::{WriterBody, WriterEntity};
            let n = buf.fresh();
            buf.entities.push(WriterEntity {
                id: n,
                body: WriterBody::Complex {
                    parts: vec![
                        (
                            "CHARACTERIZED_OBJECT".into(),
                            vec![Attribute::Derived, Attribute::Derived],
                        ),
                        ("CHARACTERIZED_REPRESENTATION".into(), vec![]),
                        ("DRAUGHTING_MODEL".into(), vec![]),
                        (
                            "REPRESENTATION".into(),
                            vec![
                                Attribute::String(dm.name),
                                Attribute::List(item_refs),
                                ctx_attr,
                            ],
                        ),
                    ],
                },
            });
            return Ok(n);
        }
        Ok(buf.push_simple(
            "DRAUGHTING_MODEL",
            vec![
                Attribute::String(dm.name),
                Attribute::List(item_refs),
                ctx_attr,
            ],
        ))
    }
}
