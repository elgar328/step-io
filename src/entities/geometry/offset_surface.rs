//! `OFFSET_SURFACE` handler.
//!
//! `OFFSET_SURFACE(name, basis_surface, distance, self_intersect)` —
//! wraps another surface as its basis. A chain of `OFFSET_SURFACE` on top of
//! `OFFSET_SURFACE` (or another derived surface) resolves naturally under
//! topological dispatch: each basis is processed before its dependent.

#![allow(clippy::doc_markdown)]

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::SurfaceOfOffset;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct OffsetSurfaceHandler;

#[step_entity(name = "OFFSET_SURFACE")]
impl SimpleEntityHandler for OffsetSurfaceHandler {
    type WriteInput = SurfaceOfOffset;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_offset_surface(entity_id, attrs)?;
        lower::lower_offset_surface(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, o: SurfaceOfOffset) -> Result<u64, WriteError> {
        let basis = buf.emit_surface(o.basis)?;
        let early = lift::lift_offset_surface(basis, o.distance, o.self_intersect);
        Ok(serialize::serialize_offset_surface(buf, &early))
    }
}
