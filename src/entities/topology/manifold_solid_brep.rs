//! `MANIFOLD_SOLID_BREP` handler — Pass 5-8.
//!
//! Mirrors the legacy `convert_manifold_solid_brep` and the
//! single-shell branch of `emit_solid`. The shared `solid_id_to_name`
//! helper builds the optional `name` value common to both solid kinds.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::SolidId;
use crate::ir::attr::{check_count, read_entity_ref, read_string};
use crate::ir::error::ConvertError;
use crate::ir::topology::Solid;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct ManifoldSolidBrepHandler;

impl SimpleEntityHandler for ManifoldSolidBrepHandler {
    const NAME: &'static str = "MANIFOLD_SOLID_BREP";
    const PASS_LEVEL: PassLevel = PassLevel::Pass5Solid;
    type WriteInput = SolidId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "MANIFOLD_SOLID_BREP")?;
        let name_str = read_string(attrs, 0, entity_id, "name")?;
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static MANIFOLD_SOLID_BREP_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ManifoldSolidBrepHandler::NAME,
    pass_level: ManifoldSolidBrepHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ManifoldSolidBrepHandler::read,
    },
};
