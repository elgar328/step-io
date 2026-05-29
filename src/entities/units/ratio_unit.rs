//! `RATIO_UNIT` handler — Pass 0-1 leaf for dimensionless ratio flavour.
//!
//! `RATIO_UNIT` is a SUBTYPE OF `NAMED_UNIT` with no additional attributes —
//! always dimensionless. `CONVERSION_BASED_UNIT` and `SI_UNIT` variants are
//! unobserved in the corpus, so the handler covers only the plain form.
//! Mirrors `SolidAngleUnitHandler` in shape but skips the unit-value enum
//! since ratio has no flavours.

use crate::entities::ComplexEntityHandler;
use crate::entities::units::shared::has_part;
use crate::ir::error::ConvertError;
use crate::ir::units::{NamedUnit, RatioFlavor};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct RatioUnitHandler;

#[step_entity_complex(name = "RATIO_UNIT", pass = Pass0Leaf, required = ["RATIO_UNIT"])]
impl ComplexEntityHandler for RatioUnitHandler {
    /// `target_id`. Ratio has no flavour enum (zero-sized [`RatioFlavor`]),
    /// so the write input is just the pre-reserved step id.
    type WriteInput = (u64, u64);

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        if has_part(parts, "CONVERSION_BASED_UNIT") || has_part(parts, "SI_UNIT") {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: "RATIO_UNIT + SI_UNIT / CONVERSION_BASED_UNIT is unsupported".into(),
            });
            return Ok(());
        }
        let dim_exp = super::shared::read_named_unit_dim_exp(ctx, parts);
        let id = ctx
            .named_units_arena
            .push(NamedUnit::Ratio(RatioFlavor { dim_exp }));
        ctx.named_unit_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (target_id, dim_exp_step): (u64, u64),
    ) -> Result<u64, WriteError> {
        let named_unit_attr = if dim_exp_step == 0 {
            Attribute::Derived
        } else {
            Attribute::EntityRef(dim_exp_step)
        };
        let parts = vec![
            ("NAMED_UNIT".into(), vec![named_unit_attr]),
            ("RATIO_UNIT".into(), vec![]),
        ];
        buf.entities.push(WriterEntity {
            id: target_id,
            body: WriterBody::Complex { parts },
        });
        Ok(target_id)
    }
}
