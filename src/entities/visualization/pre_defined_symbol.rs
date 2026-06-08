//! `PRE_DEFINED_SYMBOL` handler. Abstract supertype of
//! `PRE_DEFINED_TERMINATOR_SYMBOL`; corpus 0 in observed AP242 files.
//! Reader pushes a `PreDefinedSymbol::Plain(...)` variant into
//! `VisualizationPool::pre_defined_symbols` and records the
//! `PreDefinedSymbolId` in `viz_pre_defined_symbol_id_map`.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{PreDefinedSymbol, PreDefinedSymbolData, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct PreDefinedSymbolHandler;

#[step_entity(name = "PRE_DEFINED_SYMBOL")]
impl SimpleEntityHandler for PreDefinedSymbolHandler {
    type WriteInput = PreDefinedSymbolData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "PRE_DEFINED_SYMBOL")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .pre_defined_symbols
            .push(PreDefinedSymbol::Plain(PreDefinedSymbolData { name }));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, s: PreDefinedSymbolData) -> Result<u64, WriteError> {
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "PRE_DEFINED_SYMBOL".into(),
                attrs: vec![Attribute::String(s.name)],
            },
        });
        Ok(n)
    }
}
