//! `UNCERTAINTY_MEASURE_WITH_UNIT` handler — Pass 0-1b.

// DOMAIN_TBD: catalog ENTITY_GROUPS.md marks this as X but the reader handles length-flavour uncertainty. Catalog 갱신은 별도 작업.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::LengthUncertainty;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct UncertaintyMeasureWithUnitHandler;

#[step_entity(name = "UNCERTAINTY_MEASURE_WITH_UNIT")]
impl SimpleEntityHandler for UncertaintyMeasureWithUnitHandler {
    /// `(LengthUncertainty, unit_step_id, measure_type_name)` — caller
    /// (`emit_unit_context`) already emitted the relevant unit (length,
    /// plane-angle, or solid-angle) and supplies its STEP id; the
    /// `LengthUncertainty` carries the numeric value plus original
    /// `name` / `description` strings; the measure type name is one of
    /// `"LENGTH_MEASURE"`, `"PLANE_ANGLE_MEASURE"`, `"SOLID_ANGLE_MEASURE"`.
    type WriteInput = (LengthUncertainty, u64, &'static str);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "UNCERTAINTY_MEASURE_WITH_UNIT")?;
        let value = match attrs.first() {
            Some(Attribute::Typed { value, .. }) => match value.as_ref() {
                Attribute::Real(v) => *v,
                _ => return Ok(()),
            },
            _ => return Ok(()),
        };
        let unit_ref = read_entity_ref(attrs, 1, entity_id, "unit_component")?;
        let name = read_string_or_unset(attrs, 2, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 3, entity_id, "description")?.to_owned();
        let uncertainty = LengthUncertainty {
            value,
            name,
            description,
        };
        if ctx.length_unit_map.contains_key(&unit_ref) {
            ctx.length_uncertainty_map.insert(entity_id, uncertainty);
        } else if ctx.angle_unit_map.contains_key(&unit_ref) {
            ctx.plane_angle_uncertainty_map
                .insert(entity_id, uncertainty);
        } else if ctx.solid_angle_unit_map.contains_key(&unit_ref) {
            ctx.solid_angle_uncertainty_map
                .insert(entity_id, uncertainty);
        }
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (unc, unit_ref, measure_type): (LengthUncertainty, u64, &'static str),
    ) -> Result<u64, WriteError> {
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "UNCERTAINTY_MEASURE_WITH_UNIT".into(),
                attrs: vec![
                    Attribute::Typed {
                        type_name: measure_type.into(),
                        value: Box::new(Attribute::Real(unc.value)),
                    },
                    Attribute::EntityRef(unit_ref),
                    Attribute::String(unc.name),
                    Attribute::String(unc.description),
                ],
            },
        });
        Ok(n)
    }
}
