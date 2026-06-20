//! `PRESENTATION_VIEW` / `PRESENTATION_AREA` / `PRESENTATION_SET` —
//! phase pr-core.
//!
//! All three are `representation_item` subtype simple instances. View and
//! Area share the same arena (`PresentationRepresentation` enum) and
//! resolve `items` via the generic helper used by SDR / `DraughtingModel`;
//! Set is a minimal `name`-only carrier required by `AREA_IN_SET.in_set`.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{PresentationReprData, PresentationSet};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PresentationViewHandler;

#[step_entity(name = "PRESENTATION_VIEW")]
impl SimpleEntityHandler for PresentationViewHandler {
    type WriteInput = PresentationReprData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_presentation_view(entity_id, attrs)?;
        lower::lower_presentation_view(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: PresentationReprData) -> Result<u64, WriteError> {
        let early = lift::lift_presentation_view(buf, data)?;
        Ok(serialize::serialize_presentation_view(buf, &early))
    }
}

pub(crate) struct PresentationAreaHandler;

#[step_entity(name = "PRESENTATION_AREA")]
impl SimpleEntityHandler for PresentationAreaHandler {
    type WriteInput = PresentationReprData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_presentation_area(entity_id, attrs)?;
        lower::lower_presentation_area(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: PresentationReprData) -> Result<u64, WriteError> {
        let early = lift::lift_presentation_area(buf, data)?;
        Ok(serialize::serialize_presentation_area(buf, &early))
    }
}

pub(crate) struct PresentationSetHandler;

#[step_entity(name = "PRESENTATION_SET")]
impl SimpleEntityHandler for PresentationSetHandler {
    type WriteInput = PresentationSet;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_presentation_set(entity_id, attrs)?;
        lower::lower_presentation_set(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, _set: PresentationSet) -> Result<u64, WriteError> {
        Ok(serialize::serialize_presentation_set(
            buf,
            &lift::lift_presentation_set(),
        ))
    }
}
