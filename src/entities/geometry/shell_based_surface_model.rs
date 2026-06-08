//! `SHELL_BASED_SURFACE_MODEL` handler.
//!
//! Reader resolves the boundary list into `ShellId`s via `shell_map` and
//! caches the per-SBSM list in `sbsm_shells_map` so MSSR can flatten one
//! or more SBSMs into a `SurfaceBody`. Writer wraps the IR shell list in
//! the standard SBSM line; shell entities themselves are emitted by
//! `emit_shell` upstream.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::id::ShellId;
use crate::ir::visualization::{GeometricRepresentationItem, ShellBasedSurfaceModel};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ShellBasedSurfaceModelHandler;

#[step_entity(name = "SHELL_BASED_SURFACE_MODEL")]
impl SimpleEntityHandler for ShellBasedSurfaceModelHandler {
    type WriteInput = Vec<ShellId>;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SHELL_BASED_SURFACE_MODEL")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let shell_refs = read_entity_ref_list(attrs, 1, entity_id, "sbsm_boundary")?;
        let shells: Vec<ShellId> = shell_refs
            .iter()
            .filter_map(|r| ctx.id_cache.get::<crate::ir::id::ShellId>(*r))
            .collect();
        ctx.sbsm_shells_map.insert(entity_id, shells.clone());
        // Also push into the unified `geometric_representation_item` arena so
        // a `STYLED_ITEM` (or any other representation_item consumer) can
        // resolve the SBSM as a single id. MSSR continues to flatten through
        // `sbsm_shells_map` so the dual storage stays consistent.
        let gri_id = ctx.geometric_representation_items.push(
            GeometricRepresentationItem::ShellBasedSurfaceModel(ShellBasedSurfaceModel {
                name,
                shells,
            }),
        );
        ctx.sbsm_id_map.insert(entity_id, gri_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, shells: Vec<ShellId>) -> Result<u64, WriteError> {
        let mut shell_refs = Vec::with_capacity(shells.len());
        for s in shells {
            shell_refs.push(Attribute::EntityRef(buf.emit_shell(s)?));
        }
        Ok(buf.push_simple(
            "SHELL_BASED_SURFACE_MODEL",
            vec![
                Attribute::String(String::new()),
                Attribute::List(shell_refs),
            ],
        ))
    }
}
