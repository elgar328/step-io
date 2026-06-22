//! `MAKE_FROM_USAGE_OPTION` handler — assembly-product domain (2-layer path).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::MakeFromUsageOption;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct MakeFromUsageOptionWriteInput {
    pub(crate) mfu: MakeFromUsageOption,
    pub(crate) relating_pdef_step: u64,
    pub(crate) related_pdef_step: u64,
    pub(crate) quantity_step: u64,
}

pub(crate) struct MakeFromUsageOptionHandler;

#[step_entity(name = "MAKE_FROM_USAGE_OPTION")]
impl SimpleEntityHandler for MakeFromUsageOptionHandler {
    type WriteInput = MakeFromUsageOptionWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_make_from_usage_option(entity_id, attrs)?;
        lower::lower_make_from_usage_option(ctx, entity_id, early)
    }

    fn write(
        buf: &mut WriteBuffer,
        MakeFromUsageOptionWriteInput {
            mfu,
            relating_pdef_step,
            related_pdef_step,
            quantity_step,
        }: MakeFromUsageOptionWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_make_from_usage_option(
            mfu,
            relating_pdef_step,
            related_pdef_step,
            quantity_step,
        );
        Ok(serialize::serialize_make_from_usage_option(buf, &early))
    }
}
