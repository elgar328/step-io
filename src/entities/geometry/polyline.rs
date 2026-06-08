//! `POLYLINE` handler leaf 3D polyline.
//!
//! Subtype of `BOUNDED_CURVE`. Stores an ordered point id list. The 2D
//! sister variant is handled by `polyline_2d.rs` — discriminated
//! by the first point ref's coordinate count.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Curve, Polyline};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct PolylineHandler;

#[step_entity(name = "POLYLINE")]
impl SimpleEntityHandler for PolylineHandler {
    type WriteInput = Polyline;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "POLYLINE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let pt_refs = read_entity_ref_list(attrs, 1, entity_id, "points")?;

        // If the first referenced point is a known 2D point, this POLYLINE
        // is the 2D sister variant — silently skip (Polyline2dHandler picks it up).
        if let Some(first) = pt_refs.first()
            && ctx.point_2d_map.contains_key(first)
        {
            return Ok(());
        }

        let mut points = Vec::with_capacity(pt_refs.len());
        for r in &pt_refs {
            points.push(ctx.resolve_point(entity_id, *r, "points")?);
        }
        let id = ctx
            .geometry
            .curves
            .push(Curve::Polyline(Polyline { points }));
        ctx.curve_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, polyline: Polyline) -> Result<u64, WriteError> {
        let mut refs = Vec::with_capacity(polyline.points.len());
        for pid in polyline.points {
            refs.push(Attribute::EntityRef(buf.emit_point(pid)?));
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
