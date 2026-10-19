# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Orientation` (`Forward` or `Reversed`) says which way a face, a face's
  boundary loop, or an edge in a loop is used. Reading and authoring both use
  it, so an orientation read from a file can be passed straight to
  `StepBuilder`.
- `Report::parse_warnings` lists what the parser repaired or skipped while
  reading, such as a blank attribute or an undecodable string escape. `read()`
  used to discard these.
- `ParseWarning` implements `Display`, printing a one-line message with the
  line and column, and is re-exported as `step_io::ParseWarning`.
- `ParseWarning` gains two warnings for strings: `MalformedStringEscape`, for
  an escape that cannot be decoded and is kept as written, and
  `LowercaseHexEscape`, for a hex escape with lower-case digits, which is
  decoded all the same.

### Changed

- `StepBuilder::face` takes an `Orientation` instead of the `same_sense`
  boolean, and `FaceBoundInput` holds an `Orientation` in each edge pair and
  in its `orientation` field instead of a boolean. The file written is
  unchanged. Replace `true` with `Orientation::Forward` and `false` with
  `Orientation::Reversed`.
- `Face::same_sense()` is replaced by `Face::orientation()`, and
  `Bound::orientation()` and `Bound::oriented_edges()` return `Orientation`
  instead of `bool`. `Edge::same_sense()`, which relates an edge to its curve,
  stays a `bool`.
- Strings are decoded when read: `\X2\…\X0\`, `\X4\…\X0\`, `\X\HH`,
  `\S\` and `\\` become the characters they stand for, and line breaks that
  writers insert to wrap long strings are dropped. `parse()` and `read()`
  return the decoded text, so code that decoded these escapes itself should
  stop doing so.
- Strings are written in plain ASCII, as Part 21 requires: every character
  other than printable ASCII becomes a `\X2\` or `\X4\` escape, and `\`
  becomes `\\`. A string of printable ASCII with no backslash is written as
  before.
- `ParseWarning` is `#[non_exhaustive]`, so matching on it needs a wildcard
  arm.
- `Report` is `#[non_exhaustive]`. Its fields read as before, but it can no
  longer be built with a struct literal outside step-io.
- Entity ids and arena indices are 32-bit: each `…Id` wraps a `u32`,
  `Arena::push` returns a `u32`, `Arena::get` takes one, and the new
  `Arena::indices()` iterates them. References and `EntityKey` take 8 bytes
  instead of 16. A file of more than about four billion entities fails to
  read with the new `Error::TooLarge`.
- String fields of the generated model are `Text`s: indices into the
  model's text table, which stores each distinct string once instead of
  allocating every field (a typical file repeats a few hundred distinct
  strings, such as `NONE`, across a million fields). Read one with
  `model.text(e.name)` instead of `&e.name`; the scene handles' `name()` and
  the like still return `&str`. `MeasureValue`, `StringSelectValue` and
  `IntMeasureValue` hold `Text`s too. `Ap242Author` constructors still take
  `String`s; for values built directly, such as complex parts, make the
  `Text` with the new `Ap242Author::text`.
- A list of at most three reals — a point's `coordinates`, a direction's
  `direction_ratios`, a mesh vertex or normal — is a `Reals3` held inline
  instead of a `Vec<f64>`, so it no longer allocates. It reads as a `&[f64]`;
  build one from an array with `.into()` or from a slice with
  `Reals3::from_slice`. `Ap242Author` takes `Reals3` for these fields and
  now also checks the length of each inner list, such as a mesh vertex.
- On read, such a list with more than three values is dropped as a
  nonstandard value (`list has more values than its type allows`).
- `Error` is `#[non_exhaustive]`, so matching on it needs a wildcard arm.
- `DropKind` is `#[non_exhaustive]`, so matching on it needs a wildcard arm,
  and so is `DropReason`, whose fields read as before but which can no
  longer be built with a struct literal outside step-io.
- `StepBuilder` writes `APPLICATION_PROTOCOL_DEFINITION` with year 2020 (AP242
  edition 2) instead of 2011. Readers ignore the value; only the output text
  changes.
