//! `ITEM_DEFINED_TRANSFORMATION` handler — Pass 6-6.
//!
//! Reader resolves source / target placements through
//! `ReaderContext::resolve_placement` and stores the resulting
//! `Transform3d` keyed by entity id in `transform_map`. Writer emits an
//! IDT line with the per-instance source / target axis placements.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::Transform3d;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ItemDefinedTransformationHandler;

#[step_entity(name = "ITEM_DEFINED_TRANSFORMATION")]
impl SimpleEntityHandler for ItemDefinedTransformationHandler {
    type WriteInput = Transform3d;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "ITEM_DEFINED_TRANSFORMATION")?;
        // attrs[0] = name, attrs[1] = description — ignored.
        let source_ref = read_entity_ref(attrs, 2, entity_id, "transform_item_1")?;
        let target_ref = read_entity_ref(attrs, 3, entity_id, "transform_item_2")?;
        let source = ctx.resolve_placement(entity_id, source_ref, "transform_item_1")?;
        let target = ctx.resolve_placement(entity_id, target_ref, "transform_item_2")?;
        ctx.transform_map
            .insert(entity_id, Transform3d { source, target });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, transform: Transform3d) -> Result<u64, WriteError> {
        let source = buf.emit_axis2_placement_3d(transform.source)?;
        let target = buf.emit_axis2_placement_3d(transform.target)?;
        Ok(buf.push_simple(
            "ITEM_DEFINED_TRANSFORMATION",
            vec![
                Attribute::String(String::new()),
                Attribute::String(String::new()),
                Attribute::EntityRef(source),
                Attribute::EntityRef(target),
            ],
        ))
    }
}
