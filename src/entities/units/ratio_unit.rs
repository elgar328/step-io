//! `RATIO_UNIT` handler leaf for dimensionless ratio flavour.
//!
//! `RATIO_UNIT` is a SUBTYPE OF `NAMED_UNIT` with no additional attributes —
//! always dimensionless (fixed by its WHERE clause). Two forms occur:
//! [`RatioUnitHandler`] for the complex `(NAMED_UNIT()RATIO_UNIT()…)`
//! instantiation (`CONVERSION_BASED_UNIT` / `SI_UNIT` variants are unobserved)
//! and [`RatioUnitSimpleHandler`] for the standalone simple
//! `RATIO_UNIT(dimensions)` entity (c3d kernel). Both share the `named_units`
//! arena via [`NamedUnit::Ratio`]; `RatioFlavor.complex` records the form.

use crate::entities::units::shared::has_part;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::units::{NamedUnit, RatioFlavor};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::{step_entity, step_entity_complex};

pub(crate) struct RatioUnitHandler;

// No RATIO_UNIT complex instances in the corpus; cases follow the invariant
// unit-complex structure shared by the other named units (SI / CBU forms).
#[step_entity_complex(name = "RATIO_UNIT", cases = [
    ["CONVERSION_BASED_UNIT", "NAMED_UNIT", "RATIO_UNIT"],
    ["NAMED_UNIT", "RATIO_UNIT", "SI_UNIT"],
])]
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
        let id = ctx.named_units_arena.push(NamedUnit::Ratio(RatioFlavor {
            dim_exp,
            complex: true,
        }));
        ctx.id_cache.insert(entity_id, id);
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

/// Standalone simple `RATIO_UNIT(dimensions)` entity — the only form observed
/// in the corpus (c3d kernel). `RATIO_UNIT` is a plain `SUBTYPE OF (named_unit)`
/// (no DERIVE on `dimensions`), so a non-complex instantiation carries the
/// single inherited `dimensions` attribute. Shares the `named_units` arena
/// with the complex handler via [`NamedUnit::Ratio`]; the `RatioFlavor.complex`
/// flag records which form to re-emit.
pub(crate) struct RatioUnitSimpleHandler;

#[step_entity(name = "RATIO_UNIT")]
impl SimpleEntityHandler for RatioUnitSimpleHandler {
    /// `(target_id, dim_exp_step)` — reserved step id + resolved
    /// `DIMENSIONAL_EXPONENTS` ref (`0` → emit `$`).
    type WriteInput = (u64, u64);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "RATIO_UNIT")?;
        let dim_exp = match attrs.first() {
            Some(Attribute::EntityRef(n)) => ctx
                .id_cache
                .get::<crate::ir::id::DimensionalExponentsId>(*n),
            Some(Attribute::Unset) => {
                // [NS-ratio-unit-dimensions-unset] c3d: RATIO_UNIT.dimensions is
                // required by EXPRESS but emitted `$` (Unset). Accept as no
                // explicit dimensions — the WHERE clause fixes it dimensionless
                // regardless. See reader::nonstandard.
                ctx.record_nonstandard("RATIO_UNIT.dimensions (Unset)".into(), "no dimensions");
                None
            }
            _ => None, // `*` (Derived) — dimensionless by the WHERE clause
        };
        let id = ctx.named_units_arena.push(NamedUnit::Ratio(RatioFlavor {
            dim_exp,
            complex: false,
        }));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        (target_id, dim_exp_step): (u64, u64),
    ) -> Result<u64, WriteError> {
        let dim_attr = if dim_exp_step == 0 {
            Attribute::Unset
        } else {
            Attribute::EntityRef(dim_exp_step)
        };
        buf.push_simple_with_id(target_id, "RATIO_UNIT", vec![dim_attr]);
        Ok(target_id)
    }
}
