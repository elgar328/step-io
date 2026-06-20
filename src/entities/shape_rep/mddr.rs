//! `MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP` handler — phase mddr.
//!
//! `representation_relationship` SUBTYPE pairing two
//! representations (`DM` | `MDGPR` | `SR` per `mddr_select`). Same structure
//! as CGRR — 4 attrs, `rep_1`/`rep_2` resolved through `repr_id_map`.
//! EXPRESS WHERE rules (rep type combination constraints) are not
//! enforced — source input is trusted.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::MechanicalDesignAndDraughtingRelationship;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct MechanicalDesignAndDraughtingRelationshipHandler;

#[step_entity(name = "MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP")]
impl SimpleEntityHandler for MechanicalDesignAndDraughtingRelationshipHandler {
    type WriteInput = MechanicalDesignAndDraughtingRelationship;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_mechanical_design_and_draughting_relationship(entity_id, attrs)?;
        lower::lower_mechanical_design_and_draughting_relationship(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        mddr: MechanicalDesignAndDraughtingRelationship,
    ) -> Result<u64, WriteError> {
        // The arena holds L2 ids; resolve to output step ids here (lift is a
        // pure adapter and never touches the buffer).
        let rep_1_step = buf.step_id(mddr.rep_1);
        let rep_2_step = buf.step_id(mddr.rep_2);
        let early = lift::lift_mechanical_design_and_draughting_relationship(
            mddr.name,
            mddr.description,
            rep_1_step,
            rep_2_step,
        );
        Ok(serialize::serialize_mechanical_design_and_draughting_relationship(buf, &early))
    }
}
