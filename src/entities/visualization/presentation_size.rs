//! `AREA_IN_SET` + `PRESENTATION_SIZE` handlers — phase pr-size.
//!
//! `area_in_set` binds a `PresentationArea` representation to a
//! `PresentationSet`; `presentation_size` carries a 2D extent box paired
//! with a `presentation_size_assignment_select` (view / area /
//! `area_in_set`). Unresolved refs drop the carrier on read.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    AreaInSet, PresentationSize, PresentationSizeAssignment, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AreaInSetHandler;

#[step_entity(name = "AREA_IN_SET")]
impl SimpleEntityHandler for AreaInSetHandler {
    type WriteInput = AreaInSet;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "AREA_IN_SET")?;
        let area_ref = read_entity_ref(attrs, 0, entity_id, "area")?;
        let Some(&area) = ctx.presentation_representation_id_map.get(&area_ref) else {
            return Ok(());
        };
        let in_ref = read_entity_ref(attrs, 1, entity_id, "in_set")?;
        let Some(&in_set) = ctx.presentation_set_id_map.get(&in_ref) else {
            return Ok(());
        };
        let id = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default)
            .area_in_sets
            .push(AreaInSet { area, in_set });
        ctx.area_in_set_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ais: AreaInSet) -> Result<u64, WriteError> {
        let area_step = buf.step_id(ais.area);
        let set_step = buf.step_id(ais.in_set);
        Ok(buf.push_simple(
            "AREA_IN_SET",
            vec![
                Attribute::EntityRef(area_step),
                Attribute::EntityRef(set_step),
            ],
        ))
    }
}

pub(crate) struct PresentationSizeHandler;

#[step_entity(name = "PRESENTATION_SIZE")]
impl SimpleEntityHandler for PresentationSizeHandler {
    type WriteInput = PresentationSize;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PRESENTATION_SIZE")?;
        let unit_ref = read_entity_ref(attrs, 0, entity_id, "unit")?;
        let unit = if let Some(&id) = ctx.area_in_set_id_map.get(&unit_ref) {
            PresentationSizeAssignment::AreaInSet(id)
        } else if let Some(&id) = ctx.presentation_representation_id_map.get(&unit_ref) {
            // Spec narrows to View / Area variants — read1 cannot tell
            // which without inspecting the arena entry. Default to View;
            // emit reconstructs via the cached step id either way.
            PresentationSizeAssignment::View(id)
        } else {
            return Ok(());
        };
        let size_ref = read_entity_ref(attrs, 1, entity_id, "size")?;
        let Some(&size) = ctx.planar_extent_id_map.get(&size_ref) else {
            return Ok(());
        };
        let _id = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default)
            .presentation_sizes
            .push(PresentationSize { unit, size });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ps: PresentationSize) -> Result<u64, WriteError> {
        let unit_step = match ps.unit {
            PresentationSizeAssignment::View(id) | PresentationSizeAssignment::Area(id) => {
                buf.step_id(id)
            }
            PresentationSizeAssignment::AreaInSet(id) => buf.step_id(id),
        };
        let size_step = buf.emit_planar_extent(ps.size)?;
        Ok(buf.push_simple(
            "PRESENTATION_SIZE",
            vec![
                Attribute::EntityRef(unit_step),
                Attribute::EntityRef(size_step),
            ],
        ))
    }
}
