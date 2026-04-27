//! `UNCERTAINTY_MEASURE_WITH_UNIT` handler — Pass 0-1b.

// DOMAIN_TBD: catalog ENTITY_GROUPS.md marks this as X but the reader handles length-flavour uncertainty. Catalog 갱신은 별도 작업.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct UncertaintyMeasureWithUnitHandler;

impl SimpleEntityHandler for UncertaintyMeasureWithUnitHandler {
    const NAME: &'static str = "UNCERTAINTY_MEASURE_WITH_UNIT";
    const PASS_LEVEL: PassLevel = PassLevel::Pass0Uncertainty;
    /// `(value, length_unit_step_id)` — caller (`emit_unit_context`)
    /// already emitted the length unit and supplies its STEP id.
    type WriteInput = (f64, u64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
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
        // attrs[2] = name (보통 'distance_accuracy_value'), attrs[3] = description — 무시.
        if ctx.length_unit_map.contains_key(&unit_ref) {
            ctx.length_uncertainty_map.insert(entity_id, value);
        }
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, (value, length_unit): (f64, u64)) -> Result<u64, WriteError> {
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "UNCERTAINTY_MEASURE_WITH_UNIT".into(),
                attrs: vec![
                    Attribute::Typed {
                        type_name: "LENGTH_MEASURE".into(),
                        value: Box::new(Attribute::Real(value)),
                    },
                    Attribute::EntityRef(length_unit),
                    Attribute::String("distance_accuracy_value".into()),
                    Attribute::String("confusion accuracy".into()),
                ],
            },
        });
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static UNCERTAINTY_MEASURE_WITH_UNIT_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: UncertaintyMeasureWithUnitHandler::NAME,
    pass_level: UncertaintyMeasureWithUnitHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: UncertaintyMeasureWithUnitHandler::read,
    },
};
