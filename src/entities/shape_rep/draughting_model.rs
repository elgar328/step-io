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
use crate::ir::shape_rep::{DraughtingModel, DraughtingModelForm, Representation};
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
        let had_item_refs = !item_refs.is_empty();
        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(item) = resolve_representation_item_ref(ctx, r) {
                items.push(item);
            }
        }
        if items.is_empty() && had_item_refs {
            // The model listed items but none resolved to a modelled
            // representation_item — the whole DRAUGHTING_MODEL is dropped.
            // Surface it rather than dropping silently.
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: String::from(
                    "DRAUGHTING_MODEL dropped: no items resolved to a modelled representation_item",
                ),
            });
            return Ok(());
        }
        // A genuinely empty `items` SET (source had `()`) is schema-legal and
        // preserved as an empty draughting model.
        let repr_id = ctx
            .representations
            .push(Representation::DraughtingModel(DraughtingModel {
                name,
                items,
                context,
                form: DraughtingModelForm::Simple,
            }));
        ctx.repr_id_map.insert(entity_id, repr_id);
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
            DraughtingModelForm::Simple => Ok(buf.push_simple(
                "DRAUGHTING_MODEL",
                vec![
                    Attribute::String(dm.name),
                    Attribute::List(item_refs),
                    ctx_attr,
                ],
            )),
        }
    }
}
