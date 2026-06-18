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

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{DraughtingModel, DraughtingModelForm};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "DRAUGHTING_MODEL")?;
        let early = bind::bind_draughting_model(entity_id, attrs)?;
        lower::lower_draughting_model(ctx, entity_id, early);
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn write(buf: &mut WriteBuffer, dm: DraughtingModel) -> Result<u64, WriteError> {
        let mut item_refs = Vec::with_capacity(dm.items.len());
        for item in dm.items {
            let step = buf.emit_representation_item_ref(item)?;
            item_refs.push(Attribute::EntityRef(step));
        }
        let ctx_attr = buf.repr_context_attr(dm.context);
        match dm.form {
            DraughtingModelForm::Characterized(co_id) => {
                // Complex MI form (phase dm-complex-mi): four-part entity
                // wrapping CHARACTERIZED_OBJECT(*,*) + CHARACTERIZED_REPRESENTATION
                // + DRAUGHTING_MODEL + REPRESENTATION. The CO part is emitted
                // here inline; emit_characterized_objects skips the linked
                // arena entry via dedup so it doesn't surface twice.
                use crate::writer::entity::{WriterBody, WriterEntity};
                // Emit under the id reserved for the CO facet (so a PD targeting
                // the CHARACTERIZED_OBJECT forward-refs this one shared id);
                // fall back to a fresh id when nothing reserved it.
                let reserved = buf.step_id(co_id);
                let n = if reserved != 0 { reserved } else { buf.fresh() };
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
                Ok(n)
            }
            DraughtingModelForm::CharacterizedShapeTessellated(co_id) => {
                // 6-part MI form: the union of the Characterized and
                // ShapeTessellated facets, parts in alphabetical order so
                // re-read re-dispatches to the same case. CO emitted inline
                // (DERIVE); the standalone CO pass dedups it.
                use crate::writer::entity::{WriterBody, WriterEntity};
                let reserved = buf.step_id(co_id);
                let n = if reserved != 0 { reserved } else { buf.fresh() };
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
                            ("SHAPE_REPRESENTATION".into(), vec![]),
                            ("TESSELLATED_SHAPE_REPRESENTATION".into(), vec![]),
                        ],
                    },
                });
                Ok(n)
            }
            DraughtingModelForm::ShapeTessellated => {
                // Complex MI form (phase dm-rep-tsr-complex): four-part entity
                // `(DRAUGHTING_MODEL() REPRESENTATION(...) SHAPE_REPRESENTATION()
                // TESSELLATED_SHAPE_REPRESENTATION())`. Parts in alphabetical
                // order, matching the reader's `required` set so re-read
                // re-dispatches to the same handler.
                use crate::writer::entity::{WriterBody, WriterEntity};
                let n = buf.fresh();
                buf.entities.push(WriterEntity {
                    id: n,
                    body: WriterBody::Complex {
                        parts: vec![
                            ("DRAUGHTING_MODEL".into(), vec![]),
                            (
                                "REPRESENTATION".into(),
                                vec![
                                    Attribute::String(dm.name),
                                    Attribute::List(item_refs),
                                    ctx_attr,
                                ],
                            ),
                            ("SHAPE_REPRESENTATION".into(), vec![]),
                            ("TESSELLATED_SHAPE_REPRESENTATION".into(), vec![]),
                        ],
                    },
                });
                Ok(n)
            }
            DraughtingModelForm::Simple => {
                let Attribute::EntityRef(ctx_step) = ctx_attr else {
                    unreachable!(
                        "DRAUGHTING_MODEL context guaranteed resolved by lower → EntityRef"
                    )
                };
                let items: Vec<u64> = item_refs
                    .into_iter()
                    .map(|a| match a {
                        Attribute::EntityRef(s) => s,
                        _ => unreachable!("item refs are EntityRef"),
                    })
                    .collect();
                let early = lift::lift_draughting_model(dm.name, items, ctx_step);
                Ok(serialize::serialize_draughting_model(buf, &early))
            }
        }
    }
}
