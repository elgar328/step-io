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

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Surface, SurfaceOfOffset};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct OffsetSurfaceHandler;

impl SimpleEntityHandler for OffsetSurfaceHandler {
    const NAME: &'static str = "OFFSET_SURFACE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4_4Offset;
    type WriteInput = SurfaceOfOffset;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        // Pass 4-4B (multi-round): skip entities already interned by a
        // previous round so the arena does not accumulate duplicates.
        if ctx.surface_map.contains_key(&entity_id) {
            return Ok(());
        }
        check_count(attrs, 4, entity_id, "OFFSET_SURFACE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let basis_ref = read_entity_ref(attrs, 1, entity_id, "basis_surface")?;
        let distance = read_real(attrs, 2, entity_id, "distance")?;
        // [3] self_intersect — informational LOGICAL, skipped (see ROADMAP).

        let basis = ctx.resolve_surface(entity_id, basis_ref, "basis_surface")?;

        let id = ctx
            .geometry
            .surfaces
            .push(Surface::Offset(SurfaceOfOffset { basis, distance }));
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
                    // self_intersect LOGICAL — .F. hardcoded (informational,
                    // not stored in IR; see ROADMAP "LOGICAL 보존").
                    Attribute::Enum("F".into()),
                ],
            },
        });
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static OFFSET_SURFACE_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: OffsetSurfaceHandler::NAME,
    pass_level: OffsetSurfaceHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: OffsetSurfaceHandler::read,
    },
};
