//! `LossReport` — what schema projection dropped when retargeting output.

/// What schema projection dropped when retargeting an output to a specific
/// [`SchemaTarget`](crate::writer::SchemaTarget) — the output-side counterpart
/// of the input-side `NonStandardInput` normalization.
///
/// Returned by [`write_to`](crate::ir::StepModel::write_to),
/// [`write_to_file`](crate::ir::StepModel::write_to_file), and
/// [`write_to_string_with_loss`](crate::ir::StepModel::write_to_string_with_loss).
/// Always **empty** for [`SchemaTarget::Universal`](crate::writer::SchemaTarget::Universal)
/// (the as-is target performs no projection), so the common path's report can be ignored.
///
/// Not `#[must_use]`: the streaming write methods return it, and discarding the
/// (always-empty) `Universal` report is the normal case.
#[derive(Debug, Default, Clone)]
pub struct LossReport {
    /// `(entity_name, reason)` per dropped entity, in drop order.
    dropped: Vec<(String, String)>,
}

impl LossReport {
    /// Record one dropped entity.
    pub(crate) fn record(&mut self, entity_name: impl Into<String>, reason: impl Into<String>) {
        self.dropped.push((entity_name.into(), reason.into()));
    }

    /// Every dropped entity as `(name, reason)`, in drop order. The name is the
    /// simple entity name, or the `+`-joined part names for a complex entity.
    #[must_use]
    pub fn dropped(&self) -> &[(String, String)] {
        &self.dropped
    }

    /// No entity was dropped (always true for `Universal`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.dropped.is_empty()
    }

    /// Number of entities dropped.
    #[must_use]
    pub fn len(&self) -> usize {
        self.dropped.len()
    }

    /// Drop counts grouped by entity name — a compact summary of what was lost.
    #[must_use]
    pub fn by_type(&self) -> std::collections::BTreeMap<String, usize> {
        let mut counts = std::collections::BTreeMap::new();
        for (name, _) in &self.dropped {
            *counts.entry(name.clone()).or_insert(0) += 1;
        }
        counts
    }
}
