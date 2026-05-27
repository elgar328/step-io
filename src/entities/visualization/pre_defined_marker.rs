//! `PRE_DEFINED_MARKER` + `PRE_DEFINED_POINT_MARKER_SYMBOL` handlers.
//!
//! The base `PRE_DEFINED_MARKER` covers direct simple instances
//! (`Plain` variant). The `PRE_DEFINED_POINT_MARKER_SYMBOL` subtype
//! (corpus 6) is technically `SUBTYPE OF (pre_defined_marker,
//! pre_defined_symbol)`, but every observed corpus instance appears as
//! a simplified P21 form `PRE_DEFINED_POINT_MARKER_SYMBOL('x');` rather
//! than a complex MI — so a normal `SimpleEntityHandler` suffices.
//! Both push into the shared `pre_defined_marker` arena; the writer
//! dispatches on the enum variant to pick the correct STEP type name.

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
    type WriteInput = PreDefinedMarkerData;

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

    fn write(buf: &mut WriteBuffer, d: PreDefinedMarkerData) -> Result<u64, WriteError> {
        Ok(buf.push_simple("PRE_DEFINED_MARKER", vec![Attribute::String(d.name)]))
    }
}

pub(crate) struct PreDefinedPointMarkerSymbolHandler;

#[step_entity(name = "PRE_DEFINED_POINT_MARKER_SYMBOL", pass = Pass7Colour)]
impl SimpleEntityHandler for PreDefinedPointMarkerSymbolHandler {
    type WriteInput = PreDefinedPointMarkerSymbol;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "PRE_DEFINED_POINT_MARKER_SYMBOL")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let viz = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = viz
            .pre_defined_markers
            .push(PreDefinedMarker::PointMarkerSymbol(
                PreDefinedPointMarkerSymbol { name },
            ));
        ctx.viz_pre_defined_marker_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, p: PreDefinedPointMarkerSymbol) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PRE_DEFINED_POINT_MARKER_SYMBOL",
            vec![Attribute::String(p.name)],
        ))
    }
}
