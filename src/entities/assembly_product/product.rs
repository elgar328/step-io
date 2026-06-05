//! `PRODUCT` handler.
//!
//! Reader populates `product_arena_map` and pushes a `Product` shell whose
//! geometry/category fields are filled in by later sub-passes. Writer emits
//! the lone `PRODUCT(...)` line; the surrounding chain (PRPC / PCATEGORY /
//! formation / PDEF / SR / SDR) lives in `buffer/assembly.rs::emit_assembly_chain`
//! which dispatches through this handler.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::Product;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::id::Placement3dId;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::buffer::assembly::AssemblyContextIds;
use step_io_macros::step_entity;

pub(crate) struct ProductHandler;

#[step_entity(name = "PRODUCT")]
impl SimpleEntityHandler for ProductHandler {
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
        // attrs[3] = frame_of_reference (SET[1:?] OF PRODUCT_CONTEXT).
        // Capture the first ref for per-product context wiring; the resolve
        // to ProductContextId happens in the `resolve_product_contexts`
        // post-pass, once `product_context_id_map` is filled. Corpus is
        // consistently a single-element set.
        let pc_step_ref = match attrs.get(3) {
            Some(Attribute::List(refs)) => match refs.first() {
                Some(Attribute::EntityRef(r)) => Some(*r),
                _ => None,
            },
            _ => None,
        };

        let description = if description_raw.is_empty() {
            None
        } else {
            Some(description_raw.to_owned())
        };

        // Every Product needs a non-optional reference frame. SDR conversion
        // overwrites this with the shape representation's actual refFrame when
        // available; otherwise it defaults to the first AXIS2 in the file.
        // The "no AXIS2 at all" degenerate case is resolved in the
        // `ensure_product_ref_frames` post-pass, which synthesizes an identity
        // — deferred so the decision does not depend on how many placements
        // happen to be read before this PRODUCT (dispatch-order independent).
        let shape_ref_frame = Placement3dId(0);
        let product = Product {
            id: id.to_owned(),
            name: name.to_owned(),
            description,
            geometry: None,
            instances: Vec::new(),
            shape_ref_frame,
            outer_sr_frame: None,
            category: None,
            formation_with_source: false,
            geometry_context: None,
            product_context: None,
            pdef_context: None,
            representation_id: None,
            outer_representation_id: None,
            associated_documents: Vec::new(),
            formation: None,
            pdef: None,
        };
        let pid = ctx.assembly_products.push(product);
        ctx.product_arena_map.insert(entity_id, pid);
        if let Some(r) = pc_step_ref {
            ctx.product_pc_step_refs.insert(pid, r);
        }
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
