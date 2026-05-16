//! Plain `SHAPE_REPRESENTATION` handler — Pass 6-4a.
//!
//! Catches the bare `SHAPE_REPRESENTATION` form used by Group products and
//! by the outer wrapper of Fusion 360 / CATIA indirect-SR chains. Reader
//! captures the first `AXIS2_PLACEMENT_3D` from `items` so the SDR pass can
//! re-emit it as `Product.outer_sr_frame` when the indirection chain is
//! taken. The dispatch registry exact-matches entity names, so ABSR / MSSR
//! never reach this handler.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::assembly::Product;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct ShapeRepresentationWriteInput {
    pub(crate) product: Product,
    pub(crate) unit_ctx: u64,
}

pub(crate) struct ShapeRepresentationHandler;

impl SimpleEntityHandler for ShapeRepresentationHandler {
    const NAME: &'static str = "SHAPE_REPRESENTATION";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6ShapeRep;
    type WriteInput = ShapeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "SHAPE_REPRESENTATION")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let items = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        if let Some(&ctx_id) = ctx.context_id_map.get(&ctx_ref) {
            ctx.repr_context_map.insert(entity_id, ctx_id);
        }

        if let Some(&placement_id) = items.iter().find_map(|r| ctx.placement_map.get(r)) {
            ctx.plain_sr_frame_map.insert(entity_id, placement_id);
        }
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ShapeRepresentationWriteInput { product, unit_ctx }: ShapeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        let axis_ref = buf.emit_axis2_placement_3d(product.shape_ref_frame)?;
        Ok(buf.push_simple(
            "SHAPE_REPRESENTATION",
            vec![
                Attribute::String(String::new()),
                Attribute::List(vec![Attribute::EntityRef(axis_ref)]),
                Attribute::EntityRef(unit_ctx),
            ],
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static SHAPE_REPRESENTATION_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ShapeRepresentationHandler::NAME,
    pass_level: ShapeRepresentationHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ShapeRepresentationHandler::read,
    },
};
