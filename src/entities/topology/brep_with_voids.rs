//! `BREP_WITH_VOIDS` handler — `manifold_solid_brep` plus inner void shells
//! (2-layer path). The void resolution (orientation written back onto the inner
//! shell in place — the IR `Solid` stores only `Vec<ShellId>`) lives in
//! `lower`; the write emits each void's `ORIENTED_CLOSED_SHELL` wrapper.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::SolidId;
use crate::ir::error::ConvertError;
use crate::ir::topology::{Shell, Solid};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct BrepWithVoidsHandler;

#[step_entity(name = "BREP_WITH_VOIDS")]
impl SimpleEntityHandler for BrepWithVoidsHandler {
    type WriteInput = SolidId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_brep_with_voids(entity_id, attrs)?;
        lower::lower_brep_with_voids(ctx, entity_id, &early)
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

        let mut voids = Vec::with_capacity(s.voids().len());
        for &inner_id in s.voids() {
            let inner_shell: Shell = buf
                .model
                .topology
                .shells
                .iter()
                .nth(inner_id.0 as usize)
                .cloned()
                .ok_or_else(|| WriteError::DanglingId {
                    detail: format!("ShellId({}) void", inner_id.0),
                })?;
            let inner_cs_ref = buf.emit_shell(inner_id)?;
            voids.push(buf.emit_oriented_closed_shell(inner_cs_ref, inner_shell.orientation)?);
        }
        let early = lift::lift_brep_with_voids(name, outer, voids);
        let n = serialize::serialize_brep_with_voids(buf, &early);
        buf.set_step_id(id, n);
        Ok(n)
    }
}
