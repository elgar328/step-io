//! `PRE_DEFINED_MARKER` handler — phase pre-defined-marker.
//!
//! Simple-instance variant (`PreDefinedMarker::Plain`). The `Plain`
//! variant covers direct `PRE_DEFINED_MARKER` instances; complex-MI
//! `PRE_DEFINED_POINT_MARKER_SYMBOL` (SUBTYPE OF (`pre_defined_marker`,
//! `pre_defined_symbol`)) is deferred to a future sub-phase.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    PreDefinedMarker, PreDefinedMarkerData, PreDefinedPointMarkerSymbol, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PreDefinedMarkerHandler;

#[step_entity(name = "PRE_DEFINED_MARKER", pass = Pass7Colour)]
impl SimpleEntityHandler for PreDefinedMarkerHandler {
    type WriteInput = PreDefinedMarker;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "PRE_DEFINED_MARKER")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let viz = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = viz
            .pre_defined_markers
            .push(PreDefinedMarker::Plain(PreDefinedMarkerData { name }));
        ctx.viz_pre_defined_marker_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, m: PreDefinedMarker) -> Result<u64, WriteError> {
        let name = match m {
            PreDefinedMarker::Plain(d) => d.name,
            // PointMarkerSymbol — reserved for future sub-phase. arena never
            // contains this variant in this phase (no handler pushes it).
            PreDefinedMarker::PointMarkerSymbol(PreDefinedPointMarkerSymbol { name }) => name,
        };
        Ok(buf.push_simple("PRE_DEFINED_MARKER", vec![Attribute::String(name)]))
    }
}
