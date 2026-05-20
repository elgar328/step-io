//! `MANIFOLD_SURFACE_SHAPE_REPRESENTATION` handler — Pass 6-4a.
//!
//! Reader resolves each item that points at an SBSM and flattens the
//! collected shells into `mssr_shells_map`; the first `AXIS2_PLACEMENT_3D`
//! provides the coordinate reference frame. Writer mirrors the structure:
//! emit a per-product axis placement, then an SBSM wrapping all shells,
//! and reference both from the MSSR line.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::Product;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string};
use crate::ir::error::ConvertError;
use crate::ir::id::ShellId;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use crate::entities::geometry::shell_based_surface_model::ShellBasedSurfaceModelHandler;
use step_io_macros::step_entity;

pub(crate) struct ManifoldSurfaceShapeRepresentationWriteInput {
    pub(crate) product: Product,
    pub(crate) shells: Vec<ShellId>,
    pub(crate) unit_ctx: u64,
}

pub(crate) struct ManifoldSurfaceShapeRepresentationHandler;

#[step_entity(name = "MANIFOLD_SURFACE_SHAPE_REPRESENTATION", pass = Pass6ShapeRep)]
impl SimpleEntityHandler for ManifoldSurfaceShapeRepresentationHandler {
    type WriteInput = ManifoldSurfaceShapeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")?;
        let name = read_string(attrs, 0, entity_id, "name")?.to_owned();
        let items = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        let context = ctx.context_id_map.get(&ctx_ref).copied();
        if let Some(ctx_id) = context {
            ctx.repr_context_map.insert(entity_id, ctx_id);
        }

        let ref_frame = items.iter().find_map(|r| ctx.placement_map.get(r).copied());
        if let Some(placement_id) = ref_frame {
            ctx.mssr_ref_frame_map.insert(entity_id, placement_id);
        }

        let flattened: Vec<ShellId> = items
            .iter()
            .filter_map(|r| ctx.sbsm_shells_map.get(r))
            .flat_map(|shells| shells.iter().copied())
            .collect();
        ctx.mssr_shells_map.insert(entity_id, flattened.clone());

        // representation-refactor A-1: dual-write into the unified arena.
        let repr_id =
            ctx.representations
                .push(crate::ir::shape_rep::Representation::ManifoldSurface(
                    crate::ir::shape_rep::ManifoldSurfaceRepr {
                        name,
                        context,
                        ref_frame,
                        shells: flattened,
                    },
                ));
        ctx.repr_id_map.insert(entity_id, repr_id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ManifoldSurfaceShapeRepresentationWriteInput {
            product,
            shells,
            unit_ctx,
        }: ManifoldSurfaceShapeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        let axis_ref = buf.emit_axis2_placement_3d(product.shape_ref_frame)?;
        let sbsm_ref = ShellBasedSurfaceModelHandler::write(buf, shells)?;
        Ok(buf.push_simple(
            "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
            vec![
                Attribute::String(String::new()),
                Attribute::List(vec![
                    Attribute::EntityRef(axis_ref),
                    Attribute::EntityRef(sbsm_ref),
                ]),
                Attribute::EntityRef(unit_ctx),
            ],
        ))
    }
}
