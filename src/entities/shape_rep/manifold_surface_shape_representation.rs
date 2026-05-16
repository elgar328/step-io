//! `MANIFOLD_SURFACE_SHAPE_REPRESENTATION` handler — Pass 6-4a.
//!
//! Reader resolves each item that points at an SBSM and flattens the
//! collected shells into `mssr_shells_map`; the first `AXIS2_PLACEMENT_3D`
//! provides the coordinate reference frame. Writer mirrors the structure:
//! emit a per-product axis placement, then an SBSM wrapping all shells,
//! and reference both from the MSSR line.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::assembly::Product;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string};
use crate::ir::error::ConvertError;
use crate::ir::id::ShellId;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use crate::entities::geometry::shell_based_surface_model::ShellBasedSurfaceModelHandler;

pub(crate) struct ManifoldSurfaceShapeRepresentationWriteInput {
    pub(crate) product: Product,
    pub(crate) shells: Vec<ShellId>,
    pub(crate) unit_ctx: u64,
}

pub(crate) struct ManifoldSurfaceShapeRepresentationHandler;

impl SimpleEntityHandler for ManifoldSurfaceShapeRepresentationHandler {
    const NAME: &'static str = "MANIFOLD_SURFACE_SHAPE_REPRESENTATION";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6ShapeRep;
    type WriteInput = ManifoldSurfaceShapeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "MANIFOLD_SURFACE_SHAPE_REPRESENTATION")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let items = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        if let Some(&ctx_id) = ctx.context_id_map.get(&ctx_ref) {
            ctx.repr_context_map.insert(entity_id, ctx_id);
        }

        if let Some(&placement_id) = items.iter().find_map(|r| ctx.placement_map.get(r)) {
            ctx.mssr_ref_frame_map.insert(entity_id, placement_id);
        }

        let flattened: Vec<ShellId> = items
            .iter()
            .filter_map(|r| ctx.sbsm_shells_map.get(r))
            .flat_map(|shells| shells.iter().copied())
            .collect();
        ctx.mssr_shells_map.insert(entity_id, flattened);
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static MSSR_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ManifoldSurfaceShapeRepresentationHandler::NAME,
    pass_level: ManifoldSurfaceShapeRepresentationHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ManifoldSurfaceShapeRepresentationHandler::read,
    },
};
