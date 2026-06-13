//! `MANIFOLD_SOLID_BREP` handler — solid wrapping an outer shell (2-layer
//! path; name preserved).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::SolidId;
use crate::ir::error::ConvertError;
use crate::ir::topology::Solid;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ManifoldSolidBrepHandler;

#[step_entity(name = "MANIFOLD_SOLID_BREP")]
impl SimpleEntityHandler for ManifoldSolidBrepHandler {
    type WriteInput = SolidId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_manifold_solid_brep(entity_id, attrs)?;
        lower::lower_manifold_solid_brep(ctx, entity_id, &early)
    }

    fn write(buf: &mut WriteBuffer, id: SolidId) -> Result<u64, WriteError> {
        let cached = buf.step_id(id);
        if cached != 0 {
            return Ok(cached);
        }
        let s: Solid = buf
            .model
            .topology
            .solids
            .iter()
            .nth(id.0 as usize)
            .cloned()
            .ok_or_else(|| WriteError::DanglingId {
                detail: format!("SolidId({})", id.0),
            })?;
        let outer = buf.emit_shell(s.outer())?;
        let name = s.name().unwrap_or_default().to_owned();
        let early = lift::lift_manifold_solid_brep(name, outer);
        let n = serialize::serialize_manifold_solid_brep(buf, &early);
        buf.set_step_id(id, n);
        Ok(n)
    }
}
