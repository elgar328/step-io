//! `CAMERA_IMAGE` + `CAMERA_IMAGE_3D_WITH_SCALE` handlers — phase cm-image.
//!
//! `CAMERA_IMAGE` is `SUBTYPE OF (mapped_item)` with `mapping_source`
//! narrowed to a `camera_usage` and `mapping_target` narrowed to a
//! `planar_box`. `CAMERA_IMAGE_3D_WITH_SCALE` extends `camera_image` with
//! a DERIVE `scale` attribute (encoded as `*` in P21), so its instance
//! carries 4 attrs in the source file but no extra IR fields.
//!
//! Both variants share the [`CameraImage`] payload and live in the
//! `mapped_items` arena alongside the `Itself` carrier; dispatch on the
//! `MappedItem` enum picks the correct STEP type at write time.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{CameraImage, MappedItem};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CameraImageHandler;

#[step_entity(name = "CAMERA_IMAGE", pass = Pass8CameraImage)]
impl SimpleEntityHandler for CameraImageHandler {
    type WriteInput = CameraImage;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "CAMERA_IMAGE")?;
        let Some(body) = read_camera_image_body(ctx, entity_id, attrs, "CAMERA_IMAGE")? else {
            return Ok(());
        };
        let mi_id = ctx.mapped_items.push(MappedItem::CameraImage(body));
        ctx.mapped_item_id_map.insert(entity_id, mi_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ci: CameraImage) -> Result<u64, WriteError> {
        emit_camera_image(buf, "CAMERA_IMAGE", ci, false)
    }
}

pub(crate) struct CameraImage3dWithScaleHandler;

#[step_entity(name = "CAMERA_IMAGE_3D_WITH_SCALE", pass = Pass8CameraImage)]
impl SimpleEntityHandler for CameraImage3dWithScaleHandler {
    type WriteInput = CameraImage;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // 4 attrs in P21: the 3 inherited explicit attrs plus the DERIVE
        // `scale` encoded as `*`. We ignore attr[3] and reuse the base
        // body reader.
        check_count(attrs, 4, entity_id, "CAMERA_IMAGE_3D_WITH_SCALE")?;
        let Some(body) =
            read_camera_image_body(ctx, entity_id, attrs, "CAMERA_IMAGE_3D_WITH_SCALE")?
        else {
            return Ok(());
        };
        let mi_id = ctx
            .mapped_items
            .push(MappedItem::CameraImage3dWithScale(body));
        ctx.mapped_item_id_map.insert(entity_id, mi_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ci: CameraImage) -> Result<u64, WriteError> {
        emit_camera_image(buf, "CAMERA_IMAGE_3D_WITH_SCALE", ci, true)
    }
}

fn read_camera_image_body(
    ctx: &ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    name: &'static str,
) -> Result<Option<CameraImage>, ConvertError> {
    let _ = name;
    let ci_name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let source_ref = read_entity_ref(attrs, 1, entity_id, "mapping_source")?;
    let target_ref = read_entity_ref(attrs, 2, entity_id, "mapping_target")?;
    let Some(&mapping_source) = ctx.representation_map_id_map.get(&source_ref) else {
        return Ok(None);
    };
    let Some(&mapping_target) = ctx.planar_extent_id_map.get(&target_ref) else {
        return Ok(None);
    };
    Ok(Some(CameraImage {
        name: ci_name,
        mapping_source,
        mapping_target,
    }))
}

fn emit_camera_image(
    buf: &mut WriteBuffer,
    type_name: &'static str,
    ci: CameraImage,
    derive_scale: bool,
) -> Result<u64, WriteError> {
    let source = buf.representation_map_step_ids[ci.mapping_source.0 as usize];
    let target = buf.emit_planar_extent(ci.mapping_target)?;
    let mut attrs = vec![
        Attribute::String(ci.name),
        Attribute::EntityRef(source),
        Attribute::EntityRef(target),
    ];
    if derive_scale {
        // `scale` is DERIVE in EXPRESS — encoded as `*` in P21.
        attrs.push(Attribute::Derived);
    }
    Ok(buf.push_simple(type_name, attrs))
}
