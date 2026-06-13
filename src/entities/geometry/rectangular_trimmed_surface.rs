//! `RECTANGULAR_TRIMMED_SURFACE` handler (2-layer path).
//!
//! `RECTANGULAR_TRIMMED_SURFACE(name, basis_surface, u1, u2, v1, v2, usense,
//! vsense)` — parameter-space rectangle trimming a basis surface. The basis can
//! be any other surface (incl. another derived surface); topological dispatch
//! processes that basis first. The EXPRESS order places `usense`/`vsense` after
//! `v2` (the prior hand handler transposed `usense` to slot 4 — corpus-absent,
//! so the latent bug never surfaced; the generated bind reads the schema order).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::geometry::RectangularTrimmedSurface;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct RectangularTrimmedSurfaceHandler;

#[step_entity(name = "RECTANGULAR_TRIMMED_SURFACE")]
impl SimpleEntityHandler for RectangularTrimmedSurfaceHandler {
    type WriteInput = RectangularTrimmedSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_rectangular_trimmed_surface(entity_id, attrs)?;
        lower::lower_rectangular_trimmed_surface(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, rts: RectangularTrimmedSurface) -> Result<u64, WriteError> {
        let basis = buf.emit_surface(rts.basis)?;
        let early = lift::lift_rectangular_trimmed_surface(
            basis, rts.u1, rts.u2, rts.usense, rts.v1, rts.v2, rts.vsense,
        );
        Ok(serialize::serialize_rectangular_trimmed_surface(
            buf, &early,
        ))
    }
}
