//! `PRE_DEFINED_TERMINATOR_SYMBOL` handler. Stock terminator
//! symbol referenced from leader / dimension annotation chains (arrow
//! heads, slashes, ...). Reader pushes a
//! `PreDefinedSymbol::Terminator(...)` variant into
//! `VisualizationPool::pre_defined_symbols` and records the
//! `PreDefinedSymbolId` in `viz_pre_defined_symbol_id_map`.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{PreDefinedSymbol, PreDefinedTerminatorSymbol, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct PreDefinedTerminatorSymbolHandler;

#[step_entity(name = "PRE_DEFINED_TERMINATOR_SYMBOL")]
impl SimpleEntityHandler for PreDefinedTerminatorSymbolHandler {
    type WriteInput = PreDefinedTerminatorSymbol;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "PRE_DEFINED_TERMINATOR_SYMBOL")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool.pre_defined_symbols.push(PreDefinedSymbol::Terminator(
            PreDefinedTerminatorSymbol { name },
        ));
        ctx.viz_pre_defined_symbol_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, s: PreDefinedTerminatorSymbol) -> Result<u64, WriteError> {
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "PRE_DEFINED_TERMINATOR_SYMBOL".into(),
                attrs: vec![Attribute::String(s.name)],
            },
        });
        Ok(n)
    }
}
