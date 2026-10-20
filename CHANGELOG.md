# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Orientation` (`Forward` or `Reversed`) says which way a face, a face's
  boundary loop, an edge in a loop, or a shell in a solid is used. Reading
  and authoring both use it, so an orientation read from a file can be
  passed straight to `StepBuilder`.
- `Report::parse_warnings` lists what the parser repaired or skipped while
  reading, such as a blank attribute or an undecodable string escape. `read()`
  used to discard these.
- `ParseWarning` implements `Display`, printing a one-line message with the
  line and column, and is re-exported as `step_io::ParseWarning`.
- `ParseWarning` gains two warnings for strings: `MalformedStringEscape`, for
  an escape that cannot be decoded and is kept as written, and
  `LowercaseHexEscape`, for a hex escape with lower-case digits, which is
  decoded all the same.
- `PARABOLA` is read and written. A file using one used to lose the curve,
  and with it the edges and faces built on it; Open CASCADE writes a
  parabola wherever a plane cuts a cone parallel to a side.
- `CurveKind::Parabola` and `CurveKind::Hyperbola` give the two curves with
  their parameters (focal distance; semi-axes).
- `to_nurbs()` converts parabolas and hyperbolas exactly: the span of an
  edge, a trimmed curve or a composite curve's segment becomes a quadratic
  B-spline for a parabola and rational quadratic pieces for a hyperbola. A
  trim given as a point is followed rather than its parameter: writers
  disagree on a parabola's parameter, Open CASCADE writing `2F·u` where the
  standard has `u`.
- `CurveInput::Parabola` and `CurveInput::Hyperbola` let `StepBuilder::edge`
  write an edge along a parabola or a hyperbola branch. Its `same_sense` says
  whether the edge runs with the curve's parameter, found from the vertices;
  a zero focal distance, a semi-axis not positive, or vertices at the same
  parameter are refused with `AuthorError::InvalidInput`.
- `Shell` is a closed shell of a solid. `Solid::outer()` gives the outer
  shell and `Solid::voids()` each cavity's shell, both with the
  `Orientation` the solid uses the shell in: `Reversed` when it is reached
  through an `ORIENTED_CLOSED_SHELL` with `.F.`, as Open CASCADE writes every
  cavity, while other writers keep `.T.`. A face of a shell points the way
  its own orientation and the shell's give together.
- `Contributor::key()` and `Approver::key()`, so every scene handle has a
  key, and every handle implements `Debug`, printing its type and key.
  `FeatureGeometry` and `Target` implement `Debug` as well, and
  `FeatureGeometry` is `Clone` and `Copy`.
- `Version::product()` reaches the product a version belongs to.
- `SurfaceModel` is a surface body, a `SHELL_BASED_SURFACE_MODEL`: its
  `shells()`, each with the `Orientation` the model uses it in, its `faces()`,
  and colour, transparency, layer and visibility. `Scene::all_surface_models()`
  lists them, and `ProductDef::surface_models()` and
  `MappedInstance::surface_models()` reach them from a part, through manifold
  surface representations and the representations bridged to them;
  `ProductDef::solids()` and `MappedInstance::solids()` follow the same
  representations.
- `Shell` covers open shells as well (`is_closed()`), reaches back to its
  `solid()` or `surface_model()`, and has colour, transparency, layer and
  visibility; `Face::shell()` gives the shell listing a face.
- `GeometricSet` is a `GEOMETRIC_SET` or `GEOMETRIC_CURVE_SET` — a wireframe,
  a sketch, a set of points kept outside any b-rep. Its `elements()` are
  `SetElement`s: a `Curve`, a `Point`, or anything else by key. It has colour,
  layer and visibility. `Scene::all_geometric_sets()` lists them, and
  `ProductDef::geometric_sets()` and `MappedInstance::geometric_sets()` reach
  them from a part.
