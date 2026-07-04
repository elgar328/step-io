//! [`parse_bytes`] — STEP source to the raw, untyped entity graph.
//!
//! This is the syntax layer: it knows Part 21, not the schemas. [`tokenize`]
//! turns the text into tokens; [`parse`] and [`parse_bytes`] assemble them
//! into a [`Graph`], which holds each entity as a [`RawEntity`] (keyword
//! plus attributes). The source AP, identified from the `FILE_SCHEMA`
//! header, rides along as a [`SchemaId`]. [`read`](crate::read) drives this
//! as its first step; everything here stays public for tools that want the
//! raw Part 21 file without the schema layer — statistics, linting,
//! anonymization, and the like.

pub mod entity;
pub mod lexer;
pub mod p21;
pub mod schema;

pub use entity::{Attribute, Error, Graph, ParseWarning, RawEntity, RawEntityPart};
pub use lexer::{LexError, LexErrorKind, Lexer, Span, Token, TokenKind, tokenize};
pub use p21::{Parser, parse, parse_bytes};
pub use schema::{ApFamily, NonEmptyStringList, SchemaId, Stage};
