//! `MANIFOLD_SOLID_BREP` handler.
//!
//! Mirrors the legacy `convert_manifold_solid_brep` and the
//! `Solid::ManifoldSolidBrep` branch of `emit_solid`.

use crate::entities::SimpleEntityHandler;
use crate::ir::SolidId;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::topology::Solid;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
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
        check_count(attrs, 2, entity_id, "MANIFOLD_SOLID_BREP")?;
        let name_str = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let shell_ref = read_entity_ref(attrs, 1, entity_id, "outer")?;

        let shell_id = ctx.resolve_shell(entity_id, shell_ref, "outer")?;

        let name = if name_str.is_empty() {
            None
        } else {
            Some(name_str.to_owned())
        };

        let solid = Solid::ManifoldSolidBrep {
            outer: shell_id,
            name,
        };
        let id = ctx.topology.solids.push(solid);
        ctx.id_cache.insert(entity_id, id);
        Ok(())
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
        let outer_ref = buf.emit_shell(s.outer())?;
        let name = s.name().unwrap_or_default().to_owned();
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "MANIFOLD_SOLID_BREP".into(),
                attrs: vec![Attribute::String(name), Attribute::EntityRef(outer_ref)],
            },
        });
        buf.set_step_id(id, n);
        Ok(n)
    }
}
