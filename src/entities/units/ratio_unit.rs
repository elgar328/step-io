//! `RATIO_UNIT` handler — dimensionless ratio flavour (2-layer).
//!
//! `RATIO_UNIT` is a SUBTYPE OF `NAMED_UNIT` with no additional attributes —
//! always dimensionless (fixed by its WHERE clause). Only the standalone simple
//! `RATIO_UNIT(dimensions)` entity occurs in the corpus (c3d kernel); it shares
//! the `named_units` arena via [`NamedUnit::Ratio`]. (No complex
//! `(NAMED_UNIT()RATIO_UNIT()…)` form exists in the corpus — a ratio is not an
//! SI base unit; an unobserved complex instance falls through to
//! `warn_unhandled_complex`.)

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::units::DimensionalExponents;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// Standalone simple `RATIO_UNIT(dimensions)` entity — the only form observed
/// in the corpus (c3d kernel). `RATIO_UNIT` is a plain `SUBTYPE OF (named_unit)`
/// (no DERIVE on `dimensions`), so a non-complex instantiation carries the
/// single inherited `dimensions` attribute. Stored in the `named_units` arena
/// via [`NamedUnit::Ratio`](crate::ir::units::NamedUnit::Ratio).
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
        _: crate::early::EarlyGraph<'_>,
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