- Presentation on more handles: `Edge` reads the curve style styled onto the
  `EDGE_CURVE` itself (`color()`, `width()`, `line_font()`), `Point` reads a
  point style's marker colour, `Mesh` and `MeshGroup` read colour and
  transparency, and these, together with `Curve`, have `layer()` and
  `is_visible()`.
- `FeatureGeometry::Mesh` and `FeatureGeometry::GeometricSet` give a feature's
  display mesh or geometric set, which used to come back as `Other`.
- `EntityKey::keyword()` gives the STEP keyword of the entity type a key
  names, such as `ADVANCED_FACE`, or `None` for a complex instance.
- `Bound::vertex()` gives the vertex of a `VERTEX_LOOP` boundary, such as a
  cone's apex, which has no edges.
- `Curve::curve_3d()` reaches the 3D curve of a `SURFACE_CURVE`,
  `SEAM_CURVE`, `BOUNDED_SURFACE_CURVE` or `INTERSECTION_CURVE` as a `Curve`,
  where it used to take the raw model.
- `SceneWarning` names the entity a warning concerns, the handle whose
  method ran into it, next to the message.

### Changed

- A hyperbola's `CurveKind` is `Hyperbola` instead of `Other("HYPERBOLA")`.
- `Solid::voids()` returns an iterator of `(Shell, Orientation)` pairs, one
  per cavity, instead of a `Vec` of face lists. It used to drop the
  orientation, so a cavity's faces could not be told to point into the
  hole. Use `shell.faces()` for the faces.
- `Scene::dimensions()`, `tolerances()`, `datums()` and `features()` are
  renamed `all_dimensions()`, `all_tolerances()`, `all_datums()` and
  `all_features()`, as every other model-wide list is named `all_*`. The
  same names on a `ProductDef` stay as they are.
- Every move from a handle to any number of other handles returns an
  iterator. These used to return a `Vec`: `MeshGroup::meshes`,
  `Product::{contributors, approvals, documents, security_classifications,
  versions}`, `ProductDef::{contributors, approvals, documents,
  security_classifications}`, `assigned_targets` on `Contributor`,
  `Approval`, `Document` and `SecurityClassification`, `Approval::approvers`,
  `MappedInstance::{solids, mapped_children}`, `Tolerance::datums`,
  `Dimension::features` and `Feature::geometry`. The order is the same; add
  `.collect::<Vec<_>>()` where a list is needed.
- `ProductDef::version()` returns the `Version` handle instead of its id
  string; call `.id()` on it for the label.
- `CurveKind`, `SurfaceKind`, `FeatureKind`, `ToleranceKind`,
  `FeatureGeometry`, `Units`, `Unit` and `ApprovalDate` are
  `#[non_exhaustive]`, so later kinds and fields can be added without a
  breaking release. A `match` on one of the enums needs a wildcard arm, and
  the structs are read field by field rather than built or compared as a
  literal.
- `Scene::warnings()` returns `SceneWarning`s instead of strings. A
  `SceneWarning` prints as the message the string held, so `{w}` formatting
  is unchanged; read `.message` for the text. Asking the same handle again no
  longer repeats a warning.
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
- `AuthorError` is `#[non_exhaustive]`, so matching on it needs a wildcard
  arm.
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
  in memory. A failing input is the new `Error::Io`, which holds the
  input's `std::io::Error`.
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
  and its entity types, the `Span` that errors carry, and
  `MAX_NESTING_DEPTH`. `Error`, `LexError`,
  `LexErrorKind`, `ParseWarning`, `SchemaId`, `ApFamily` and `Stage` are at
  the crate root only. `parse_bytes` (use `parse`), `Parser`, `Lexer`,
  `tokenize`, `Token`, `Span::slice` and the `entity`, `lexer`, `p21` and
  `schema` submodules are no longer public, nor is `TokenKind`.
