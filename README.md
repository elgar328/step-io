# step-io

[![crates.io](https://img.shields.io/crates/v/step-io.svg)](https://crates.io/crates/step-io)
[![docs.rs](https://img.shields.io/docsrs/step-io)](https://docs.rs/step-io)
[![CI](https://github.com/elgar328/step-io/actions/workflows/ci.yml/badge.svg)](https://github.com/elgar328/step-io/actions/workflows/ci.yml)
[![license](https://img.shields.io/crates/l/step-io.svg)](#license)

Lean STEP (ISO 10303) I/O for 3D CAD kernels — reads every mainstream AP,
writes only modern AP242.

> ⚠️ **Experimental** — early stage; expect breaking API changes at any time.

## Features

- **Reads everything, writes AP242** — any mainstream AP comes in, legacy
  included; only AP242 edition 2 (IS) goes out.
- **Lean** — one universal schema model instead of a per-schema
  implementation; entities that never appear in real STEP files are pruned
  out.
- **Heals non-standard input** — what can be fixed is normalized; what
  cannot is dropped with a reason, never silently.
- **Ergonomic API** — `scene()` navigates what you read with lightweight
  handles; `StepBuilder` writes new files the way kernels hold data.

## Quickstart

Reading STEP into a kernel or viewer:

```rust,no_run
let source = std::fs::read("model.step").unwrap();
let (model, report) = step_io::read(&source).unwrap();

for solid in model.scene().all_solids() {
    for face in solid.faces() {
        let surface = face.surface().kind(); // Plane, Cylindrical, …
        for bound in face.bounds() {
            for (edge, forward) in bound.oriented_edges() {
                let curve = edge.curve().kind(); // Line, Circle, …
                let ends = (edge.start(), edge.end()); // bounding vertices
            }
        }
    }
}
```

Writing STEP from a kernel:

```rust,no_run
let mut b = step_io::StepBuilder::new().unwrap();
let part = b.part("wheel").unwrap();

// mirror your kernel's arrays, keeping one handle per element
let vs: Vec<_> = kernel.points.iter()
    .map(|p| b.vertex(*p).unwrap())
    .collect();
let es: Vec<_> = kernel.edges.iter()
    .map(|e| b.edge(vs[e.start], vs[e.end], curve_of(e)).unwrap())
    .collect();
let fs: Vec<_> = kernel.faces.iter()
    .map(|f| b.face(surface_of(f), f.same_sense, loops_of(f, &es)).unwrap())
    .collect();
b.solid(part, "wheel body", fs).unwrap();

std::fs::write("wheel.step", b.finish().unwrap()).unwrap();
```

## Design

step-io exists to sit between a 3D CAD kernel and the STEP files it
exchanges.

Most STEP libraries carry per-schema entity and read/write code, so each
supported schema adds source and binary size. step-io instead merges the
mainstream APs into one *universal* schema and reads them all through a
single pipeline. Entities that never appear in practice are left out, judged
against 50,000+ real-world files — the [entity coverage
matrix](docs/entities.md) lists exactly what is read and written.

Output is AP242 edition 2 (IS) only — AP203 and AP214 were merged into
it in 2014, and every modern tool reads it. When edition 3 reaches IS
and takes over, the output target moves up with it: one output schema,
always the current one.

Most of the pipeline is generated from the schemas rather than written
by hand — no drift, little to maintain.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE)
at your option.
