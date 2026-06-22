//! Output `SchemaProfile` — hand query layer over the baked per-target legal
//! sets + header constants (`generated::profile`).
//!
//! Consumed by the write-side projection. `Universal` carries no legal set (no
//! projection — emit as-is); each `Ap*` resolves to that target's baked legal
//! entity set (UPPER wire form, sorted) + `FILE_SCHEMA` / `APD` header.

use crate::early::generated::profile as baked;
use crate::writer::SchemaTarget;

/// Resolved output profile for one [`SchemaTarget`].
pub(crate) struct SchemaProfile {
    /// Legal entity names, UPPER wire form, sorted (`binary_search`). `None` for
    /// `Universal` — everything is legal (no projection).
    legal: Option<&'static [&'static str]>,
    /// `FILE_SCHEMA` descriptor string(s) to declare. `None` for `Universal`
    /// (keep the model's source-preserved header).
    file_schema: Option<&'static [&'static str]>,
    /// `APPLICATION_PROTOCOL_DEFINITION` `(status, application, year)` +
    /// `APPLICATION_CONTEXT` description. `None` for `Universal` (keep the
    /// model's source AC/APD).
    apd: Option<(&'static str, &'static str, i64, &'static str)>,
    /// Lossless subtype -> supertype downgrades (UPPER wire form, sorted by
    /// subtype). `None` for `Universal` (no projection). Consulted by the
    /// projection to rename a target-illegal subtype to its legal supertype
    /// before dropping.
    downgrade: Option<&'static [(&'static str, &'static str)]>,
}

impl SchemaProfile {
    /// Resolve the profile for `target` from the baked constants.
    pub(crate) fn for_target(target: SchemaTarget) -> Self {
        match target {
            SchemaTarget::Universal => Self {
                legal: None,
                file_schema: None,
                apd: None,
                downgrade: None,
            },
            SchemaTarget::Ap214 => Self {
                legal: Some(baked::AP214E3_LEGAL),
                file_schema: Some(baked::AP214E3_FILE_SCHEMA),
                apd: Some(baked::AP214E3_APD),
                downgrade: Some(baked::AP214E3_DOWNGRADE),
            },
            SchemaTarget::Ap242 => Self {
                legal: Some(baked::AP242E2_LEGAL),
                file_schema: Some(baked::AP242E2_FILE_SCHEMA),
                apd: Some(baked::AP242E2_APD),
                downgrade: Some(baked::AP242E2_DOWNGRADE),
            },
            SchemaTarget::Ap203 => Self {
                legal: Some(baked::AP203E2_LEGAL),
                file_schema: Some(baked::AP203E2_FILE_SCHEMA),
                apd: Some(baked::AP203E2_APD),
                downgrade: Some(baked::AP203E2_DOWNGRADE),
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

    /// APD `(description, status, schema_name, year)` to synthesize, or `None`
    /// to keep source. Tuple order matches the writer's `apd_info` fallback so
    /// callers bind both sources uniformly.
    pub(crate) fn apd(&self) -> Option<(&'static str, &'static str, &'static str, i64)> {
        self.apd
            .map(|(status, name, year, desc)| (desc, status, name, year))
    }

    /// The legal supertype to rename `name` (UPPER wire form) to, if it is a
    /// target-illegal subtype with a lossless downgrade. `None` for `Universal`
    /// or when no downgrade applies (the entity is then left to be dropped).
    pub(crate) fn downgrade(&self, name: &str) -> Option<&'static str> {
        let table = self.downgrade?;
        table
            .binary_search_by(|(sub, _)| (*sub).cmp(name))
            .ok()
            .map(|i| table[i].1)
    }
}