- Documentation and the crate description present step-io as STEP I/O for
  3D CAD applications rather than for CAD kernels.
- `read()` and `parser::parse()` take any `std::io::Read` — a `File`, a
  `&[u8]`, … — instead of the whole file as bytes, and read it a chunk at a
  time, so the file is never held in memory: pass `File::open(path)?`
  instead of `&std::fs::read(path)?`, and `text.as_bytes()` for text already
  in memory. A failing input is the new `Error::Io`.
- `read()` streams the file instead of holding all of it as raw entities
  until the model is built, and takes less than half as long: a 110 MB file
  of one million entities now reads in 0.7 s instead of 1.7 s, and the
  process peaks at about 115 MB of memory instead of 760 MB (with the
  smaller model described below). It reads the same entities and values as
  before.
- `write()` takes the output — a `File`, a `&mut Vec<u8>`, … — and writes
  the text to it 64 KB at a time instead of returning it as a `String`, so
  the text is never held in memory: writing a 90 MB file of one million
  entities takes about 30 MB on top of the model instead of 310 MB, in the
  same time. The output is flushed at the end, so a `File` needs no
  `BufWriter`, and a failing output is returned as an `std::io::Error`.
- `StepBuilder::finish()`, `Ap242Author::finish()` and
  `Ap242Author::finish_with_header()` return the finished `StepModel`
  instead of its text. Write it with
  `step_io::write(&b.finish()?, File::create(path)?)?`, or into a
  `Vec<u8>` to get the text.
- `StepModel::ref_graph()`, which the scene handles build on first use,
  holds its index in two flat arrays instead of a map of lists: for a model
  of one million entities it takes 16 MB instead of 97 MB and builds in
  13 ms instead of 67 ms. `RefGraph::referrers()` returns the same entities
  in the same order.
- Each entity, with the blanks and comments before it, reads as UTF-8 when
  its bytes are valid UTF-8 and as ISO 8859-1 otherwise; the whole file used
  to read one way or the other. A UTF-8 file with one broken character now
  reads the rest of its text correctly instead of reading all of it as
  ISO 8859-1.
- The `parser` module keeps what reading a raw file needs: `parse`, `Graph`
  and its entity types, `Error`, `LexError`, `LexErrorKind`, the `Span` and
  `TokenKind` that errors carry, `MAX_NESTING_DEPTH` and the schema types.
  `parse_bytes` (use `parse`), `Parser`, `Lexer`, `tokenize`, `Token`,
  `Span::slice` and the `entity`, `lexer`, `p21` and `schema` submodules are
  no longer public, and `TokenKind` no longer implements `logos::Logos`.
- `Span::start` and `Span::end` are `u64`, so offsets past 4 GB stay exact on
  32-bit targets.
- `Error::DuplicateEntityId` gives the `line` of the repeat and the
  `first_line` of the entity it repeats instead of a `span`.
- Section markers (`HEADER`, `DATA`, `ENDSEC`, …) ignore ASCII case only, so
  `ENDſEC` no longer reads as `ENDSEC`.
- The `Graph` that `parse()` returns takes about 40% less
  memory, 300 MB instead of 480 MB for the same file, as its attribute lists
  are allocated at their exact length.
- `Report::norm` lists the notes in the order of the entities in the file;
  it used to list the entity-specific fixes first and the generic ones after,
  each in id order.
- `Report::dropped` is sorted by entity id.
- An entity dropped because it references an entity that normalization
  dropped now carries that entity's reason in its cascade label, such as
  `via-required reference is missing`, instead of `via-dangling`, which now
  means only a reference to an id the file does not define.

### Fixed

- step-io builds on Rust 1.85, the minimum supported Rust version it declares.
  Versions 0.1.0 through 0.2.5 used a Rust 1.88 feature and did not build on
  1.85.
- `Occurrence::transform()` places components correctly in assemblies whose
  placement relationship lists the assembly's representation first, as
  SolidWorks writes them. It used to return the inverse placement there.
  `from` is now always the frame in the component and `to` the frame in the
  assembly. A placement whose order cannot be decided is read component-first
  and reported in `Scene::warnings`.
