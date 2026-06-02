//! `SHAPE_REPRESENTATION_RELATIONSHIP` handler.
//!
//! Reader records SR-to-SR equivalences for the Fusion 360 / CATIA
//! `SDR → plain SR → SRR → ABSR/MSSR` indirection. When one side already
//! resolves to a known geometry-carrying rep, the other (typically a plain
//! SR) is recorded as equivalent. Multi-hop chains and complex
//! `_WITH_TRANSFORMATION` variants belong to the CDSR / RRWT path.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{RepresentationRelationship, ShapeRepresentationRelationshipIr};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "SHAPE_REPRESENTATION_RELATIONSHIP")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let rep_1 = read_entity_ref(attrs, 2, entity_id, "rep_1")?;
        let rep_2 = read_entity_ref(attrs, 3, entity_id, "rep_2")?;

        // Keep the SDR indirection cache populated so the Fusion 360 /
        // CATIA `SDR → plain SR → SRR → ABSR/MSSR` chain still resolves
        // through `srr_equiv_map`. Arena push runs alongside so all SRRs
        // (1-to-many fan-out, multi-hop, ABSR↔MSSR direct) round-trip.
        let r1_target = ctx.absr_solid_map.contains_key(&rep_1)
            || ctx.mssr_shells_map.contains_key(&rep_1)
            || ctx.wireframe_data_map.contains_key(&rep_1);
        let r2_target = ctx.absr_solid_map.contains_key(&rep_2)
            || ctx.mssr_shells_map.contains_key(&rep_2)
            || ctx.wireframe_data_map.contains_key(&rep_2);
        match (r1_target, r2_target) {
            (true, false) => {
                ctx.srr_equiv_map.insert(rep_2, rep_1);
            }
            (false, true) => {
                ctx.srr_equiv_map.insert(rep_1, rep_2);
            }
            _ => {}
        }

        let (Some(&r1_id), Some(&r2_id)) =
            (ctx.repr_id_map.get(&rep_1), ctx.repr_id_map.get(&rep_2))
        else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "SHAPE_REPRESENTATION_RELATIONSHIP #{entity_id} references unmodelled representation(s) (#{rep_1} or #{rep_2})"
                ),
            });
            return Ok(());
        };
        ctx.representation_relationships.push(
            RepresentationRelationship::ShapeRepresentationRelationship(
                ShapeRepresentationRelationshipIr {
                    name,
                    description,
                    rep_1: r1_id,
                    rep_2: r2_id,
                },
            ),
        );
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ShapeRepresentationRelationshipWriteInput { rep_1, rep_2 }: ShapeRepresentationRelationshipWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "SHAPE_REPRESENTATION_RELATIONSHIP",
            vec![
                Attribute::String("SRR".into()),
                Attribute::String("None".into()),
                Attribute::EntityRef(rep_1),
                Attribute::EntityRef(rep_2),
            ],
        ))
    }
}
