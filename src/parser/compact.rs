//! Lossless in-memory storage for a raw graph, with one record decoded on demand.
//!
//! Entity ids index offsets in bounded byte blocks. Repeated entity/typed names
//! and enumeration labels share a symbol table; arbitrary strings remain exact
//! UTF-8 values. Real and integer payloads retain their original bits. Source
//! spans and all graph metadata survive. This representation does not persist
//! a new file format and does not filter, normalize or approximate geometry.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::io::Read;

use super::entity::{Attribute, Error, ParseWarning, RawEntity, RawEntityPart};
use super::lexer::Span;
use super::p21::Parser;
use super::schema::SchemaId;

const BLOCK: usize = 1024 * 1024;

/// The raw graph with packed, lossless records instead of an owned attribute
/// tree for every entity. [`Self::entity`] reconstructs one original record.
///
/// The index and symbol table remain in memory. Record bytes use blocks of at
/// most 1 MiB, avoiding whole-file buffer reallocation. This is in-memory
/// storage only: its private encoding is not a stable serialization contract.
#[non_exhaustive]
pub struct CompactGraph {
    pub schema: SchemaId,
    pub header: Vec<RawEntity>,
    pub external_references: BTreeMap<u64, String>,
    pub anchors: Vec<(String, u64)>,
    pub warnings: Vec<ParseWarning>,
    records: BTreeMap<u64, usize>,
    symbols: Vec<String>,
    data: Blocks,
}

impl fmt::Debug for CompactGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompactGraph")
            .field("entities", &self.len())
            .field("record_bytes", &self.stored_bytes())
            .field("symbols", &self.symbol_count())
            .finish_non_exhaustive()
    }
}

impl CompactGraph {
    fn reader(&self, id: u64) -> Option<RecordReader<'_>> {
        Some(RecordReader {
            data: &self.data,
            position: *self.records.get(&id)?,
            symbols: &self.symbols,
        })
    }

    /// The original simple entity keyword, or `COMPLEX` for a complex record.
    /// This query does not decode or allocate any attribute payload.
    #[must_use]
    pub fn kind(&self, id: u64) -> Option<&str> {
        let mut reader = self.reader(id)?;
        reader.span();
        Some(match reader.byte() {
            0 => reader.symbol_ref(),
            1 => "COMPLEX",
            _ => unreachable!("private record tag"),
        })
    }

    /// Whether the original record is complex, without decoding its attributes.
    /// Unlike [`Self::kind`], this distinguishes a simple keyword named `COMPLEX`.
    #[must_use]
    pub fn is_complex(&self, id: u64) -> Option<bool> {
        let mut reader = self.reader(id)?;
        reader.span();
        Some(match reader.byte() {
            0 => false,
            1 => true,
            _ => unreachable!("private record tag"),
        })
    }

    /// Decode just one attribute, optionally inside a named complex part.
    /// `path[0]` selects the top-level attribute; subsequent indices select
    /// nested list items. Missing records, parts, indices and non-list steps
    /// return `None`. Skipped strings and lists are not allocated.
    #[must_use]
    pub fn attribute_at(&self, id: u64, part: Option<&str>, path: &[usize]) -> Option<Attribute> {
        let mut reader = self.reader(id)?;
        reader.span();
        match reader.byte() {
            0 if part.is_none() => {
                reader.unsigned();
                reader.select(path)
            }
            1 if part.is_some() => {
                for _ in 0..reader.length() {
                    let name = reader.symbol_ref();
                    if Some(name) == part {
                        return reader.select(path);
                    }
                    reader.skip_attributes();
                }
                None
            }
            0 | 1 => None,
            _ => unreachable!("private record tag"),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Original file ids, in ascending order, including unsupported entities.
    pub fn ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.records.keys().copied()
    }

    /// Bytes used by packed records, excluding the id index and symbol table.
    #[must_use]
    pub fn stored_bytes(&self) -> usize {
        self.data.len
    }

    #[must_use]
    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    /// Reconstruct the complete raw record, including its original span.
    /// The returned record owns its parameters; it does not borrow the graph.
    #[must_use]
    pub fn entity(&self, id: u64) -> Option<RawEntity> {
        let mut reader = RecordReader {
            data: &self.data,
            position: *self.records.get(&id)?,
            symbols: &self.symbols,
        };
        let span = reader.span();
        Some(match reader.byte() {
            0 => RawEntity::Simple {
                id,
                span,
                name: reader.symbol(),
                attributes: reader.attributes(),
            },
            1 => RawEntity::Complex {
                id,
                span,
                parts: (0..reader.length())
                    .map(|_| RawEntityPart {
                        name: reader.symbol(),
                        attributes: reader.attributes(),
                    })
                    .collect(),
            },
            _ => unreachable!("private record tag"),
        })
    }
}

