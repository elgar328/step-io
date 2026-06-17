//! `RATIO_UNIT` handler leaf for dimensionless ratio flavour.
//!
//! `RATIO_UNIT` is a SUBTYPE OF `NAMED_UNIT` with no additional attributes —
//! always dimensionless (fixed by its WHERE clause). Two forms occur:
//! [`RatioUnitHandler`] for the complex `(NAMED_UNIT()RATIO_UNIT()…)`
//! instantiation (`CONVERSION_BASED_UNIT` / `SI_UNIT` variants are unobserved)
//! and [`RatioUnitSimpleHandler`] for the standalone simple
//! `RATIO_UNIT(dimensions)` entity (c3d kernel). Both share the `named_units`
//! arena via [`NamedUnit::Ratio`]; `RatioFlavor.complex` records the form.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::units::shared::has_part;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::units::{DimensionalExponents, NamedUnit, RatioFlavor};
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
        // RATIO_UNIT.dimensions is a required entity_ref, but c3d emits `$`
        // (Unset). Normalize a non-ref `dimensions` to a synthetic dimensionless
        // DIMENSIONAL_EXPONENTS before the strict generated bind (the WHERE
        // clause fixes RATIO_UNIT dimensionless regardless). L1 stays strict.
        let attrs = if let Some(Attribute::EntityRef(_)) = attrs.first() {
            attrs.to_vec()
        } else {
            ctx.ns_record(
                crate::reader::NsCase::RatioUnitDimensionsUnset,
                "RATIO_UNIT.dimensions (Unset/Derived)".into(),
                "synthetic dimensionless DIMENSIONAL_EXPONENTS",
            );
            let synth_id = ctx.alloc_synthetic_entity_id();
            let de_id = ctx.dimensional_exponents.push(DimensionalExponents {
                length_exponent: 0.0,
                mass_exponent: 0.0,
                time_exponent: 0.0,
                electric_current_exponent: 0.0,
                thermodynamic_temperature_exponent: 0.0,
                amount_of_substance_exponent: 0.0,
                luminous_intensity_exponent: 0.0,
            });
            ctx.id_cache.insert(synth_id, de_id);
            vec![Attribute::EntityRef(synth_id)]
        };
        let early = bind::bind_ratio_unit(entity_id, &attrs)?;
        lower::lower_ratio_unit(ctx, entity_id, &early);
        Ok(())
    }

    /// Fresh-id serialize (trait contract). The units pool emitter calls
    /// `serialize_ratio_unit_with_id` directly at the pre-reserved id, so this
    /// is not on the hot path; `target_id` is ignored here.
    fn write(
        buf: &mut WriteBuffer,
        (_target_id, dim_exp_step): (u64, u64),
    ) -> Result<u64, WriteError> {
        Ok(serialize::serialize_ratio_unit(
            buf,
            &lift::lift_ratio_unit(dim_exp_step),
        ))
    }
}
