//! Topology entity converters (Pass 5-1 through 5-8).

use super::ReaderContext;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string};
use crate::ir::error::ConvertError;
use crate::ir::topology::{Orientation, Solid};
use crate::parser::entity::Attribute;

impl ReaderContext {
    // ------------------------------------------------------------------
    // Pass 5-8: MANIFOLD_SOLID_BREP (depends on CLOSED_SHELL)
    // ------------------------------------------------------------------

    pub(super) fn convert_manifold_solid_brep(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "MANIFOLD_SOLID_BREP")?;
        let name_str = read_string(attrs, 0, entity_id, "name")?;
        let shell_ref = read_entity_ref(attrs, 1, entity_id, "outer")?;

        let shell_id = self.resolve_shell(entity_id, shell_ref, "outer")?;

        let name = if name_str.is_empty() {
            None
        } else {
            Some(name_str.to_owned())
        };

        let solid = Solid {
            shells: vec![shell_id],
            name,
        };
        let id = self.topology.solids.push(solid);
        self.solid_map.insert(entity_id, id);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 5-8: BREP_WITH_VOIDS (depends on CLOSED_SHELL + OCS map)
    //
    // Overwrites each inner shell's orientation in place rather than
    // cloning — keeps the arena free of unreferenced duplicates.
    // ------------------------------------------------------------------

    pub(super) fn convert_brep_with_voids(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "BREP_WITH_VOIDS")?;
        let name_str = read_string(attrs, 0, entity_id, "name")?;
        let outer_ref = read_entity_ref(attrs, 1, entity_id, "outer")?;
        let void_refs = read_entity_ref_list(attrs, 2, entity_id, "voids")?;

        let outer_id = self.resolve_shell(entity_id, outer_ref, "outer")?;

        let mut shells = Vec::with_capacity(1 + void_refs.len());
        shells.push(outer_id);

        for &ocs_ref in &void_refs {
            let (inner_id, orientation) = *self.oriented_closed_shell_map.get(&ocs_ref).ok_or(
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
            let existing = self.topology.shells[inner_id].orientation;
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
            self.topology.shells[inner_id].orientation = orientation;
            shells.push(inner_id);
        }

        let name = if name_str.is_empty() {
            None
        } else {
            Some(name_str.to_owned())
        };
        let solid = Solid { shells, name };
        let id = self.topology.solids.push(solid);
        self.solid_map.insert(entity_id, id);
        Ok(())
    }
}
