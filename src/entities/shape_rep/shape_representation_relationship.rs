//! `SHAPE_REPRESENTATION_RELATIONSHIP` handler.
//!
//! Reader records SR-to-SR equivalences for the Fusion 360 / CATIA
//! `SDR → plain SR → SRR → ABSR/MSSR` indirection (see `lower`). When one
//! side already resolves to a known geometry-carrying rep, the other
//! (typically a plain SR) is recorded as equivalent. Multi-hop chains and
//! complex `_WITH_TRANSFORMATION` variants belong to the CDSR / RRWT path.
//!
//! Writer emits the legacy synthesised form (`'SRR'` / `'None'` labels).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

#[derive(Clone, Copy)]
pub(crate) struct ShapeRepresentationRelationshipWriteInput {
    pub(crate) rep_1: u64,
    pub(crate) rep_2: u64,
}

pub(crate) struct ShapeRepresentationRelationshipHandler;

#[step_entity(name = "SHAPE_REPRESENTATION_RELATIONSHIP")]
impl SimpleEntityHandler for ShapeRepresentationRelationshipHandler {
    type WriteInput = ShapeRepresentationRelationshipWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_shape_representation_relationship(entity_id, attrs)?;
        lower::lower_shape_representation_relationship(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ShapeRepresentationRelationshipWriteInput { rep_1, rep_2 }: ShapeRepresentationRelationshipWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_shape_representation_relationship(rep_1, rep_2);
        Ok(serialize::serialize_shape_representation_relationship(
            buf, &early,
        ))
    }
}