/// Parse the same Part 21 syntax as [`super::parse`] into lossless packed
/// storage. No record is discarded, including unrecognized entity types.
///
/// # Errors
/// Returns the same syntax, I/O and first-duplicate-id errors as [`super::parse`].
pub fn parse_compact(input: impl Read) -> Result<CompactGraph, Error> {
    let mut parser = Parser::new(input);
    let prelude = parser.read_prelude()?;
    let mut graph = CompactGraph {
        schema: prelude.schema,
        header: prelude.header,
        external_references: prelude.external_references,
        anchors: prelude.anchors,
        warnings: Vec::new(),
        records: BTreeMap::new(),
        symbols: Vec::new(),
        data: Blocks::default(),
    };
    let mut symbol_ids = HashMap::new();
    while let Some(entity) = parser.next_entity()? {
        let id = entity.id();
        if let Some(&offset) = graph.records.get(&id) {
            let mut reader = RecordReader {
                data: &graph.data,
                position: offset,
                symbols: &graph.symbols,
            };
            return Err(Error::DuplicateEntityId {
                id,
                line: entity.span().line,
                first_line: reader.span().line,
            });
        }
        graph.records.insert(id, graph.data.len);
        let mut writer = RecordWriter {
            data: &mut graph.data,
            symbols: &mut graph.symbols,
            symbol_ids: &mut symbol_ids,
        };
        writer.span(entity.span());
        match &entity {
            RawEntity::Simple {
                name, attributes, ..
            } => {
                writer.byte(0);
                writer.symbol(name);
                writer.attributes(attributes);
            }
            RawEntity::Complex { parts, .. } => {
                writer.byte(1);
                writer.length(parts.len());
                for part in parts {
                    writer.symbol(&part.name);
                    writer.attributes(&part.attributes);
                }
            }
        }
    }
    graph.warnings = parser.finish()?;
    Ok(graph)
}

#[derive(Default)]
struct Blocks {
    chunks: Vec<Vec<u8>>,
    len: usize,
}

impl Blocks {
    fn write(&mut self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            if self.chunks.last().is_none_or(|last| last.len() == BLOCK) {
                self.chunks.push(Vec::with_capacity(BLOCK));
            }
            let chunk = self.chunks.last_mut().expect("block allocated");
            let count = bytes.len().min(BLOCK - chunk.len());
            chunk.extend_from_slice(&bytes[..count]);
            bytes = &bytes[count..];
            self.len += count;
        }
    }

    fn byte(&self, position: usize) -> u8 {
        self.chunks[position / BLOCK][position % BLOCK]
    }

    fn read(&self, position: &mut usize, bytes: &mut [u8]) {
        let mut out = bytes;
        while !out.is_empty() {
            let chunk = &self.chunks[*position / BLOCK];
            let start = *position % BLOCK;
            let count = out.len().min(chunk.len() - start);
            out[..count].copy_from_slice(&chunk[start..start + count]);
            *position += count;
            out = &mut out[count..];
        }
    }
}

struct RecordWriter<'a> {
    data: &'a mut Blocks,
    symbols: &'a mut Vec<String>,
    symbol_ids: &'a mut HashMap<String, usize>,
}

