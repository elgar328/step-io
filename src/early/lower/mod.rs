//! `lower`: `EarlyModel` (L1) → `StepModel` (L2). The hand-written semantic
//! glue — SELECT resolution, flatten, normalization live here. Each migrated
//! entity's `lower` reproduces exactly what its previous one-shot handler
//! produced (same arena, same insertion order), so the writer is untouched and
//! corpus output stays byte-identical.
//!
//! One submodule per domain, mirroring `src/entities/`; everything is
//! re-exported flat so handlers call `lower::lower_<entity>(..)` regardless of
//! domain.

pub(crate) mod assembly_product;
pub(crate) mod visualization;

pub(crate) use assembly_product::*;
pub(crate) use visualization::*;
