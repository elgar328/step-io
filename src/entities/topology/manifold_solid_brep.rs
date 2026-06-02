//! `MANIFOLD_SOLID_BREP` handler — Pass 5-8.
//!
//! Mirrors the legacy `convert_manifold_solid_brep` and the
//! single-shell branch of `emit_solid`. The shared `solid_id_to_name`
//! helper builds the optional `name` value common to both solid kinds.

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

        let solid = Solid {
            shells: vec![shell_id],
            name,
        };
        let id = ctx.topology.solids.push(solid);
        ctx.solid_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, id: SolidId) -> Result<u64, WriteError> {
        if let Some(&n) = buf.solid_ids.get(&id) {
            return Ok(n);
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
        let outer_id = *s.shells.first().ok_or_else(|| WriteError::DanglingId {
            detail: format!("SolidId({}) has no shells", id.0),
        })?;
        let outer_ref = buf.emit_shell(outer_id)?;
        let name = s.name.clone().unwrap_or_default();
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "MANIFOLD_SOLID_BREP".into(),
                attrs: vec![Attribute::String(name), Attribute::EntityRef(outer_ref)],
            },
        });
        buf.solid_ids.insert(id, n);
        Ok(n)
    }
}
