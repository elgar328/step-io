# step-io

STEP (ISO 10303) file I/O for Rust — parse STEP files into a typed,
in-memory model and write them back losslessly.

> ⚠️ **Experimental.** This is an early, incomplete implementation. The
> public API is not finalized and may change completely — expect breaking
> changes at any time.

## Scope

Focused on 3D CAD geometry and assembly across AP203, AP214, and
AP242 (through edition 3):

- **Geometry** — points, directions, placements, curves and surfaces
  (lines, conics, NURBS, swept / offset surfaces)
- **Topology** — vertices, edges, loops, faces, shells, B-rep solids
- **Assembly** — products, product definitions, assembly trees, mapped items
- **Units & contexts** — SI / conversion-based units, uncertainties
- **PMI / GD&T** — datums, tolerances, dimensions, draughting models
- **Presentation** — styled items, colours, layers, annotations
- **PLM** — persons, organisations, approvals, documents, classifications

## Design

A three-stage pipeline:

1. **parser** — a Part 21 lexer + parser producing a raw entity graph
2. **reader** — resolves the graph into a typed, arena-backed model
3. **writer** — serialises the model back to Part 21 text

Round-trip fidelity is the central goal: a file read and written back
reproduces the same data. The reader and writer are validated against a
large corpus of real-world CAD exports.

## Usage

```rust
use step_io::parse;
use step_io::reader::ReaderContext;

let text = std::fs::read_to_string("part.stp")?;

// Parse Part 21 text into a raw entity graph.
let graph = parse(&text)?;

// Resolve the graph into the typed model.
let result = ReaderContext::convert(&graph);
let model = result.model;

// Inspect or modify the model, then serialise back to STEP.
let out = model.write_to_string()?;
```

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE)
at your option.