- Each public item has one path. The generated model is `step_io::model`
  (was `step_io::generated::model`, and `generated::resolve` for the
  `…RefView` types). The `reader`, `emit`, `header`, `refgraph` and
  `generated` modules are private: what they held is at the crate root.
  `scene`'s submodules are private and all their types are at `scene::`, such as
  `scene::Face` (was `scene::geometry::Face`) and `scene::ProductDef` (was
  `scene::product::ProductDef`). `build` no longer re-exports `NurbsCurve`,
  `NurbsSurface` and `Rgb`; use `scene::`. `StepBuilder`, `StepModel` and
  `EntityKey` stay at the crate root as well as in their modules.
- The model is read only from outside the crate. Its per-type entity lists
  (the `…_arena` fields) and `Arena` are no longer public; read the model
  with `model.all::<T>()`, which lists the entities of type `T` with their
  ids (was `model.t_arena.items`), and `model.get(id)` or `model[id]` (was
  `model.t_arena.get(id.0)`), through the new `Entity` and `EntityId`
  traits. New entities enter only through the `Ap242Author` constructors,
  which check them; pushing to an arena skipped the checks, and a model
  changed that way could make `write` panic or write a non-conforming file.
- The generated enums' `parse` and `token` and the reference enums'
  `from_any` are no longer public. `resolve` and `entity_key` stay.
- `Span::start` and `Span::end` are `u64`, so offsets past 4 GB stay exact on
  32-bit targets.
- `Error::DuplicateEntityId` gives the `line` of the repeat and the
  `first_line` of the entity it repeats instead of a `span`.
- `Error` no longer implements `Clone` and `PartialEq`, as `Error::Io` holds
  an `std::io::Error`; match on its variants instead. Its message includes
  a wrapped lexical or input error's, and `source()` is `None`, so an error
  chain prints each message once.
- `Error::UnexpectedToken` names the token `found` as a `String`, about as
  written in the file (`FOO`, `')'`, `#12`), instead of a `TokenKind`, and
  the message quotes a punctuation mark it expected (`expected ';'`).
- `LexError` and `LexErrorKind` are `#[non_exhaustive]`. A string the file
  never closes is `UnterminatedString` and a malformed binary literal
  `InvalidBinary`; both used to be `UnexpectedCharacter`.
- The `step_io::Result` alias is removed; write
  `Result<T, step_io::Error>`.
- `parser::Graph` is `#[non_exhaustive]`.
- `Report`'s counts are `entities_read`, `entities_added` and
  `entities_kept` (were `n_in`, `n_synth` and `validated`), and
  `DropReason::key` is `DropReason::detail`. `DropKind` and `DropReason`
  implement `Display` (`cascade: via-dangling`).
- A file whose `FILE_SCHEMA` names no schema reads as `ApFamily::Other`; it
  used to read as AP214, the default `ApFamily` had. `ApFamily` has no
  default now, and `ApFamily`, `Stage` and `SchemaId` are
  `#[non_exhaustive]`.
- `SchemaId::raw` is the `Vec<String>` of `FILE_SCHEMA` names, empty when the
  file gives none; `NonEmptyStringList`, the `raw()` method and
  `SchemaId::synthetic()` are removed.
- `SchemaId` prints only what the file pins, `AP214` or `AP214 (CD)` rather
  than `AP214 ed? (?)`, and an unrecognised schema prints its names rather
  than `unrecognized schema`.
- Section markers (`HEADER`, `DATA`, `ENDSEC`, …) ignore ASCII case only, so
  `ENDſEC` no longer reads as `ENDSEC`.
- The `Graph` that `parse()` returns takes about 40% less
  memory, 300 MB instead of 480 MB for the same file, as its attribute lists
  are allocated at their exact length.
- `Report::normalized` (was `norm`) lists each note with the id of the
  entity it fixed, in the order of the entities in the file; it used to list
  the notes alone, the entity-specific fixes first and the generic ones
  after, each in id order. The entity-specific notes read as sentences like
  the others (`missing surface colour set to an unspecified COLOUR`).
- `Report::dropped` is sorted by entity id.
- An entity dropped because it references an entity that normalization
  dropped now carries that entity's reason in its cascade label, such as
  `via-required reference is missing`, instead of `via-dangling`, which now
  means only a reference to an id the file does not define.
