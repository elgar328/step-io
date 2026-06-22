//! `PRODUCT` handler.
//!
//! 2-layer path: the generated `bind` extracts the four attrs and `lower`
//! pushes a `Product` shell whose geometry/category fields are filled in by
//! later sub-passes. Writer emits the lone `PRODUCT(...)` line; the
//! surrounding chain (PRPC / PCATEGORY / formation / PDEF / SR / SDR) lives in
//! `buffer/assembly.rs::emit_assembly_chain` which dispatches through this
//! handler.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::Product;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
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
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_product(entity_id, attrs)?;
        lower::lower_product(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (product, ctx): (Product, AssemblyContextIds),
    ) -> Result<u64, WriteError> {
        let early = lift::lift_product(product, &ctx);
        Ok(serialize::serialize_product(buf, &early))
    }
}
