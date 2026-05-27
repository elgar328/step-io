//! `BREP_WITH_VOIDS` handler — Pass 5-8.
//!
//! Mirrors the legacy `convert_brep_with_voids` and the multi-shell
//! branch of `emit_solid`. Reads the inner shells via the
//! `oriented_closed_shell_map` populated by Pass 5-7b and, on the read
//! side, overwrites each inner shell's orientation in place rather
//! than cloning so the arena stays free of duplicates.

// IR_PRESSURE: read side mutates `topology.shells[inner_id].orientation`
// in place because `BREP_WITH_VOIDS` has its own ORIENTED_CLOSED_SHELL
// wrappers but the IR `Solid` only stores `Vec<ShellId>`. A future IR
// refactor (Plan 7+) may replace this with an `OrientedShell` arena
// variant so the wrapper stays first-class instead of leaking into the
// Shell record.

use crate::entities::SimpleEntityHandler;
use crate::ir::SolidId;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::topology::{Orientation, Shell, Solid};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct BrepWithVoidsHandler;

#[step_entity(name = "BREP_WITH_VOIDS", pass = Pass5Solid)]
impl SimpleEntityHandler for BrepWithVoidsHandler {
    type WriteInput = SolidId;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "BREP_WITH_VOIDS")?;
        let name_str = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let outer_ref = read_entity_ref(attrs, 1, entity_id, "outer")?;
        let void_refs = read_entity_ref_list(attrs, 2, entity_id, "voids")?;

        let outer_id = ctx.resolve_shell(entity_id, outer_ref, "outer")?;

        let mut shells = Vec::with_capacity(1 + void_refs.len());
        shells.push(outer_id);

        for &ocs_ref in &void_refs {
            let (inner_id, orientation) = *ctx.oriented_closed_shell_map.get(&ocs_ref).ok_or(
                ConvertError::MissingReference {
                    from: entity_id,
                    to: ocs_ref,
                    field_name: "voids",
                },
            )?;
            // Guard against a CS being wrapped by multiple OCS with conflicting
            // orientations, or serving as both outer and inner. Not observed in
            // any fixture so far; if it ever occurs we'd need a copy-based
            // fallback, but for now we surface it as an IR violation.
            let existing = ctx.topology.shells[inner_id].orientation;
            if existing != Orientation::Forward && existing != orientation {
                return Err(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "shared CLOSED_SHELL (ShellId {}) with conflicting \
                         orientations in multiple roles",
                        inner_id.0
                    ),
                });
            }
            ctx.topology.shells[inner_id].orientation = orientation;
            shells.push(inner_id);
        }

        let name = if name_str.is_empty() {
            None
        } else {
            Some(name_str.to_owned())
        };
        let solid = Solid { shells, name };
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

        let mut void_refs = Vec::with_capacity(s.shells.len() - 1);
        for &inner_id in &s.shells[1..] {
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
            let ocs_ref = buf.emit_oriented_closed_shell(inner_cs_ref, inner_shell.orientation)?;
            void_refs.push(Attribute::EntityRef(ocs_ref));
        }
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "BREP_WITH_VOIDS".into(),
                attrs: vec![
                    Attribute::String(name),
                    Attribute::EntityRef(outer_ref),
                    Attribute::List(void_refs),
                ],
            },
        });
        buf.solid_ids.insert(id, n);
        Ok(n)
    }
}
