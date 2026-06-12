//! `CONSTRUCTIVE_GEOMETRY_REPRESENTATION_RELATIONSHIP` handler — phase cgrr.
//!
//! `representation_relationship` SUBTYPE that pairs an `rep_1`
//! (`CGR | SR` SELECT) with an `rep_2` (`CGR` ref). step-io stores both
//! as plain `RepresentationId` and trusts the source's `repr_id_map`
//! resolution — unresolved refs drop the carrier (symmetric on re-read).
//!
//! Orphan entity (no inbound refs); pushed into the new
//! `representation_relationships` arena. Writer emits the arena from
//! `emit_representation_relationships` after every Representation slot
//! of `representation_step_ids` is populated.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::ConstructiveGeometryRepresentationRelationship;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ConstructiveGeometryRepresentationRelationshipHandler;

#[step_entity(name = "CONSTRUCTIVE_GEOMETRY_REPRESENTATION_RELATIONSHIP")]
impl SimpleEntityHandler for ConstructiveGeometryRepresentationRelationshipHandler {
    type WriteInput = ConstructiveGeometryRepresentationRelationship;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_constructive_geometry_representation_relationship(entity_id, attrs)?;
        lower::lower_constructive_geometry_representation_relationship(ctx, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        cgrr: ConstructiveGeometryRepresentationRelationship,
    ) -> Result<u64, WriteError> {
        // The arena holds L2 ids; resolve to output step ids here (lift is a
        // pure adapter and never touches the buffer).
        let rep_1_step = buf.step_id(cgrr.rep_1);
        let rep_2_step = buf.step_id(cgrr.rep_2);
        let early = lift::lift_constructive_geometry_representation_relationship(
            cgrr.name,
            cgrr.description,
            rep_1_step,
            rep_2_step,
        );
        Ok(serialize::serialize_constructive_geometry_representation_relationship(buf, &early))
    }
}
