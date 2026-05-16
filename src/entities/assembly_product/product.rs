//! `PRODUCT` handler — Pass 6-1.
//!
//! Reader populates `product_arena_map` and pushes a `Product` shell whose
//! geometry/category fields are filled in by later sub-passes. Writer emits
//! the lone `PRODUCT(...)` line; the surrounding chain (PRPC / PCATEGORY /
//! formation / PDEF / SR / SDR) lives in `buffer/assembly.rs::emit_assembly_chain`
//! which dispatches through this handler.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::assembly::{Product, ProductContent};
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::id::Placement3dId;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::buffer::assembly::AssemblyContextIds;

pub(crate) struct ProductHandler;

impl SimpleEntityHandler for ProductHandler {
    const NAME: &'static str = "PRODUCT";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6Product;
    /// `(product, context)` — `Product` clones from IR (single-emit per
    /// product, so the clone is incidental), `AssemblyContextIds` is `Copy`.
    type WriteInput = (Product, AssemblyContextIds);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "PRODUCT")?;
        let id = read_string_or_unset(attrs, 0, entity_id, "id")?;
        let name = read_string_or_unset(attrs, 1, entity_id, "name")?;
        let description_raw = read_string_or_unset(attrs, 2, entity_id, "description")?;
        // attrs[3] = frame_of_reference (list of PRODUCT_CONTEXT) — ignored.

        let description = if description_raw.is_empty() {
            None
        } else {
            Some(description_raw.to_owned())
        };

        // Every Product needs a non-optional reference frame. SDR conversion
        // overwrites this with the shape representation's actual refFrame when
        // available. As a placeholder, reuse the first AXIS2 already pushed
        // during the geometry passes so the arena count stays faithful to the
        // source file. Only fall back to pushing a fresh identity when no
        // AXIS2 exists (degenerate fixture).
        #[allow(clippy::cast_possible_truncation)]
        let shape_ref_frame = if ctx.geometry.placements.is_empty() {
            ctx.geometry.identity_placement()
        } else {
            Placement3dId(0)
        };
        let product = Product {
            id: id.to_owned(),
            name: name.to_owned(),
            description,
            content: ProductContent::Group(Vec::new()),
            shape_ref_frame,
            outer_sr_frame: None,
            category: None,
            formation_with_source: false,
            geometry_context: None,
        };
        let pid = ctx.assembly_products.push(product);
        ctx.product_arena_map.insert(entity_id, pid);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (product, ctx): (Product, AssemblyContextIds),
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PRODUCT",
            vec![
                Attribute::String(product.id),
                Attribute::String(product.name),
                Attribute::String(product.description.unwrap_or_default()),
                Attribute::List(vec![Attribute::EntityRef(ctx.product_ctx)]),
            ],
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static PRODUCT_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ProductHandler::NAME,
    pass_level: ProductHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ProductHandler::read,
    },
};
