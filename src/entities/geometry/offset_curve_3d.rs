//! `OFFSET_CURVE_3D` handler — Pass 4-4B (fixpoint dispatch).
//!
//! `OFFSET_CURVE_3D(name, basis_curve, distance, self_intersect,
//! ref_direction)` — wraps another 3D curve as its basis offset by
//! `distance` along `ref_direction`. Forward-ref to the basis curve is
//! tolerated by the `Pass4_4Offset` fixpoint loop (same mechanism as
//! `OFFSET_SURFACE`).

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{
    check_count, logical_to_step, read_entity_ref, read_logical, read_real, read_string,
};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{Curve, OffsetCurve3d};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct OffsetCurve3dHandler;

#[step_entity(name = "OFFSET_CURVE_3D", pass = Pass4_4Offset)]
impl SimpleEntityHandler for OffsetCurve3dHandler {
    type WriteInput = OffsetCurve3d;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        if ctx.curve_map.contains_key(&entity_id) {
            return Ok(());
        }
        check_count(attrs, 5, entity_id, "OFFSET_CURVE_3D")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let basis_ref = read_entity_ref(attrs, 1, entity_id, "basis_curve")?;
        let distance = read_real(attrs, 2, entity_id, "distance")?;
        let self_intersect = read_logical(attrs, 3, entity_id, "self_intersect")?;
        let direction_ref = read_entity_ref(attrs, 4, entity_id, "ref_direction")?;

        let basis = ctx.resolve_curve(entity_id, basis_ref, "basis_curve")?;
        let ref_direction = ctx.resolve_direction(entity_id, direction_ref, "ref_direction")?;

        let id = ctx
            .geometry
            .curves
            .push(Curve::OffsetCurve3d(OffsetCurve3d {
                basis,
                distance,
                self_intersect,
                ref_direction,
            }));
        ctx.curve_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, oc: OffsetCurve3d) -> Result<u64, WriteError> {
        let basis = buf.emit_curve(oc.basis)?;
        let dir = buf.emit_direction(oc.ref_direction)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "OFFSET_CURVE_3D".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(basis),
                    Attribute::Real(oc.distance),
                    Attribute::Enum(logical_to_step(oc.self_intersect).into()),
                    Attribute::EntityRef(dir),
                ],
            },
        });
        Ok(n)
    }
}
