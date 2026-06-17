//! `GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION` handler — 2-layer path.
//! Sister of `gbwsr`. Shares the writer helper (`wireframe_write_items`) and
//! registers the `_SURFACE_` wrapper name so the writer round-trips the spelling
//! the source file used. `read` = generated `bind` + the hand
//! `lower_geometrically_bounded_surface_shape_representation`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

use super::gbwsr::{WireframeRepresentationWriteInput, wireframe_write_items};

pub(crate) struct GbssrHandler;

#[step_entity(name = "GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION")]
impl SimpleEntityHandler for GbssrHandler {
    type WriteInput = WireframeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            3,
            entity_id,
            "GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION",
        )?;
        let early =
            bind::bind_geometrically_bounded_surface_shape_representation(entity_id, attrs)?;
        lower::lower_geometrically_bounded_surface_shape_representation(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: WireframeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        let (items, unit_ctx) = wireframe_write_items(buf, input)?;
        let early = lift::lift_geometrically_bounded_surface_shape_representation(
            String::new(),
            items,
            unit_ctx,
        );
        Ok(serialize::serialize_geometrically_bounded_surface_shape_representation(buf, &early))
    }
}
