//! `PLANAR_EXTENT` / `PLANAR_BOX` handlers.
//!
//! A `concrete_supertype` pair: both push into the single `planar_extents`
//! arena as variants of [`PlanarExtent`]. `PLANAR_EXTENT(name, size_in_x,
//! size_in_y)` is the base; `PLANAR_BOX` adds a trailing `placement`
//! (`axis2_placement` SELECT — 2D or 3D).

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{PlanarBox, PlanarBoxPlacement, PlanarExtent, PlanarExtentData};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PlanarExtentHandler;

#[step_entity(name = "PLANAR_EXTENT")]
impl SimpleEntityHandler for PlanarExtentHandler {
    type WriteInput = PlanarExtentData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "PLANAR_EXTENT")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let size_in_x = read_real(attrs, 1, entity_id, "size_in_x")?;
        let size_in_y = read_real(attrs, 2, entity_id, "size_in_y")?;
        let id = ctx
            .geometry
            .planar_extents
            .push(PlanarExtent::Itself(PlanarExtentData {
                name,
                size_in_x,
                size_in_y,
            }));
        ctx.planar_extent_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: PlanarExtentData) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PLANAR_EXTENT",
            vec![
                Attribute::String(data.name),
                Attribute::Real(data.size_in_x),
                Attribute::Real(data.size_in_y),
            ],
        ))
    }
}

pub(crate) struct PlanarBoxHandler;

#[step_entity(name = "PLANAR_BOX")]
impl SimpleEntityHandler for PlanarBoxHandler {
    type WriteInput = PlanarBox;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "PLANAR_BOX")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let size_in_x = read_real(attrs, 1, entity_id, "size_in_x")?;
        let size_in_y = read_real(attrs, 2, entity_id, "size_in_y")?;
        let placement_ref = read_entity_ref(attrs, 3, entity_id, "placement")?;

        // `placement` is the `axis2_placement` SELECT — resolve against the
        // 3D then 2D placement maps.
        let placement = if let Some(&id) = ctx.placement_map.get(&placement_ref) {
            PlanarBoxPlacement::Placement3d(id)
        } else if let Some(&id) = ctx.placement_2d_map.get(&placement_ref) {
            PlanarBoxPlacement::Placement2d(id)
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "PLANAR_BOX.placement #{placement_ref} did not resolve to an AXIS2_PLACEMENT"
                ),
            });
            return Ok(());
        };
        let id = ctx
            .geometry
            .planar_extents
            .push(PlanarExtent::PlanarBox(PlanarBox {
                name,
                size_in_x,
                size_in_y,
                placement,
            }));
        ctx.planar_extent_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, pb: PlanarBox) -> Result<u64, WriteError> {
        let placement_step = match pb.placement {
            PlanarBoxPlacement::Placement3d(id) => buf.emit_axis2_placement_3d(id)?,
            PlanarBoxPlacement::Placement2d(id) => buf.emit_axis2_placement_2d(id)?,
        };
        Ok(buf.push_simple(
            "PLANAR_BOX",
            vec![
                Attribute::String(pb.name),
                Attribute::Real(pb.size_in_x),
                Attribute::Real(pb.size_in_y),
                Attribute::EntityRef(placement_step),
            ],
        ))
    }
}
