//! `AREA_IN_SET` + `PRESENTATION_SIZE` handlers — phase pr-size.
//!
//! `area_in_set` binds a `PresentationArea` representation to a
//! `PresentationSet`; `presentation_size` carries a 2D extent box paired
//! with a `presentation_size_assignment_select` (view / area /
//! `area_in_set`). Unresolved refs drop the carrier on read.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::visualization::{AreaInSet, PresentationSize};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AreaInSetHandler;

#[step_entity(name = "AREA_IN_SET")]
impl SimpleEntityHandler for AreaInSetHandler {
    type WriteInput = AreaInSet;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_area_in_set(entity_id, attrs)?;
        lower::lower_area_in_set(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ais: AreaInSet) -> Result<u64, WriteError> {
        Ok(serialize::serialize_area_in_set(
            buf,
            &lift::lift_area_in_set(buf, ais),
        ))
    }
}

pub(crate) struct PresentationSizeHandler;

#[step_entity(name = "PRESENTATION_SIZE")]
impl SimpleEntityHandler for PresentationSizeHandler {
    type WriteInput = PresentationSize;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_presentation_size(entity_id, attrs)?;
        lower::lower_presentation_size(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ps: PresentationSize) -> Result<u64, WriteError> {
        let early = lift::lift_presentation_size(buf, ps)?;
        Ok(serialize::serialize_presentation_size(buf, &early))
    }
}
