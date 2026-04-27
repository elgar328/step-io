//! Topology entity converters (Pass 5-1 through 5-8).

use super::{ReaderContext, bool_to_orientation};
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_entity_ref_list, read_string};
use crate::ir::error::ConvertError;
use crate::ir::topology::{Face, FaceKind, Orientation, Shell, Solid, Wire};
use crate::parser::entity::Attribute;

impl ReaderContext {
    // ------------------------------------------------------------------
    // Pass 5-5: FACE_BOUND / FACE_OUTER_BOUND (depends on EDGE_LOOP)
    // ------------------------------------------------------------------

    pub(super) fn convert_face_bound(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
        is_outer: bool,
    ) -> Result<(), ConvertError> {
        let entity_name = if is_outer {
            "FACE_OUTER_BOUND"
        } else {
            "FACE_BOUND"
        };
        check_count(attrs, 3, entity_id, entity_name)?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let loop_ref = read_entity_ref(attrs, 1, entity_id, "bound")?;
        let orientation = read_bool(attrs, 2, entity_id, "orientation")?;

        // `loop_ref` targets either an EDGE_LOOP (edges) or a VERTEX_LOOP
        // (single degenerate vertex). Try both maps before reporting a
        // missing reference.
        let wire = if self.edge_loop_map.contains_key(&loop_ref) {
            let edges = self.resolve_edge_loop(entity_id, loop_ref, "bound")?;
            Wire {
                edges,
                vertex: None,
                is_outer,
                orientation: bool_to_orientation(orientation),
            }
        } else if let Some(&vertex) = self.vertex_loop_map.get(&loop_ref) {
            Wire {
                edges: Vec::new(),
                vertex: Some(vertex),
                is_outer,
                orientation: bool_to_orientation(orientation),
            }
        } else {
            return Err(ConvertError::MissingReference {
                from: entity_id,
                to: loop_ref,
                field_name: "bound",
            });
        };
        let id = self.topology.wires.push(wire);
        self.face_bound_map.insert(entity_id, id);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 5-6: ADVANCED_FACE / FACE_SURFACE (depends on FACE_BOUND + SURFACE)
    // ------------------------------------------------------------------

    pub(super) fn convert_face(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
        kind: FaceKind,
    ) -> Result<(), ConvertError> {
        let step_name = match kind {
            FaceKind::Advanced => "ADVANCED_FACE",
            FaceKind::General => "FACE_SURFACE",
        };
        check_count(attrs, 4, entity_id, step_name)?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let bound_refs = read_entity_ref_list(attrs, 1, entity_id, "bounds")?;
        let surface_ref = read_entity_ref(attrs, 2, entity_id, "face_geometry")?;
        let same_sense = read_bool(attrs, 3, entity_id, "same_sense")?;

        let mut bounds = Vec::with_capacity(bound_refs.len());
        for &r in &bound_refs {
            let wire_id = self.resolve_face_bound(entity_id, r, "bounds")?;
            bounds.push(wire_id);
        }

        let surface = self.resolve_surface(entity_id, surface_ref, "face_geometry")?;

        let face = Face {
            surface,
            bounds,
            orientation: bool_to_orientation(same_sense),
            kind,
        };
        let id = self.topology.faces.push(face);
        self.face_map.insert(entity_id, id);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 5-7: CLOSED_SHELL (depends on ADVANCED_FACE)
    // ------------------------------------------------------------------

    pub(super) fn convert_closed_shell(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        self.convert_shell_inner(entity_id, attrs, "CLOSED_SHELL", false)
    }

    pub(super) fn convert_open_shell(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        self.convert_shell_inner(entity_id, attrs, "OPEN_SHELL", true)
    }

    fn convert_shell_inner(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
        entity_name: &str,
        is_open: bool,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, entity_name)?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let face_refs = read_entity_ref_list(attrs, 1, entity_id, "cfs_faces")?;

        let mut faces = Vec::with_capacity(face_refs.len());
        for &r in &face_refs {
            let face_id = self.resolve_face(entity_id, r, "cfs_faces")?;
            faces.push(face_id);
        }

        let shell = Shell {
            faces,
            orientation: Orientation::Forward,
            is_open,
        };
        let id = self.topology.shells.push(shell);
        self.shell_map.insert(entity_id, id);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Pass 5-7b: ORIENTED_CLOSED_SHELL (depends on CLOSED_SHELL)
    //
    // Records the wrapper in `oriented_closed_shell_map` so
    // `convert_brep_with_voids` can later apply the wrapper's orientation
    // to the referenced shell. Does not push into the Shell arena.
    // ------------------------------------------------------------------

    pub(super) fn convert_oriented_closed_shell(
        &mut self,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "ORIENTED_CLOSED_SHELL")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        // attrs[1] is the derived `*` field — skip.
        let shell_ref = read_entity_ref(attrs, 2, entity_id, "closed_shell_element")?;
        let orientation = read_bool(attrs, 3, entity_id, "orientation")?;

        let shell_id = self.resolve_shell(entity_id, shell_ref, "closed_shell_element")?;
        self.oriented_closed_shell_map
            .insert(entity_id, (shell_id, bool_to_orientation(orientation)));
        Ok(())
    }

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
