use crate::parser::entity::Attribute;

/// A DATA-section entity awaiting serialization. Always carries an `#N` id.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::writer) struct WriterEntity {
    pub id: u64,
    pub body: WriterBody,
}

/// Body of a [`WriterEntity`] — either a simple entity or a complex entity
/// composed of multiple sub-parts.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::writer) enum WriterBody {
    /// Emits as `#N = NAME(attrs);`.
    Simple { name: String, attrs: Vec<Attribute> },
    /// Emits as `#N = ( NAME1(attrs1) NAME2(attrs2) ... );`.
    /// Used for unit contexts, rational B-splines, RRWT, etc.
    Complex {
        parts: Vec<(String, Vec<Attribute>)>,
    },
}

/// A HEADER-section entity. Unlike [`WriterEntity`], HEADER entities have no
/// `#N` identifier and emit as `NAME(attrs);`.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::writer) struct HeaderEntity {
    pub name: String,
    pub attrs: Vec<Attribute>,
}
