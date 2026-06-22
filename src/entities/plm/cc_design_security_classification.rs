//! `CC_DESIGN_SECURITY_CLASSIFICATION` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::{CcDesignSecurityClassification, SecurityClassificationItem};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct CcDesignSecurityClassificationHandler;

#[step_entity(name = "CC_DESIGN_SECURITY_CLASSIFICATION")]
impl SimpleEntityHandler for CcDesignSecurityClassificationHandler {
    type WriteInput = CcDesignSecurityClassification;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_cc_design_security_classification(entity_id, attrs)?;
        lower::lower_cc_design_security_classification(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: CcDesignSecurityClassification) -> Result<u64, WriteError> {
        let sc_step = buf.step_id(c.assigned_security_classification);
        let items: Vec<u64> = c
            .items
            .into_iter()
            .map(|item| match item {
                SecurityClassificationItem::Product(pid) => buf.product_def_ids[&pid],
            })
            .collect();
        let early = lift::lift_cc_design_security_classification(sc_step, items);
        Ok(serialize::serialize_cc_design_security_classification(
            buf, &early,
        ))
    }
}