- `StepBuilder::place` refuses a placement that would make an assembly
  contain itself, the part placed being the assembly or already containing
  it, with `AuthorError::CyclicAssembly`. The standard forbids such cycles,
  and a reader following one may never stop.
- `StepBuilder` refuses a NURBS curve or surface that breaks the standard's
  B-spline rules (degree at least 1, more control points than the degree,
  knot count, knots that never decrease, knot repeats, one positive weight
  per control point) and a mesh without triangles, with an index past its
  points, or with per-vertex normals that do not number the points, with
  the new `AuthorError::InvalidInput`.
- The types of a complex instance take Part 21's names: `ComplexEntity`
  (was `ComplexUnit`) and its `ComplexEntityId`, `PartialEntity` (was
  `UnitPart`) for each entity type it combines, and `EntityKey::ComplexEntity`.
  They hold every complex instance, not only units.
- The `Ap242Author` constructors take their strings as `&str`, an optional
  one as `Option<&str>`, and `add_person`'s name lists as `Option<&[&str]>`,
  instead of `String`s: write `a.add_product("p1", "part", None, …)` rather
  than `"p1".to_owned()`.
- The `Ap242Author` constructors check that a complex instance given for a
  reference is of a fitting type: a unit is refused where a curve belongs,
  with `AuthorError::InvalidComplex`. They used to check only that it
  existed.

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
- `to_nurbs()` no longer panics or runs out of memory on values a file
  cannot be trusted with. It returns `None`, with a warning in
  `Scene::warnings`, for a B-spline of degree 0 or above 25, one with fewer
  control points than its degree needs, one whose knot multiplicities do not
  add up to its control points, a circle or ellipse trimmed at angles too
  large to subtract, and a B-spline trimmed outside its parameter range.
- A circle or ellipse trimmed more than a full turn apart converts to the
  arc within one turn, as OCCT reads it. It used to wind round as many times
  as the angles said, and a trim like `1.E17` never finished.
- `Scene::root_definitions()` no longer lists a
  `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS` that an assembly uses as a
  component; it only checked plain `PRODUCT_DEFINITION` components, so such a
  part showed both under its assembly and as a root of its own.
- `Feature::geometry()` returns a `FACE_SURFACE` as a `Face` and a
  `BREP_WITH_VOIDS` as a `Solid`, and `Mesh::face()` reaches a linked
  `FACE_SURFACE`. They used to give `Other` and `None`.
- `StepBuilder` and `Ap242Author` refuse a NaN or infinite real with the new
  `AuthorError::NonFiniteReal`. They used to write it as `NaN` or `inf`,
  which Part 21 has no spelling for, so the file could not be read back.
- The `Ap242Author` `add_*_cyclic` constructors check aggregate sizes, as
  the other constructors do; an empty `SET [1:?]` used to pass.
- `Face::to_nurbs()` reads a cone's half-angle in the file's plane-angle unit.
  It used to take it as radians, so a cone in a file that measures angles in
  degrees came out with the wrong radius.
- `Face::to_nurbs()` converts a cone face that closes at its apex, bounded by
  a circle and a `VERTEX_LOOP` at the apex or by the circle alone. It used to
  return `None` there for a degenerate extent.
- `read()` reports a Part 21 edition 3 `ANCHOR` or `REFERENCE` section in
  `Report::parse_warnings` (`ParseWarning::Ed3SectionDiscarded`), as it
  already did `SIGNATURE`; the model has no place for them, and they used to
  be dropped without a word. `parser::parse()` keeps them in its `Graph` as
  before.
- A face boundary that is a `POLY_LOOP`, a bare `LOOP` or a complex instance
  is reported in `Scene::warnings`; `Bound::edges()` used to come back empty
  without a word.
- `MeshGroup::solid()` finds the solid of a tessellated shell linked to a
  closed shell that the solid reaches through an `ORIENTED_CLOSED_SHELL`, a
  cavity included, as `Face::solid()` does. It used to find only an outer
  shell referenced directly.

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
