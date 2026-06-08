//! `MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP` handler — phase mddr.
//!
//! `representation_relationship` SUBTYPE pairing two
//! representations (`DM` | `MDGPR` | `SR` per `mddr_select`). Same structure
//! as CGRR — 4 attrs, `rep_1`/`rep_2` resolved through `repr_id_map`.
//! EXPRESS WHERE rules (rep type combination constraints) are not
//! enforced — source input is trusted.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{MechanicalDesignAndDraughtingRelationship, RepresentationRelationship};
use crate::parser::entity::{Attribute, EntityGraph};
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            4,
            entity_id,
            "MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP",
        )?;
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
                    "MDDR #{entity_id}.rep_1 #{rep_1_ref} did not resolve to a \
                     modelled REPRESENTATION subtype — skipping"
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
                    "MDDR #{entity_id}.rep_2 #{rep_2_ref} did not resolve to a \
                     modelled REPRESENTATION subtype — skipping"
                ),
            });
            return Ok(());
        };
        ctx.representation_relationships.push(
            RepresentationRelationship::MechanicalDesignAndDraughtingRelationship(
                MechanicalDesignAndDraughtingRelationship {
                    name,
                    description,
                    rep_1,
                    rep_2,
                },
            ),
        );
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        mddr: MechanicalDesignAndDraughtingRelationship,
    ) -> Result<u64, WriteError> {
        let rep_1_step = buf.step_id(mddr.rep_1);
        let rep_2_step = buf.step_id(mddr.rep_2);
        Ok(buf.push_simple(
            "MECHANICAL_DESIGN_AND_DRAUGHTING_RELATIONSHIP",
            vec![
                Attribute::String(mddr.name),
                Attribute::String(mddr.description),
                Attribute::EntityRef(rep_1_step),
                Attribute::EntityRef(rep_2_step),
            ],
        ))
    }
}
