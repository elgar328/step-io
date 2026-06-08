//! `REPRESENTATION_RELATIONSHIP` handler — base `representation_relationship`
//! `Itself` carrier variant.
//!
//! The unconstrained supertype `(name, description, rep_1, rep_2)`. NIST AP203
//! PMI files use it standalone to relate two `DRAUGHTING_MODEL`s. Same shape as
//! the subtypes (CGRR / MDDR / SRR) — `rep_1`/`rep_2` resolved through
//! `repr_id_map`; EXPRESS narrowing WHERE rules are not enforced.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{RepresentationRelationship, RepresentationRelationshipData};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "REPRESENTATION_RELATIONSHIP")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let rep_1_ref = read_entity_ref(attrs, 2, entity_id, "rep_1")?;
        let rep_2_ref = read_entity_ref(attrs, 3, entity_id, "rep_2")?;
        let Some(rep_1) = ctx
            .id_cache
            .get::<crate::ir::id::RepresentationId>(rep_1_ref)
        else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "REPRESENTATION_RELATIONSHIP #{entity_id}.rep_1 #{rep_1_ref} did not \
                     resolve to a modelled REPRESENTATION subtype — skipping"
                ),
            });
            return Ok(());
        };
        let Some(rep_2) = ctx
            .id_cache
            .get::<crate::ir::id::RepresentationId>(rep_2_ref)
        else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "REPRESENTATION_RELATIONSHIP #{entity_id}.rep_2 #{rep_2_ref} did not \
                     resolve to a modelled REPRESENTATION subtype — skipping"
                ),
            });
            return Ok(());
        };
        ctx.representation_relationships
            .push(RepresentationRelationship::Itself(
                RepresentationRelationshipData {
                    name,
                    description,
                    rep_1,
                    rep_2,
                },
            ));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, rr: RepresentationRelationshipData) -> Result<u64, WriteError> {
        let rep_1_step = buf.step_id(rr.rep_1);
        let rep_2_step = buf.step_id(rr.rep_2);
        Ok(buf.push_simple(
            "REPRESENTATION_RELATIONSHIP",
            vec![
                Attribute::String(rr.name),
                Attribute::String(rr.description),
                Attribute::EntityRef(rep_1_step),
                Attribute::EntityRef(rep_2_step),
            ],
        ))
    }
}
