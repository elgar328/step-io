//! `CURVE_BOUNDED_SURFACE` handler — phase cbs.
//!
//! `bounded_surface` SUBTYPE. 4 attr: `name` + `basis_surface` /
//! `boundaries` / `implicit_outer`. `boundary_curve` is not yet modelled
//! in step-io; the boundaries SET is narrowed to generic `CurveId` via
//! `curve_map`. Corpus 0 instances per `ir.toml` — round-trip test only.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref, read_entity_ref_list, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{CurveBoundedSurface, Surface};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CurveBoundedSurfaceHandler;

#[step_entity(name = "CURVE_BOUNDED_SURFACE")]
impl SimpleEntityHandler for CurveBoundedSurfaceHandler {
    type WriteInput = CurveBoundedSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "CURVE_BOUNDED_SURFACE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let basis_ref = read_entity_ref(attrs, 1, entity_id, "basis_surface")?;
        let bdry_refs = read_entity_ref_list(attrs, 2, entity_id, "boundaries")?;
        let implicit_outer = read_bool(attrs, 3, entity_id, "implicit_outer")?;
        let Some(basis_surface) = ctx.id_cache.get::<crate::ir::id::SurfaceId>(basis_ref) else {
            return Ok(());
        };
        let boundaries: Vec<_> = bdry_refs
            .iter()
            .filter_map(|r| ctx.id_cache.get::<crate::ir::id::CurveId>(*r))
            .collect();
        if boundaries.is_empty() {
            return Ok(());
        }
        let id = ctx
            .geometry
            .surfaces
            .push(Surface::CurveBounded(CurveBoundedSurface {
                name,
                basis_surface,
                boundaries,
                implicit_outer,
            }));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, cbs: CurveBoundedSurface) -> Result<u64, WriteError> {
        let basis_step = buf.emit_surface(cbs.basis_surface)?;
        let mut bdry_attrs = Vec::with_capacity(cbs.boundaries.len());
        for c in cbs.boundaries {
            let step = buf.emit_curve(c)?;
            bdry_attrs.push(Attribute::EntityRef(step));
        }
        Ok(buf.push_simple(
            "CURVE_BOUNDED_SURFACE",
            vec![
                Attribute::String(cbs.name),
                Attribute::EntityRef(basis_step),
                Attribute::List(bdry_attrs),
                Attribute::Enum(if cbs.implicit_outer {
                    "T".into()
                } else {
                    "F".into()
                }),
            ],
        ))
    }
}
