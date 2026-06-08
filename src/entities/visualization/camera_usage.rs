//! `CAMERA_USAGE` handler — phase cm-usage.
//!
//! `representation_map` SUBTYPE that narrows `mapping_origin` to a
//! `camera_model`. The `mapped_representation` may target any
//! `representation`, including a `DRAUGHTING_MODEL`. Topo order processes
//! that target first, so the DM slot of `repr_id_map` is populated before
//! this handler resolves the ref.
//!
//!
//! Writer side, the carrier is emitted by `emit_camera_usage_arena`
//! (delayed-emit pattern, parallel to `Mdgpr` / `DraughtingModel`) so the
//! `representation_step_ids` cache is fully populated before the
//! `mapped_representation` index is looked up.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{CameraUsage, RepresentationMap};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CameraUsageHandler;

#[step_entity(name = "CAMERA_USAGE")]
impl SimpleEntityHandler for CameraUsageHandler {
    type WriteInput = CameraUsage;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "CAMERA_USAGE")?;
        let origin_ref = read_entity_ref(attrs, 0, entity_id, "mapping_origin")?;
        let mapped_ref = read_entity_ref(attrs, 1, entity_id, "mapped_representation")?;

        let Some(&mapping_origin) = ctx.viz_camera_model_id_map.get(&origin_ref) else {
            return Ok(());
        };
        let Some(&mapped_representation) = ctx.repr_id_map.get(&mapped_ref) else {
            return Ok(());
        };

        let id = ctx
            .representation_maps
            .push(RepresentationMap::CameraUsage(CameraUsage {
                mapping_origin,
                mapped_representation,
            }));
        ctx.representation_map_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, cu: CameraUsage) -> Result<u64, WriteError> {
        let origin = buf.step_id(cu.mapping_origin);
        let mapped = buf.representation_step_ids[cu.mapped_representation.0 as usize];
        Ok(buf.push_simple(
            "CAMERA_USAGE",
            vec![Attribute::EntityRef(origin), Attribute::EntityRef(mapped)],
        ))
    }
}
