//! `SYMBOL_COLOUR` handler — phase symbol-colour.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{SymbolColour, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SymbolColourHandler;

#[step_entity(name = "SYMBOL_COLOUR")]
impl SimpleEntityHandler for SymbolColourHandler {
    type WriteInput = SymbolColour;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "SYMBOL_COLOUR")?;
        let colour_ref = read_entity_ref(attrs, 0, entity_id, "colour_of_symbol")?;
        let Some(&colour_of_symbol) = ctx.viz_colour_id_map.get(&colour_ref) else {
            return Ok(());
        };
        let viz = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = viz.symbol_colours.push(SymbolColour { colour_of_symbol });
        ctx.symbol_colour_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, sc: SymbolColour) -> Result<u64, WriteError> {
        let colour_step = buf.step_id(sc.colour_of_symbol);
        Ok(buf.push_simple("SYMBOL_COLOUR", vec![Attribute::EntityRef(colour_step)]))
    }
}
