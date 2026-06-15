//! `CONSTRUCTIVE_GEOMETRY_REPRESENTATION` handler — phase cgr.
//!
//! `representation` SUBTYPE carrying a SET of geometry items narrowed by
//! EXPRESS WHERE wr2 to `placement` / `curve` / `edge` / `face` /
//! `point` / `surface` / `face_surface` / `vertex_point`. step-io's
//! `resolve_representation_item_ref` covers all eight types via the
//! per-arena id maps. Items unresolved by the resolver are skipped
//! (symmetric on re-read).
//!
//! Emit is delayed (`Mdgpr` / `DraughtingModel` / `TSR` pattern) —
//! `emit_representations_pre_pass` skips this variant, and
//! `emit_constructive_geometry_representations` writes into the
//! `representation_step_ids` slot by `RepresentationId`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::ConstructiveGeometryRepr;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ConstructiveGeometryRepresentationHandler;

#[step_entity(name = "CONSTRUCTIVE_GEOMETRY_REPRESENTATION")]
impl SimpleEntityHandler for ConstructiveGeometryRepresentationHandler {
    type WriteInput = ConstructiveGeometryRepr;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_constructive_geometry_representation(entity_id, attrs)?;
        lower::lower_constructive_geometry_representation(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, cgr: ConstructiveGeometryRepr) -> Result<u64, WriteError> {
        let early = lift::lift_constructive_geometry_representation(buf, cgr)?;
        Ok(serialize::serialize_constructive_geometry_representation(
            buf, &early,
        ))
    }
}
