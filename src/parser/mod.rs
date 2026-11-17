//! [`parse`] and [`stream`] — STEP input to raw, untyped entities.
//!
//! This is the syntax layer: it knows Part 21, not the schemas. Each entity
//! is a [`RawEntity`] (keyword plus attributes), and the source AP,
//! identified from the `FILE_SCHEMA` header, rides along as a [`SchemaId`].
//! [`stream`] reads a file one entity at a time, in memory that does not grow
//! with the file (apart from the warnings it collects); [`parse`] collects
//! every entity into a [`Graph`] keyed by id, for looking entities up and
//! following references between them.
//! [`read`](crate::read) takes its entities from the same [`Stream`]; this
//! stays public for tools that want the raw Part 21 file without the schema
//! layer — statistics, linting, anonymization, and the like.

pub(crate) mod entity;
pub(crate) mod lexer;
pub(crate) mod p21;
pub(crate) mod schema;
pub(crate) mod string;

pub use entity::{Attribute, Graph, RawEntity, RawEntityPart};
pub use lexer::Span;
pub use p21::{MAX_NESTING_DEPTH, Stream, parse, stream};

// Public at the crate root; here for the crate's own paths.
pub(crate) use entity::{Error, ParseWarning};
pub(crate) use schema::SchemaId;
