//! `RECTANGULAR_TRIMMED_SURFACE` handler.

#![allow(clippy::similar_names)] // `usense` / `vsense` mirror the EXPRESS field names.
//!
//! `RECTANGULAR_TRIMMED_SURFACE(name, basis_surface, u1, u2, usense, v1,
//! v2, vsense)` — parameter-space rectangle trimming a basis surface. The
//! basis can be any other surface (including another derived surface);
//! topological dispatch processes that basis before this trimmed surface.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{RectangularTrimmedSurface, Surface};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct RectangularTrimmedSurfaceHandler;

#[step_entity(name = "RECTANGULAR_TRIMMED_SURFACE")]
impl SimpleEntityHandler for RectangularTrimmedSurfaceHandler {
    type WriteInput = RectangularTrimmedSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        if ctx.surface_map.contains_key(&entity_id) {
            return Ok(());
        }
        check_count(attrs, 8, entity_id, "RECTANGULAR_TRIMMED_SURFACE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let basis_ref = read_entity_ref(attrs, 1, entity_id, "basis_surface")?;
        let u1 = read_real(attrs, 2, entity_id, "u1")?;
        let u2 = read_real(attrs, 3, entity_id, "u2")?;
        let usense = read_bool(attrs, 4, entity_id, "usense")?;
        let v1 = read_real(attrs, 5, entity_id, "v1")?;
        let v2 = read_real(attrs, 6, entity_id, "v2")?;
        let vsense = read_bool(attrs, 7, entity_id, "vsense")?;

        let basis = ctx.resolve_surface(entity_id, basis_ref, "basis_surface")?;
        let id =
            ctx.geometry
                .surfaces
                .push(Surface::RectangularTrimmed(RectangularTrimmedSurface {
                    basis,
                    u1,
                    u2,
                    usense,
                    v1,
                    v2,
                    vsense,
                }));
        ctx.surface_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, rts: RectangularTrimmedSurface) -> Result<u64, WriteError> {
        let basis = buf.emit_surface(rts.basis)?;
        let bool_attr = |b: bool| Attribute::Enum(if b { "T".into() } else { "F".into() });
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "RECTANGULAR_TRIMMED_SURFACE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(basis),
                    Attribute::Real(rts.u1),
                    Attribute::Real(rts.u2),
                    bool_attr(rts.usense),
                    Attribute::Real(rts.v1),
                    Attribute::Real(rts.v2),
                    bool_attr(rts.vsense),
                ],
            },
        });
        Ok(n)
    }
}
