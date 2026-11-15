# Lossless packed raw graphs

`parser::parse_compact` is an optional representation of the same Part 21 syntax
as `parser::parse`. It preserves every raw entity, including unsupported entity
types, header records, schema, warnings, anchors, external references and source
spans. `entity(id)` reconstructs an owned `RawEntity`; ids iterate in ascending
order. Callers that edit or retain decoded records can keep using `Graph`.

```rust
use step_io::parser::{Attribute, parse_compact};

# fn inspect(input: impl std::io::Read) -> Result<(), step_io::Error> {
let graph = parse_compact(input)?;
for id in graph.ids() {
    if graph.kind(id) == Some("CARTESIAN_POINT") {
        let coordinates = graph.attribute_at(id, None, &[1]);
        if let Some(Attribute::List(coordinates)) = coordinates {
            println!("#{id}: {coordinates:?}");
        }
    }
}
# Ok(())
# }
```

`is_complex` reads the record shape independently of its keyword: a simple,
unsupported entity named `COMPLEX` is still simple. `attribute_at` takes an
optional complex-part name and a path of list indices. It returns `None` for a
missing record, part or index, an empty path, or a non-list intermediate value.
Skipped strings, typed values and lists are traversed without allocating them.
The selected value is owned, so selecting an entire large list still allocates
that list. No decoded-record cache grows with traversal.

## Storage and correctness

An ordered id index addresses append-only record bytes in blocks of at most
1 MiB. Repeated keywords, typed names and enumeration labels share a symbol
table. Integers retain all 64 bits and real values retain their bit patterns,
including negative zero. Strings and binary values retain their decoded values;
references, nesting, complex-part order and source spans survive reconstruction.
Record boundaries may cross blocks. The private encoding is an in-memory detail,
not a persistence format or a stable serialization contract.

Both graph parsers retain records directly in their final index instead of first
growing a whole-file `Vec<RawEntity>`. They report the first repeated id in file
order, with the first and repeated source lines. A repeat already encountered
takes precedence over a later syntax or input error, as the original parser did.
The typed `read` model and its normalization policy are unchanged.

`stored_bytes` counts packed record payload, excluding the id index, block
capacity, symbol table and graph metadata. It is not the graph's heap size or a
process-memory measurement. Dense ids are not assumed, and index storage still
grows with the number of entities. Decoding one record can temporarily require
that record's full attribute tree.

## Reproducible memory experiment

`examples/graph_memory.rs` generates a stream of synthetic Cartesian points. It
does not contain or derive geometry from an external STEP file. Each mode keeps
one graph, hashes every reconstructed entity, and prints its entity count and
same-toolchain content hash. Compare the hashes and measure each process
separately with an operating-system memory tool:

```sh
cargo build --release --example graph_memory
/usr/bin/time -l target/release/examples/graph_memory raw 1000000
/usr/bin/time -l target/release/examples/graph_memory compact 1000000
```

This isolates raw-graph storage. It does not measure typed-model construction,
B-rep import, feature recognition or a caller's retained input buffer. Peak
resident memory from this command must not be mixed with private committed
memory measured on another operating system. The hash is an experiment aid,
not a cryptographic proof or a stable interchange identifier.
