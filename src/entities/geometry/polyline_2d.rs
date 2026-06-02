//! `POLYLINE` handler — Pass 4a (2D, pcurve subtree).
//!
//! Sister of `polyline.rs`. The first point ref's coordinate count
//! discriminates: 2D points dispatch here, 3D fall through to the 3D handler.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Curve2d, Polyline2d};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct Polyline2dHandler;

#[step_entity(name = "POLYLINE", is_2d)]
impl SimpleEntityHandler for Polyline2dHandler {
    type WriteInput = Polyline2d;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "POLYLINE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let pt_refs = read_entity_ref_list(attrs, 1, entity_id, "points")?;

        // First cross-ref discriminates: if absent from the 2D arena,
        // this is the 3D POLYLINE — leave it for PolylineHandler.
        let Some(first) = pt_refs.first() else {
            return Ok(());
        };
        if !ctx.point_2d_map.contains_key(first) {
            return Ok(());
        }

        let mut points = Vec::with_capacity(pt_refs.len());
        for r in &pt_refs {
            let Some(&pid) = ctx.point_2d_map.get(r) else {
                return Err(ConvertError::MissingReference {
                    from: entity_id,
                    to: *r,
                    field_name: "points",
                });
            };
            points.push(pid);
        }
        let id = ctx
            .geometry
            .curves_2d
            .push(Curve2d::Polyline(Polyline2d { points }));
        ctx.curve_2d_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, polyline: Polyline2d) -> Result<u64, WriteError> {
        let mut refs = Vec::with_capacity(polyline.points.len());
        for pid in polyline.points {
            refs.push(Attribute::EntityRef(buf.emit_point_2d(pid)?));
        }
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "POLYLINE".into(),
                attrs: vec![Attribute::String(String::new()), Attribute::List(refs)],
            },
        });
        Ok(n)
    }
}
