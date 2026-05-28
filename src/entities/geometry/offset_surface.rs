//! `OFFSET_SURFACE` handler — Pass 4-4B (fixpoint dispatch).
//!
//! `OFFSET_SURFACE(name, basis_surface, distance, self_intersect)` —
//! wraps another surface as its basis. When that basis is itself an
//! `OFFSET_SURFACE` or a Pass 4-4A derived surface that comes later in
//! entity-id order, a single sweep fails to resolve. `passes.rs` calls
//! [`ReaderContext::dispatch_registry_until_fixpoint`] with this pass
//! level and the `geometry.surfaces.len()` measure, repeating until the
//! arena stops growing.

#![allow(clippy::doc_markdown)]

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{
    check_count, logical_to_step, read_entity_ref, read_logical, read_real, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Surface, SurfaceOfOffset};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct OffsetSurfaceHandler;

#[step_entity(name = "OFFSET_SURFACE", pass = Pass4_4Offset)]
impl SimpleEntityHandler for OffsetSurfaceHandler {
    type WriteInput = SurfaceOfOffset;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // Pass 4-4B (multi-round): skip entities already interned by a
        // previous round so the arena does not accumulate duplicates.
        if ctx.surface_map.contains_key(&entity_id) {
            return Ok(());
        }
        check_count(attrs, 4, entity_id, "OFFSET_SURFACE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let basis_ref = read_entity_ref(attrs, 1, entity_id, "basis_surface")?;
        let distance = read_real(attrs, 2, entity_id, "distance")?;
        let self_intersect = read_logical(attrs, 3, entity_id, "self_intersect")?;

        let basis = ctx.resolve_surface(entity_id, basis_ref, "basis_surface")?;

        let id = ctx.geometry.surfaces.push(Surface::Offset(SurfaceOfOffset {
            basis,
            distance,
            self_intersect,
        }));
        ctx.surface_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, o: SurfaceOfOffset) -> Result<u64, WriteError> {
        let basis = buf.emit_surface(o.basis)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "OFFSET_SURFACE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(basis),
                    Attribute::Real(o.distance),
                    Attribute::Enum(logical_to_step(o.self_intersect).into()),
                ],
            },
        });
        Ok(n)
    }
}
