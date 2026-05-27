//! `CAMERA_MODEL_D3_WITH_HLHSR` + `CAMERA_MODEL_D3_MULTI_CLIPPING`
//! handlers — phase cm-variants.
//!
//! Both subtype `camera_model_d3` and reuse its first 3 attrs
//! (`name`, `view_reference_system`, `perspective_of_volume`); each adds
//! one extra attr (boolean `hidden_line_surface_removal` for hlhsr; a
//! SET of `shape_clipping` for `multi_clipping`).

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref, read_entity_ref_list, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    CameraModel, CameraModelD3, CameraModelD3MultiClipping, CameraModelD3WithHlhsr, ShapeClipping,
    VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CameraModelD3WithHlhsrHandler;

#[step_entity(name = "CAMERA_MODEL_D3_WITH_HLHSR", pass = Pass8CameraModel)]
impl SimpleEntityHandler for CameraModelD3WithHlhsrHandler {
    type WriteInput = CameraModelD3WithHlhsr;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "CAMERA_MODEL_D3_WITH_HLHSR")?;
        let Some(inherited) = read_cmd3_body(ctx, entity_id, attrs, "CAMERA_MODEL_D3_WITH_HLHSR")?
        else {
            return Ok(());
        };
        let hidden_line_surface_removal =
            read_bool(attrs, 3, entity_id, "hidden_line_surface_removal")?;
        let id = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default)
            .camera_models
            .push(CameraModel::CameraModelD3WithHlhsr(
                CameraModelD3WithHlhsr {
                    inherited,
                    hidden_line_surface_removal,
                },
            ));
        ctx.viz_camera_model_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: CameraModelD3WithHlhsr) -> Result<u64, WriteError> {
        let vrs = buf.emit_axis2_placement_3d(c.inherited.view_reference_system)?;
        let pov = buf.founded_item_step_ids[c.inherited.perspective_of_volume.0 as usize];
        Ok(buf.push_simple(
            "CAMERA_MODEL_D3_WITH_HLHSR",
            vec![
                Attribute::String(c.inherited.name),
                Attribute::EntityRef(vrs),
                Attribute::EntityRef(pov),
                Attribute::Enum(if c.hidden_line_surface_removal {
                    "T".into()
                } else {
                    "F".into()
                }),
            ],
        ))
    }
}

pub(crate) struct CameraModelD3MultiClippingHandler;

#[step_entity(name = "CAMERA_MODEL_D3_MULTI_CLIPPING", pass = Pass8CameraModel)]
impl SimpleEntityHandler for CameraModelD3MultiClippingHandler {
    type WriteInput = CameraModelD3MultiClipping;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "CAMERA_MODEL_D3_MULTI_CLIPPING")?;
        let Some(inherited) =
            read_cmd3_body(ctx, entity_id, attrs, "CAMERA_MODEL_D3_MULTI_CLIPPING")?
        else {
            return Ok(());
        };
        let refs = read_entity_ref_list(attrs, 3, entity_id, "shape_clipping")?;
        let mut shape_clipping = Vec::with_capacity(refs.len());
        for r in refs {
            if let Some(&sid) = ctx.surface_map.get(&r) {
                shape_clipping.push(ShapeClipping::Plane(sid));
            }
            // camera_model_d3_multi_clipping_union members: not modelled,
            // silently skipped.
        }
        if shape_clipping.is_empty() {
            return Ok(());
        }
        let id = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default)
            .camera_models
            .push(CameraModel::CameraModelD3MultiClipping(
                CameraModelD3MultiClipping {
                    inherited,
                    shape_clipping,
                },
            ));
        ctx.viz_camera_model_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: CameraModelD3MultiClipping) -> Result<u64, WriteError> {
        let vrs = buf.emit_axis2_placement_3d(c.inherited.view_reference_system)?;
        let pov = buf.founded_item_step_ids[c.inherited.perspective_of_volume.0 as usize];
        let mut clipping = Vec::with_capacity(c.shape_clipping.len());
        for sc in c.shape_clipping {
            match sc {
                ShapeClipping::Plane(id) => {
                    let step = buf.emit_surface(id)?;
                    clipping.push(Attribute::EntityRef(step));
                }
            }
        }
        Ok(buf.push_simple(
            "CAMERA_MODEL_D3_MULTI_CLIPPING",
            vec![
                Attribute::String(c.inherited.name),
                Attribute::EntityRef(vrs),
                Attribute::EntityRef(pov),
                Attribute::List(clipping),
            ],
        ))
    }
}

fn read_cmd3_body(
    ctx: &ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    name: &'static str,
) -> Result<Option<CameraModelD3>, ConvertError> {
    let cm_name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let vrs_ref = read_entity_ref(attrs, 1, entity_id, "view_reference_system")?;
    let pov_ref = read_entity_ref(attrs, 2, entity_id, "perspective_of_volume")?;
    let Some(&view_reference_system) = ctx.placement_map.get(&vrs_ref) else {
        return Ok(None);
    };
    let Some(&perspective_of_volume) = ctx.viz_view_volume_id_map.get(&pov_ref) else {
        return Ok(None);
    };
    let _ = name;
    Ok(Some(CameraModelD3 {
        name: cm_name,
        view_reference_system,
        perspective_of_volume,
    }))
}
