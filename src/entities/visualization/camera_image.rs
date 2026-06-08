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

use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{CameraImage, MappedItem};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::{step_entity, step_entity_complex};

pub(crate) struct CameraImageHandler;

#[step_entity(name = "CAMERA_IMAGE")]
impl SimpleEntityHandler for CameraImageHandler {
    type WriteInput = CameraImage;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "CAMERA_IMAGE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let source_ref = read_entity_ref(attrs, 1, entity_id, "mapping_source")?;
        let target_ref = read_entity_ref(attrs, 2, entity_id, "mapping_target")?;
        let Some(body) = resolve_camera_image_body(ctx, name, source_ref, target_ref) else {
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

/// Styled `CAMERA_IMAGE_3D_WITH_SCALE` — read only as the AND-combined complex
/// `(CAMERA_IMAGE CAMERA_IMAGE_3D_WITH_SCALE GEOMETRIC_REPRESENTATION_ITEM
/// MAPPED_ITEM REPRESENTATION_ITEM)`, the only form in the corpus (the simple
/// single-name entity has count 0). `name` is on the `REPRESENTATION_ITEM` part,
/// `mapping_source` (a `camera_usage`) + `mapping_target` (a `planar_box`) on the
/// `MAPPED_ITEM` part; unresolved members drop the carrier, symmetric on re-read.
/// Stored in the `mapped_items` arena + `mapped_item_id_map` so a `PRESENTATION_VIEW`
/// / `STYLED_ITEM` consumer resolves it through `RepresentationItemRef::MappedItem`.
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let ri = require_part_attrs(parts, "REPRESENTATION_ITEM", entity_id)?;
        let name = read_string_or_unset(ri, 0, entity_id, "name")?.to_owned();
        let mi = require_part_attrs(parts, "MAPPED_ITEM", entity_id)?;
        let source_ref = read_entity_ref(mi, 0, entity_id, "mapping_source")?;
        let target_ref = read_entity_ref(mi, 1, entity_id, "mapping_target")?;
        let Some(body) = resolve_camera_image_body(ctx, name, source_ref, target_ref) else {
            return Ok(());
        };
        let mi_id = ctx
            .mapped_items
            .push(MappedItem::CameraImage3dWithScale(body));
        ctx.mapped_item_id_map.insert(entity_id, mi_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ci: CameraImage) -> Result<u64, WriteError> {
        // Re-emit the AND-combined complex form (alphabetical parts); data only on
        // MAPPED_ITEM (mapping_source, mapping_target) + REPRESENTATION_ITEM (name).
        let source = buf.representation_map_step_ids[ci.mapping_source.0 as usize];
        let target = buf.emit_planar_extent(ci.mapping_target)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex {
                parts: vec![
                    ("CAMERA_IMAGE".into(), vec![]),
                    ("CAMERA_IMAGE_3D_WITH_SCALE".into(), vec![]),
                    ("GEOMETRIC_REPRESENTATION_ITEM".into(), vec![]),
                    (
                        "MAPPED_ITEM".into(),
                        vec![Attribute::EntityRef(source), Attribute::EntityRef(target)],
                    ),
                    (
                        "REPRESENTATION_ITEM".into(),
                        vec![Attribute::String(ci.name)],
                    ),
                ],
            },
        });
        Ok(n)
    }
}

/// Resolve a `camera_image` body from an already-read `name` + `mapping_source`
/// / `mapping_target` step refs. `None` when either ref does not resolve (the
/// carrier is dropped, symmetric on re-read). Shared by the plain `CAMERA_IMAGE`
/// reader and the `CAMERA_IMAGE_3D_WITH_SCALE` complex reader.
fn resolve_camera_image_body(
    ctx: &ReaderContext,
    name: String,
    source_ref: u64,
    target_ref: u64,
) -> Option<CameraImage> {
    let &mapping_source = ctx.representation_map_id_map.get(&source_ref)?;
    let &mapping_target = ctx.planar_extent_id_map.get(&target_ref)?;
    Some(CameraImage {
        name,
        mapping_source,
        mapping_target,
    })
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
