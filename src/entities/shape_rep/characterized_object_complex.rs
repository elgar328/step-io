//! Single owner of the `DRAUGHTING_MODEL`-family complex MI — phase
//! exact-case-merge. Three exact part-sets co-occur in the corpus:
//!   A `(CHARACTERIZED_OBJECT CHARACTERIZED_REPRESENTATION DRAUGHTING_MODEL
//!      REPRESENTATION)`                                     → `Characterized`
//!   C `(DRAUGHTING_MODEL REPRESENTATION SHAPE_REPRESENTATION
//!      TESSELLATED_SHAPE_REPRESENTATION)`                   → `ShapeTessellated`
//!   B their union (6 parts)              → `CharacterizedShapeTessellated`
//! All three carry data only on the `REPRESENTATION` part (name / items /
//! context); the `CHARACTERIZED_OBJECT` part is `(*, *)` (both DERIVE). One
//! handler claims all three so no two handlers fight over the 6-part form.

use crate::entities::ComplexEntityHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{read_entity_ref_list, read_optional_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{
    CharacterizedObject, CharacterizedObjectData, DraughtingModel, DraughtingModelForm,
    Representation, RepresentationContextRef,
};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, has_all_parts, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct CharacterizedObjectComplexHandler;

#[step_entity_complex(
    name = "CHARACTERIZED_OBJECT",
    cases = [
        ["CHARACTERIZED_OBJECT", "CHARACTERIZED_REPRESENTATION", "DRAUGHTING_MODEL", "REPRESENTATION"],
        ["CHARACTERIZED_OBJECT", "CHARACTERIZED_REPRESENTATION", "DRAUGHTING_MODEL", "REPRESENTATION", "SHAPE_REPRESENTATION", "TESSELLATED_SHAPE_REPRESENTATION"],
        ["DRAUGHTING_MODEL", "REPRESENTATION", "SHAPE_REPRESENTATION", "TESSELLATED_SHAPE_REPRESENTATION"],
    ]
)]
impl ComplexEntityHandler for CharacterizedObjectComplexHandler {
    type WriteInput = CharacterizedObjectData;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // `name` / `items` / `context_of_items` live on the REPRESENTATION part.
        let repr_attrs = require_part_attrs(parts, "REPRESENTATION", entity_id)?;
        let name = read_string_or_unset(repr_attrs, 0, entity_id, "name")?.to_owned();

        let has_co = has_all_parts(
            parts,
            &["CHARACTERIZED_OBJECT", "CHARACTERIZED_REPRESENTATION"],
        );
        let has_st = has_all_parts(
            parts,
            &["SHAPE_REPRESENTATION", "TESSELLATED_SHAPE_REPRESENTATION"],
        );

        // The CHARACTERIZED_OBJECT facet (cases A / B) gets a carrier arena entry
        // so a PROPERTY_DEFINITION / CIWR targeting it resolves; cases C have none.
        let co_id = has_co.then(|| {
            ctx.characterized_objects
                .push(CharacterizedObject::Itself(CharacterizedObjectData {
                    name: name.clone(),
                    description: None,
                }))
        });

        let item_refs = read_entity_ref_list(repr_attrs, 1, entity_id, "items")?;
        let ctx_ref_opt = read_optional_entity_ref(repr_attrs, 2, entity_id, "context_of_items")?;
        let context = ctx_ref_opt.and_then(|r| ctx.resolve_repr_context(r));
        // Surface a Unitful context so an SDR/PDR referencing this draughting
        // model resolves its unit context (mirrors the simple DM reader).
        if let Some(RepresentationContextRef::Unitful(ctx_id)) = context {
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

        let form = match (co_id, has_st) {
            (Some(id), true) => DraughtingModelForm::CharacterizedShapeTessellated(id),
            (Some(id), false) => DraughtingModelForm::Characterized(id),
            (None, true) => DraughtingModelForm::ShapeTessellated,
            (None, false) => DraughtingModelForm::Simple,
        };
        let repr_id = ctx
            .representations
            .push(Representation::DraughtingModel(DraughtingModel {
                name,
                items,
                context,
                form,
            }));
        ctx.id_cache.insert(entity_id, repr_id);
        // Register the CHARACTERIZED_OBJECT facet so a PROPERTY_DEFINITION
        // targeting this MBD model (as a characterized_object) resolves it.
        if let Some(cid) = co_id {
            ctx.id_cache.insert(entity_id, cid);
        }
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: CharacterizedObjectData) -> Result<u64, WriteError> {
        let desc = match data.description {
            Some(d) => Attribute::String(d),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "CHARACTERIZED_OBJECT",
            vec![Attribute::String(data.name), desc],
        ))
    }
}
