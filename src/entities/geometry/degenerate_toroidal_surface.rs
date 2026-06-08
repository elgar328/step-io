//! `DEGENERATE_TOROIDAL_SURFACE` handler leaf surface.
//!
//! Subtype of `TOROIDAL_SURFACE` adding the `select_outer` boolean which
//! chooses the outer or inner sheet when the minor radius produces a
//! degenerate (self-intersecting) torus.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::axis2_placement_3d::Axis2Placement3dHandler;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{DegenerateToroidalSurface, Surface};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct DegenerateToroidalSurfaceHandler;

#[step_entity(name = "DEGENERATE_TOROIDAL_SURFACE")]
impl SimpleEntityHandler for DegenerateToroidalSurfaceHandler {
    type WriteInput = DegenerateToroidalSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 5, entity_id, "DEGENERATE_TOROIDAL_SURFACE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let pos_ref = read_entity_ref(attrs, 1, entity_id, "position")?;
        let major_radius = read_real(attrs, 2, entity_id, "major_radius")?;
        let minor_radius = read_real(attrs, 3, entity_id, "minor_radius")?;
        let select_outer = read_bool(attrs, 4, entity_id, "select_outer")?;

        let position = ctx.resolve_placement(entity_id, pos_ref, "position")?;

        let id =
            ctx.geometry
                .surfaces
                .push(Surface::DegenerateToroidal(DegenerateToroidalSurface {
                    position,
                    major_radius,
                    minor_radius,
                    select_outer,
                }));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dts: DegenerateToroidalSurface) -> Result<u64, WriteError> {
        let pos = Axis2Placement3dHandler::write(buf, dts.position)?;
        let bool_attr = Attribute::Enum(if dts.select_outer {
            "T".into()
        } else {
            "F".into()
        });
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "DEGENERATE_TOROIDAL_SURFACE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(pos),
                    Attribute::Real(dts.major_radius),
                    Attribute::Real(dts.minor_radius),
                    bool_attr,
                ],
            },
        });
        Ok(n)
    }
}
