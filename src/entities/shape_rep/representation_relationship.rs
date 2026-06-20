//! `REPRESENTATION_RELATIONSHIP` handler — base `representation_relationship`
//! `Itself` carrier variant.
//!
//! The unconstrained supertype `(name, description, rep_1, rep_2)`. NIST AP203
//! PMI files use it standalone to relate two `DRAUGHTING_MODEL`s. Same shape as
//! the subtypes (CGRR / MDDR / SRR) — `rep_1`/`rep_2` resolved through the
//! `RepresentationId` cache; EXPRESS narrowing WHERE rules are not enforced.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::RepresentationRelationshipData;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct RepresentationRelationshipHandler;

#[step_entity(name = "REPRESENTATION_RELATIONSHIP")]
impl SimpleEntityHandler for RepresentationRelationshipHandler {
    type WriteInput = RepresentationRelationshipData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_representation_relationship(entity_id, attrs)?;
        lower::lower_representation_relationship(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, rr: RepresentationRelationshipData) -> Result<u64, WriteError> {
        // The arena holds L2 ids; resolve to output step ids here (lift is a
        // pure adapter and never touches the buffer).
        let rep_1_step = buf.step_id(rr.rep_1);
        let rep_2_step = buf.step_id(rr.rep_2);
        let early =
            lift::lift_representation_relationship(rr.name, rr.description, rep_1_step, rep_2_step);
        Ok(serialize::serialize_representation_relationship(
            buf, &early,
        ))
    }
}