impl RecordWriter<'_> {
    fn byte(&mut self, value: u8) {
        self.data.write(&[value]);
    }

    fn unsigned(&mut self, mut value: u64) {
        while value >= 128 {
            self.byte(u8::try_from(value & 127).expect("seven bits") | 128);
            value >>= 7;
        }
        self.byte(u8::try_from(value).expect("seven bits"));
    }

    fn length(&mut self, value: usize) {
        self.unsigned(u64::try_from(value).expect("addressable length"));
    }

    fn symbol(&mut self, value: &str) {
        let id = if let Some(&id) = self.symbol_ids.get(value) {
            id
        } else {
            let id = self.symbols.len();
            self.symbols.push(value.to_owned());
            self.symbol_ids.insert(value.to_owned(), id);
            id
        };
        self.length(id);
    }

    fn text(&mut self, value: &str) {
        self.length(value.len());
        self.data.write(value.as_bytes());
    }

    fn span(&mut self, span: Span) {
        self.unsigned(span.start);
        self.unsigned(span.end);
        self.unsigned(u64::from(span.line));
        self.unsigned(u64::from(span.column));
    }

    fn attributes(&mut self, values: &[Attribute]) {
        self.length(values.len());
        for value in values {
            self.attribute(value);
        }
    }

    fn attribute(&mut self, value: &Attribute) {
        match value {
            Attribute::Integer(value) => {
                self.byte(0);
                self.data.write(&value.to_le_bytes());
            }
            Attribute::Real(value) => {
                self.byte(1);
                self.data.write(&value.to_bits().to_le_bytes());
            }
            Attribute::String(value) => {
                self.byte(2);
                self.text(value);
            }
            Attribute::Enum(value) => {
                self.byte(3);
                self.symbol(value);
            }
            Attribute::Binary(value) => {
                self.byte(4);
                self.text(value);
            }
            Attribute::EntityRef(value) => {
                self.byte(5);
                self.unsigned(*value);
            }
            Attribute::Unset => self.byte(6),
            Attribute::Derived => self.byte(7),
            Attribute::List(values) => {
                self.byte(8);
                self.attributes(values);
            }
            Attribute::Typed { type_name, value } => {
                self.byte(9);
                self.symbol(type_name);
                self.attribute(value);
            }
        }
    }
}

struct RecordReader<'a> {
    data: &'a Blocks,
    position: usize,
    symbols: &'a [String],
}

impl<'a> RecordReader<'a> {
    fn byte(&mut self) -> u8 {
        let value = self.data.byte(self.position);
        self.position += 1;
        value
    }

    fn unsigned(&mut self) -> u64 {
        let mut value = 0;
        for shift in (0..70).step_by(7) {
            let byte = self.byte();
            value |= u64::from(byte & 127) << shift;
            if byte < 128 {
                return value;
            }
        }
        unreachable!("private varint written from u64")
    }

    fn length(&mut self) -> usize {
        usize::try_from(self.unsigned()).expect("addressable stored length")
    }

    fn symbol(&mut self) -> String {
        self.symbol_ref().to_owned()
    }

    fn symbol_ref(&mut self) -> &'a str {
        let id = self.length();
        &self.symbols[id]
    }

    fn select(&mut self, path: &[usize]) -> Option<Attribute> {
        let (&index, rest) = path.split_first()?;
        if index >= self.length() {
            return None;
        }
        for _ in 0..index {
            self.skip_attribute();
        }
        if rest.is_empty() {
            Some(self.attribute())
        } else if self.byte() == 8 {
            self.select(rest)
        } else {
            None
        }
    }

    fn skip_attributes(&mut self) {
        for _ in 0..self.length() {
            self.skip_attribute();
        }
    }

    fn skip_attribute(&mut self) {
        match self.byte() {
            0 | 1 => self.position += 8,
            2 | 4 => self.position += self.length(),
            3 | 5 => {
                self.unsigned();
            }
            6 | 7 => {}
            8 => self.skip_attributes(),
            9 => {
                self.unsigned();
                self.skip_attribute();
            }
            _ => unreachable!("private attribute tag"),
        }
    }

    fn text(&mut self) -> String {
        let mut bytes = vec![0; self.length()];
        self.data.read(&mut self.position, &mut bytes);
        String::from_utf8(bytes).expect("stored UTF-8")
    }

    fn span(&mut self) -> Span {
        Span {
            start: self.unsigned(),
            end: self.unsigned(),
            line: u32::try_from(self.unsigned()).expect("stored line"),
            column: u32::try_from(self.unsigned()).expect("stored column"),
        }
    }

    fn attributes(&mut self) -> Vec<Attribute> {
        (0..self.length()).map(|_| self.attribute()).collect()
    }

    fn attribute(&mut self) -> Attribute {
        match self.byte() {
            0 => {
                let mut bytes = [0; 8];
                self.data.read(&mut self.position, &mut bytes);
                Attribute::Integer(i64::from_le_bytes(bytes))
            }
            1 => {
                let mut bytes = [0; 8];
                self.data.read(&mut self.position, &mut bytes);
                Attribute::Real(f64::from_bits(u64::from_le_bytes(bytes)))
            }
            2 => Attribute::String(self.text()),
            3 => Attribute::Enum(self.symbol()),
            4 => Attribute::Binary(self.text()),
            5 => Attribute::EntityRef(self.unsigned()),
            6 => Attribute::Unset,
            7 => Attribute::Derived,
            8 => Attribute::List(self.attributes()),
            9 => Attribute::Typed {
                type_name: self.symbol(),
                value: Box::new(self.attribute()),
            },
            _ => unreachable!("private attribute tag"),
        }
    }
}
