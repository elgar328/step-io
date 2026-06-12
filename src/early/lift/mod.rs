//! `lift`: `StepModel` (L2) → `EarlyModel` (L1), the write-side inverse of
//! [`lower`](super::lower). The kernel only ever produced L2, so `lift`
//! synthesizes a valid L1 from it (not a "reconstruction" of a parsed L1).
//!
//! For migrated entities `lift` is mechanical: the L2 already carries
//! everything the canonical output form needs. Cross-references are resolved
//! to their **output** step id (`#N`) via the writer's `StepIdCache`, so the
//! L1 node carries the same `#N`-as-`u64` form its read-side counterpart held
//! (read = input `#N`, write = output `#N`). [`serialize`](super::serialize)
//! then emits Part21 text mechanically.
//!
//! One submodule per domain, mirroring `src/entities/`; everything is
//! re-exported flat so handlers call `lift::lift_<entity>(..)` regardless of
//! domain.

pub(crate) mod assembly_product;
pub(crate) mod shape_rep;
pub(crate) mod visualization;

pub(crate) use assembly_product::*;
pub(crate) use shape_rep::*;
pub(crate) use visualization::*;
