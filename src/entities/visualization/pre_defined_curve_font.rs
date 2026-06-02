//! `PRE_DEFINED_CURVE_FONT` handler — Pass 7-1. The abstract supertype of
//! `DRAUGHTING_PRE_DEFINED_CURVE_FONT`; corpus 0 in observed AP242 files,
//! but the entity is still legal as a standalone instance per the
//! schema. Reader pushes a `PreDefinedCurveFont::Plain(...)` variant into
//! `VisualizationPool::pre_defined_curve_fonts` and records the
//! `PreDefinedCurveFontId` in `viz_pre_defined_curve_font_id_map` so the
//! `CURVE_STYLE` reader can resolve a font ref pointing at the bare
//! supertype.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{PreDefinedCurveFont, PreDefinedCurveFontData, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct PreDefinedCurveFontHandler;

#[step_entity(name = "PRE_DEFINED_CURVE_FONT")]
impl SimpleEntityHandler for PreDefinedCurveFontHandler {
    type WriteInput = PreDefinedCurveFontData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "PRE_DEFINED_CURVE_FONT")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .pre_defined_curve_fonts
            .push(PreDefinedCurveFont::Plain(PreDefinedCurveFontData { name }));
        ctx.viz_pre_defined_curve_font_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, f: PreDefinedCurveFontData) -> Result<u64, WriteError> {
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "PRE_DEFINED_CURVE_FONT".into(),
                attrs: vec![Attribute::String(f.name)],
            },
        });
        Ok(n)
    }
}