- `StepBuilder` writes assembly placements the way the standard requires.
  Each part placed in an assembly gets a geometric context of its own, the
  frame a component is placed at is an item of the assembly's
  representation, and the transformation starts from the component's
  origin. Placements land where they did before, and a file without
  placements is written unchanged.

## [0.2.5] - 2026-10-03

### Added

- `CurveInput::LineAlong(direction)` authors a straight edge along a unit
  direction the caller already holds, written bit for bit, so a kernel that
  knows the exact direction keeps it instead of the one two rounded vertex
  positions imply. A direction pointing from the end vertex back to the start
  is negated so the line runs with the edge.

### Changed

- A straight edge (`CurveInput::Line`) is written with a `VECTOR` magnitude of
  1 instead of the edge's length. The magnitude is only the line's parameter
  scale — the edge is bounded by its vertices — so readers see the same shape,
  and the value no longer carries the rounding of a length computed from two
  `f64` points. `ProfileInput::Line` already wrote 1.

## [0.2.4] - 2026-07-09

### Changed

- `StepBuilder::solid_with_voids()` now takes a `VoidShellNormals` argument
  declaring which way the void faces' normals point: `AwayFromMaterial` keeps
  them as authored (`ORIENTED_CLOSED_SHELL(..., .T.)`), `TowardMaterial`
  reverses them (`.F.`). This lets kernels pass cavity shells in their native
  orientation instead of the fixed reversal 0.2.3 applied.

## [0.2.3] - 2026-07-09

### Added

- `StepBuilder::solid_with_voids()` authors a solid with internal cavities
  (`BREP_WITH_VOIDS`), such as a hollow casting. Void solids read back like any
  other solid: `Scene::all_solids()`, `Face::solid()`, and `ProductDef::solids()`
  include them, `Solid::voids()` returns the faces of each cavity shell, and
  `StyleTarget::VoidSolid` colours, layers, and hides them.

### Changed

- `StepBuilder::mesh()` now takes `Option<SolidRef>` for the solid a display mesh
  tessellates (was `Option<ManifoldSolidBrepId>`), so a mesh can link to a plain
  solid or a `BREP_WITH_VOIDS`. Existing calls migrate with `Some(id.into())`.

## [0.2.2] - 2026-07-06

### Added

- `Datum::datum_feature()` — the physical `DATUM_FEATURE` a datum is established
  from, as a `Feature` (so its faces/edges are reachable via `Feature::geometry`).

### Changed

- `Report` drop reasons are now plain-language. `DropKind::SlotLocal`/`Nonstandard`
  are renamed to `NonstandardValue`/`NonstandardReference` (a nonstandard attribute
  *value* vs. a nonstandard *reference*), and the per-reason labels read as
  sentences (e.g. `required reference is missing`) instead of terse codes
  (`req-ref<-$`).

### Fixed

- The scene now surfaces `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS` (the
  `PRODUCT_DEFINITION` subtype used for a part that carries documents) as a
  `ProductDef`, so such a part appears in the assembly tree with its geometry
  placed — instead of the part vanishing and its solids orphaning — and its
  documents/approvals/security are reachable.
- A `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS` part now reports its PMI —
  `ProductDef::{features, dimensions, tolerances, datums}` resolve for the
  documented-part subtype as they do for a plain `PRODUCT_DEFINITION`, instead of
  always returning empty.

## [0.2.1] - 2026-07-06

### Fixed

- `ProductDef::solids()` now follows `SHAPE_REPRESENTATION_RELATIONSHIP`
  bridges, so a part reports its solids even when the geometry lives in a
  separate `ADVANCED_BREP_SHAPE_REPRESENTATION` — the common AP242 assembly
  layout, where those solids were previously left unattached.

## [0.2.0] - 2026-07-06

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

[Unreleased]: https://github.com/elgar328/step-io/compare/v0.2.5...HEAD
[0.2.5]: https://github.com/elgar328/step-io/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/elgar328/step-io/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/elgar328/step-io/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/elgar328/step-io/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/elgar328/step-io/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/elgar328/step-io/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/elgar328/step-io/compare/v0.1.0-alpha.1...v0.1.0
