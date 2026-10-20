//! [`parse`] — STEP input to the raw, untyped entity graph.
//!
//! This is the syntax layer: it knows Part 21, not the schemas. [`parse`]
//! reads a file into a [`Graph`], which holds each entity as a
//! [`RawEntity`] (keyword plus attributes). The source AP, identified from
//! the `FILE_SCHEMA` header, rides along as a [`SchemaId`].
//! [`read`](crate::read) drives the same parser as its first step; this
//! stays public for tools that want the raw Part 21 file without the schema
//! layer — statistics, linting, anonymization, and the like.

pub(crate) mod entity;
pub(crate) mod lexer;
pub(crate) mod p21;
pub(crate) mod schema;
pub(crate) mod string;

pub use entity::{Attribute, Graph, RawEntity, RawEntityPart};
pub use lexer::{Span, TokenKind};
pub use p21::{MAX_NESTING_DEPTH, parse};
pub use schema::NonEmptyStringList;

// Public at the crate root; here for the crate's own paths.
pub(crate) use entity::{Error, ParseWarning};
pub(crate) use schema::SchemaId;
