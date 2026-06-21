//! Part 21 serializer.
//!
//! Serialization is a 2-pass pipeline:
//!
//! 1. `WriteBuffer` walks [`StepModel`](crate::ir::StepModel) and assembles a
//!    `Vec<WriterEntity>` with contiguous `#N` ids (see `buffer.rs`).
//! 2. `write_file` streams the HEADER + DATA sections through a
//!    `std::io::Write` target (see `serialize.rs`).
//!
//! The public entry points are the three `write_*` methods on
//! [`StepModel`](crate::ir::StepModel).

pub(crate) mod buffer;
pub(crate) mod entity;
mod header;
mod lexical;
mod project;
mod serialize;

/// Errors that the writer can emit.
#[derive(Debug)]
pub enum WriteError {
    /// The IR contains a variant the writer does not yet serialize.
    UnsupportedIrVariant { detail: String },
    /// The IR references an id that does not resolve inside the model.
    DanglingId { detail: String },
    /// A real attribute carried a non-finite value. Part 21 admits only
    /// finite reals.
    InvalidFloat { value: f64, context: &'static str },
    /// An underlying I/O error from the `Write` target.
    Io(std::io::Error),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedIrVariant { detail } => {
                write!(f, "write error: unsupported IR variant ({detail})")
            }
            Self::DanglingId { detail } => {
                write!(f, "write error: dangling id ({detail})")
            }
            Self::InvalidFloat { value, context } => {
                write!(f, "write error: non-finite real {value} in {context}")
            }
            Self::Io(e) => write!(f, "write error: io ({e})"),
        }
    }
}

impl std::error::Error for WriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for WriteError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Output schema target for the write API.
///
/// `Universal` emits the model as-is against step-io's internal union schema
/// (the superset of every supported AP) — no projection. The union is a
/// non-standard step-io superset, not an ISO schema, which is exactly why it is
/// never a silent default. The `Ap*` variants project the output to one curated
/// IS edition, dropping entities (and retargeting the `FILE_SCHEMA` / `APD` header)
/// that the edition cannot express.
///
/// There is **no default**: callers always choose. A non-standard superset must
/// not be a silent default, and a real AP cannot be the default without breaking
/// round-trip (it would silently retarget every input).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaTarget {
    /// Internal union (superset of all supported APs); emit as-is, no projection.
    Universal,
    /// Project to AP214 edition 3 (IS) — `AUTOMOTIVE_DESIGN`.
    Ap214,
    /// Project to AP242 edition 2 (IS) — `AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF`.
    Ap242,
    /// Project to AP203 edition 2 (IS) — modular `AP203_..._MIM_LF`.
    Ap203,
}

impl crate::ir::StepModel {
    /// Stream Part 21 text to any `std::io::Write` target.
    ///
    /// # Errors
    /// Returns [`WriteError::UnsupportedIrVariant`] / [`WriteError::DanglingId`]
    /// / [`WriteError::InvalidFloat`] for IR problems, or [`WriteError::Io`]
    /// if the underlying writer fails.
    pub fn write_to<W: std::io::Write>(
        &self,
        mut writer: W,
        target: SchemaTarget,
    ) -> Result<(), WriteError> {
        let mut buffer = buffer::WriteBuffer::new(self, target);
        buffer.emit_all()?;
        // Build the P21 edition 3 ANCHOR / REFERENCE section lines from the
        // model + the ids reserved in `emit_all` (grabbed before
        // `finish_entities` consumes the buffer).
        let ed3 = serialize::Ed3Sections::build(self, &buffer.step_ids);
        let mut entities = buffer.finish_entities();
        // Schema projection: drop target-illegal entities + cascade (no-op for
        // Universal). LossReport is computed but not yet surfaced on this path.
        let profile = crate::early::profile::SchemaProfile::for_target(target);
        let _loss = project::project(&mut entities, &profile);
        let headers = header::header_for(self, target);
        serialize::write_file(&mut writer, &headers, &ed3, &entities)
    }

    /// Serialize Part 21 text to an owned `String`.
    ///
    /// # Errors
    /// Same IR-level errors as [`write_to`](Self::write_to); I/O is
    /// in-memory so [`WriteError::Io`] never occurs on this path.
    ///
    /// # Panics
    /// Panics only if the writer produced non-UTF-8 bytes — impossible
    /// unless this crate has an internal bug, since every emission path
    /// stays within the ASCII range.
    pub fn write_to_string(&self, target: SchemaTarget) -> Result<String, WriteError> {
        let mut buf = Vec::new();
        self.write_to(&mut buf, target)?;
        Ok(String::from_utf8(buf).expect("writer emits valid UTF-8"))
    }

    /// Serialize Part 21 text to the given file path, buffered.
    ///
    /// Any existing file at `path` is truncated. A `BufWriter` wraps the
    /// file and is explicitly flushed so that flush errors propagate
    /// instead of being swallowed by `Drop`.
    ///
    /// # Errors
    /// Same IR-level errors as [`write_to`](Self::write_to), plus
    /// [`WriteError::Io`] if file creation, writing, or the final flush
    /// fails.
    pub fn write_to_file<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        target: SchemaTarget,
    ) -> Result<(), WriteError> {
        use std::io::Write as _;
        let file = std::fs::File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);
        self.write_to(&mut writer, target)?;
        writer.flush()?;
        Ok(())
    }
}
