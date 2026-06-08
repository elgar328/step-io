//! `SYMBOL_STYLE` handler — phase symbol-style.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{FoundedItem, SymbolStyle, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SymbolStyleHandler;

#[step_entity(name = "SYMBOL_STYLE")]
impl SimpleEntityHandler for SymbolStyleHandler {
    type WriteInput = SymbolStyle;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "SYMBOL_STYLE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let colour_ref = read_entity_ref(attrs, 1, entity_id, "style_of_symbol")?;
        let Some(&style_of_symbol) = ctx.symbol_colour_id_map.get(&colour_ref) else {
            return Ok(());
        };
        let viz = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        viz.founded_items
            .push(FoundedItem::SymbolStyle(SymbolStyle {
                name,
                style_of_symbol,
            }));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ss: SymbolStyle) -> Result<u64, WriteError> {
        let colour_step = buf.symbol_colour_step_ids[ss.style_of_symbol.0 as usize];
        Ok(buf.push_simple(
            "SYMBOL_STYLE",
            vec![
                Attribute::String(ss.name),
                Attribute::EntityRef(colour_step),
            ],
        ))
    }
}
