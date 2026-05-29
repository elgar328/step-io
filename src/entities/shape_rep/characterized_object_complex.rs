//! `(CHARACTERIZED_OBJECT CHARACTERIZED_REPRESENTATION ...)` complex MI
//! handler — phase characterized-min.
//!
//! Detects the corpus 100%-complex-MI form where `CHARACTERIZED_OBJECT`
//! and `CHARACTERIZED_REPRESENTATION` parts always co-occur (with
//! `DRAUGHTING_MODEL` / `REPRESENTATION` / `SHAPE_REPRESENTATION` /
//! `TESSELLATED_SHAPE_REPRESENTATION` companions). The `CHARACTERIZED_OBJECT`
//! part itself carries `(*, *)` (both attrs DERIVE); the `name` value
//! lives in the `REPRESENTATION` part — extracted here. Other parts'
//! data is discarded (minimal scope).

use crate::entities::ComplexEntityHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::attr::{read_entity_ref_list, read_optional_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{
    CharacterizedObject, CharacterizedObjectData, DraughtingModel, Representation,
};
use crate::parser::entity::{EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, has_all_parts, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity_complex;

pub(crate) struct CharacterizedObjectComplexHandler;

#[step_entity_complex(
    name = "CHARACTERIZED_OBJECT",
    pass = Pass8CharacterizedComplex,
    required = ["CHARACTERIZED_OBJECT", "CHARACTERIZED_REPRESENTATION"]
)]
impl ComplexEntityHandler for CharacterizedObjectComplexHandler {
    type WriteInput = CharacterizedObjectData;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // REPRESENTATION part is always co-instantiated per ir.toml
        // (verified: 48/48 inst). Extract `name` from its first attr.
        let repr_attrs = require_part_attrs(parts, "REPRESENTATION", entity_id)?;
        let name = read_string_or_unset(repr_attrs, 0, entity_id, "name")?.to_owned();
        let co_id =
            ctx.characterized_objects
                .push(CharacterizedObject::Itself(CharacterizedObjectData {
                    name: name.clone(),
                    description: None,
                }));
        // NIST AP242 MBD pattern: same complex MI entity also carries
        // DRAUGHTING_MODEL + REPRESENTATION parts. Surface the representation
        // side via repr_id_map so downstream MDDR / DMIA resolve.
        if has_all_parts(parts, &["DRAUGHTING_MODEL", "REPRESENTATION"]) {
            let item_refs = read_entity_ref_list(repr_attrs, 1, entity_id, "items")?;
            let ctx_ref_opt =
                read_optional_entity_ref(repr_attrs, 2, entity_id, "context_of_items")?;
            let context = ctx_ref_opt.and_then(|r| ctx.resolve_repr_context(r));
            let mut items = Vec::with_capacity(item_refs.len());
            for r in item_refs {
                if let Some(item) = resolve_representation_item_ref(ctx, r) {
                    items.push(item);
                }
            }
            if !items.is_empty() {
                let repr_id =
                    ctx.representations
                        .push(Representation::DraughtingModel(DraughtingModel {
                            name,
                            items,
                            context,
                            characterized_object_id: Some(co_id),
                        }));
                ctx.repr_id_map.insert(entity_id, repr_id);
            }
        }
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: CharacterizedObjectData) -> Result<u64, WriteError> {
        use crate::parser::entity::Attribute;
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
