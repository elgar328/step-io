//! `STEP` form-feature handler.
//!
//! AP242 `step SUBTYPE OF feature_definition SUBTYPE OF characterized_object`
//! manufacturing-step entity. Inherits `(name, description)` from
//! `characterized_object`. Not to be confused with the P21 file format —
//! this is the entity name `STEP` appearing in DATA section lines.

use crate::entities::SimpleEntityHandler;
use crate::entities::assembly_product::product_definition_relationship::{
    description_attr, read_optional_description,
};
use crate::ir::FormFeaturesPool;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::form_features::Step;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct StepHandler;

#[step_entity(name = "STEP")]
impl SimpleEntityHandler for StepHandler {
    type WriteInput = Step;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "STEP")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_optional_description(attrs, 1, entity_id)?;
        let pool = ctx
            .form_features
            .get_or_insert_with(FormFeaturesPool::default);
        let id = pool.feature_definitions.push(Step { name, description });
        ctx.feature_definition_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, s: Step) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "STEP",
            vec![Attribute::String(s.name), description_attr(s.description)],
        ))
    }
}
