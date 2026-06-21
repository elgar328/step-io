//! Output `SchemaProfile` — hand query layer over the baked per-target legal
//! sets + header constants (`generated::profile`).
//!
//! Consumed by the write-side projection (batch 2c). `Universal` carries no
//! legal set (no projection — emit as-is); each `Ap*` resolves to that target's
//! baked legal entity set (UPPER wire form, sorted) + `FILE_SCHEMA` / `APD` header.

use crate::early::generated::profile as baked;
use crate::writer::SchemaTarget;

/// Resolved output profile for one [`SchemaTarget`].
#[allow(dead_code)] // fields/methods consumed by the projection pass (batch 2c)
pub(crate) struct SchemaProfile {
    /// Legal entity names, UPPER wire form, sorted (`binary_search`). `None` for
    /// `Universal` — everything is legal (no projection).
    legal: Option<&'static [&'static str]>,
    /// `FILE_SCHEMA` descriptor string(s) to declare. `None` for `Universal`
    /// (keep the model's source-preserved header).
    file_schema: Option<&'static [&'static str]>,
    /// `APPLICATION_PROTOCOL_DEFINITION` `(status, application, year)`. `None` for
    /// `Universal` (keep the model's source APD).
    apd: Option<(&'static str, &'static str, i64)>,
}

#[allow(dead_code)] // consumed by the projection pass (batch 2c)
impl SchemaProfile {
    /// Resolve the profile for `target` from the baked constants.
    pub(crate) fn for_target(target: SchemaTarget) -> Self {
        match target {
            SchemaTarget::Universal => Self {
                legal: None,
                file_schema: None,
                apd: None,
            },
            SchemaTarget::Ap214 => Self {
                legal: Some(baked::AP214E3_LEGAL),
                file_schema: Some(baked::AP214E3_FILE_SCHEMA),
                apd: Some(baked::AP214E3_APD),
            },
            SchemaTarget::Ap242 => Self {
                legal: Some(baked::AP242E2_LEGAL),
                file_schema: Some(baked::AP242E2_FILE_SCHEMA),
                apd: Some(baked::AP242E2_APD),
            },
            SchemaTarget::Ap203 => Self {
                legal: Some(baked::AP203E2_LEGAL),
                file_schema: Some(baked::AP203E2_FILE_SCHEMA),
                apd: Some(baked::AP203E2_APD),
            },
        }
    }

    /// No-projection target (`Universal`)?
    pub(crate) fn is_universal(&self) -> bool {
        self.legal.is_none()
    }

    /// Is `name` (UPPER wire form) legal in this target? `Universal` = always.
    /// Relies on the baked set being sorted in UPPER-case order.
    pub(crate) fn is_legal(&self, name: &str) -> bool {
        match self.legal {
            None => true,
            Some(set) => set.binary_search(&name).is_ok(),
        }
    }

    /// `FILE_SCHEMA` descriptor(s) to declare, or `None` to keep source header.
    pub(crate) fn file_schema(&self) -> Option<&'static [&'static str]> {
        self.file_schema
    }

    /// APD `(status, application, year)` to synthesize, or `None` to keep source.
    pub(crate) fn apd(&self) -> Option<(&'static str, &'static str, i64)> {
        self.apd
    }
}

/// What the projection dropped when retargeting — the output-side counterpart of
/// the input-side `NonStandardInput`. Empty under `Universal`.
#[derive(Debug, Default)]
#[allow(dead_code)] // consumed by the projection pass (batch 2c)
pub(crate) struct LossReport {
    /// `(entity_name, reason)` per dropped entity, in drop order.
    dropped: Vec<(String, String)>,
}

#[allow(dead_code)] // consumed by the projection pass (batch 2c)
impl LossReport {
    /// Record one dropped entity.
    pub(crate) fn record(&mut self, entity_name: impl Into<String>, reason: impl Into<String>) {
        self.dropped.push((entity_name.into(), reason.into()));
    }

    /// Dropped entities recorded so far.
    pub(crate) fn dropped(&self) -> &[(String, String)] {
        &self.dropped
    }
}
