//! `DRAUGHTING_PRE_DEFINED_CURVE_FONT` handler (shares the
//! leaf pass with `COLOUR_RGB` and the new `PRE_DEFINED_CURVE_FONT` self
//! variant). Pre-defined curve fonts name a stock line pattern
//! (`"continuous"`, `"dashed"`, etc.). Reader pushes a
//! `PreDefinedCurveFont::Draughting(...)` variant into
//! `VisualizationPool::pre_defined_curve_fonts` and records the
//! `PreDefinedCurveFontId` in `viz_pre_defined_curve_font_id_map` so the
//! `CURVE_STYLE` reader can resolve a font ref to an arena index.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    DraughtingPreDefinedCurveFont, PreDefinedCurveFont, VisualizationPool,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct DraughtingPreDefinedCurveFontHandler;

#[step_entity(name = "DRAUGHTING_PRE_DEFINED_CURVE_FONT")]
impl SimpleEntityHandler for DraughtingPreDefinedCurveFontHandler {
    type WriteInput = DraughtingPreDefinedCurveFont;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "DRAUGHTING_PRE_DEFINED_CURVE_FONT")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .pre_defined_curve_fonts
            .push(PreDefinedCurveFont::Draughting(
                DraughtingPreDefinedCurveFont { name },
            ));
        ctx.viz_pre_defined_curve_font_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, f: DraughtingPreDefinedCurveFont) -> Result<u64, WriteError> {
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "DRAUGHTING_PRE_DEFINED_CURVE_FONT".into(),
                attrs: vec![Attribute::String(f.name)],
            },
        });
        Ok(n)
    }
}
