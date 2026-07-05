# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- The full Part 21 HEADER is read into the model: `model.header()` carries
  the file name, timestamp, authors, the originating CAD system, and the
  identified schema — one `FileHeader` type, shared with the write side.
- `write(&model)` — serialize a model to Part 21 text under its own header,
  losslessly.

### Changed

- The identified source schema moved from `Report.schema` to
  `model.header().schema`.

### Removed

- The write-side projection surface — `write_target`, `SchemaTarget`, and
  `LossReport`. The authoring API is AP242 by construction, so its output
  never needed projecting.
- The generated read/write plumbing (`generated::{read, write, walk,
  generic_normalize, schema}`) is crate-internal now; `generated::{model,
  resolve, author}` remain the public raw layer.

## [0.1.0] - 2026-07-04

### Added

- Read every mainstream application protocol — AP203, AP214, and AP242, legacy
  included — and write modern AP242 edition 2 (IS).
- Navigate what you read with `model.scene()` — lightweight handles over b-rep
  geometry, the assembly tree, display meshes, PMI, product metadata,
  presentation, and units.
- Author what you write with `StepBuilder`, which builds files the way kernels
  hold data; `Ap242Author` is the strict low-level layer beneath it.
- Heal non-standard input — what can be fixed is normalized, what cannot is
  dropped with a reason in a provenance `Report`, never silently.
- Run on one lean universal schema model instead of per-schema code, with
  entities that never appear in real STEP files pruned out. The generated
  coverage matrix ([`docs/entities.md`](docs/entities.md)) lists exactly what is
  read and written.

[Unreleased]: https://github.com/elgar328/step-io/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/elgar328/step-io/compare/v0.1.0-alpha.1...v0.1.0
