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
pub(crate) mod plm;
pub(crate) mod pmi;
pub(crate) mod property;
pub(crate) mod shape_rep;
pub(crate) mod units;
pub(crate) mod visualization;

pub(crate) use assembly_product::*;
pub(crate) use plm::*;
pub(crate) use pmi::*;
pub(crate) use property::*;
pub(crate) use shape_rep::*;
pub(crate) use units::*;
pub(crate) use visualization::*;
