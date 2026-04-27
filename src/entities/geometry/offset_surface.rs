//! `OFFSET_SURFACE` handler — Pass 4-4B. A chain of `OFFSET_SURFACE` on
//! top of `OFFSET_SURFACE` may resolve in any order, so the dispatcher uses
//! [`ReaderContext::dispatch_registry_until_fixpoint`] with the
//! `geometry.surfaces.len()` measure. Body wired in Plan 7 stage C6.
