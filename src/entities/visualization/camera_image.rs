//! `CAMERA_IMAGE` + `CAMERA_IMAGE_3D_WITH_SCALE` handlers (2-layer path).
//!
//! `CAMERA_IMAGE` is `SUBTYPE OF (mapped_item)` with `mapping_source` narrowed
//! to a `camera_usage` and `mapping_target` to a `planar_box`.
//! `CAMERA_IMAGE_3D_WITH_SCALE` is the AND-combined complex form (the only form
//! in the corpus; the `scale` DERIVE attr is not stored or emitted). Both share
//! the [`CameraImage`] payload in the `mapped_items` arena; the `MappedItem`
//! enum picks the STEP type at write time.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::CameraImage;
use crate::parser::entity::{Attribute, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::{step_entity, step_entity_complex};

pub(crate) struct CameraImageHandler;

#[step_entity(name = "CAMERA_IMAGE")]
impl SimpleEntityHandler for CameraImageHandler {
    type WriteInput = CameraImage;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_camera_image(entity_id, attrs)?;
        lower::lower_camera_image(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ci: CameraImage) -> Result<u64, WriteError> {
        let source = buf.step_id(ci.mapping_source);
        let target = buf.emit_planar_extent(ci.mapping_target)?;
        let early = lift::lift_camera_image(ci.name, source, target);
        Ok(serialize::serialize_camera_image(buf, &early))
    }
}

pub(crate) struct CameraImage3dWithScaleHandler;

/// `CAMERA_IMAGE_3D_WITH_SCALE` — read only as the AND-combined complex (the only
/// corpus form; the single-name entity has count 0). Stored in `mapped_items` so
/// a `PRESENTATION_VIEW` / `STYLED_ITEM` consumer resolves it through
/// `RepresentationItemRef::MappedItem`.
#[step_entity_complex(
    name = "CAMERA_IMAGE_3D_WITH_SCALE",
    cases = [[
        "CAMERA_IMAGE",
        "CAMERA_IMAGE_3D_WITH_SCALE",
        "GEOMETRIC_REPRESENTATION_ITEM",
        "MAPPED_ITEM",
        "REPRESENTATION_ITEM",
    ]]
)]
impl ComplexEntityHandler for CameraImage3dWithScaleHandler {
    type WriteInput = CameraImage;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_camera_image_3d_with_scale(entity_id, parts)?;
        lower::lower_camera_image_3d_with_scale(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ci: CameraImage) -> Result<u64, WriteError> {
        let source = buf.step_id(ci.mapping_source);
        let target = buf.emit_planar_extent(ci.mapping_target)?;
        let early = lift::lift_camera_image_3d_with_scale(ci.name, source, target);
        Ok(serialize::serialize_camera_image_3d_with_scale(buf, &early))
    }
}
