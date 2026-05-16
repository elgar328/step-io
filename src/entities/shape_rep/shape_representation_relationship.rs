//! `SHAPE_REPRESENTATION_RELATIONSHIP` handler — Pass 6-4b.
//!
//! Reader records SR-to-SR equivalences for the Fusion 360 / CATIA
//! `SDR → plain SR → SRR → ABSR/MSSR` indirection. When one side already
//! resolves to a known geometry-carrying rep, the other (typically a plain
//! SR) is recorded as equivalent. Multi-hop chains and complex
//! `_WITH_TRANSFORMATION` variants belong to the CDSR / RRWT path.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct ShapeRepresentationRelationshipWriteInput {
    pub(crate) rep_1: u64,
    pub(crate) rep_2: u64,
}

pub(crate) struct ShapeRepresentationRelationshipHandler;

impl SimpleEntityHandler for ShapeRepresentationRelationshipHandler {
    const NAME: &'static str = "SHAPE_REPRESENTATION_RELATIONSHIP";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6SrRel;
    type WriteInput = ShapeRepresentationRelationshipWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "SHAPE_REPRESENTATION_RELATIONSHIP")?;
        // attrs[0] = name, attrs[1] = description — ignored.
        let rep_1 = read_entity_ref(attrs, 2, entity_id, "rep_1")?;
        let rep_2 = read_entity_ref(attrs, 3, entity_id, "rep_2")?;

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
            (true, true) => {
                // Both sides are geometry-carrying reps (e.g. ABSR↔MSSR
                // direct relation). SDR can reach either directly — no
                // indirection needed. Silently skip.
            }
            (false, false) => {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: format!(
                        "SHAPE_REPRESENTATION_RELATIONSHIP #{entity_id} connects two non-target representations — multi-hop SRR chains are not supported"
                    ),
                });
            }
        }
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static SR_REL_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ShapeRepresentationRelationshipHandler::NAME,
    pass_level: ShapeRepresentationRelationshipHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ShapeRepresentationRelationshipHandler::read,
    },
};
