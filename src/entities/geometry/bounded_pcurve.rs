//! `BOUNDED_PCURVE` handler — phase bpc.
//!
//! `parameter_space_curve` SUBTYPE — orphan in step-io (no inbound refs).
//! 3 attr inherited from `pcurve` (`name` + `basis_surface` + `reference_to_curve`).
//! `reference_to_curve` narrows to a `definitional_representation` — step-io
//! stores it as a plain `RepresentationId`.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::geometry::{BoundedPCurve, ParameterSpaceCurve};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct BoundedPCurveHandler;

#[step_entity(name = "BOUNDED_PCURVE")]
impl SimpleEntityHandler for BoundedPCurveHandler {
    type WriteInput = BoundedPCurve;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "BOUNDED_PCURVE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let basis_ref = read_entity_ref(attrs, 1, entity_id, "basis_surface")?;
        let ref_ref = read_entity_ref(attrs, 2, entity_id, "reference_to_curve")?;
        let Some(&basis_surface) = ctx.surface_map.get(&basis_ref) else {
            return Ok(());
        };
        let Some(&reference_to_curve) = ctx.repr_id_map.get(&ref_ref) else {
            return Ok(());
        };
        ctx.geometry
            .parameter_space_curves
            .push(ParameterSpaceCurve::BoundedPCurve(BoundedPCurve {
                name,
                basis_surface,
                reference_to_curve,
            }));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, b: BoundedPCurve) -> Result<u64, WriteError> {
        let basis_step = buf.emit_surface(b.basis_surface)?;
        let ref_step = buf.step_id(b.reference_to_curve);
        Ok(buf.push_simple(
            "BOUNDED_PCURVE",
            vec![
                Attribute::String(b.name),
                Attribute::EntityRef(basis_step),
                Attribute::EntityRef(ref_step),
            ],
        ))
    }
}
