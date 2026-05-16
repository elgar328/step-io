//! `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION` handler — Pass 6-7.
//!
//! Binds each NAUO to a `Transform3d` by walking the RR-complex sub-entity
//! that the CDSR's first attribute references. Reader body needs `&graph`
//! to resolve the complex parts (`REPRESENTATION_RELATIONSHIP` +
//! `REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION` +
//! `SHAPE_REPRESENTATION_RELATIONSHIP`). Writer emits the two-attr form:
//! `CDSR(rr_complex_ref, pdef_shape_ref)`.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph, RawEntity};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct ContextDependentShapeRepresentationWriteInput {
    pub(crate) rrwt: u64,
    pub(crate) nauo_pds: u64,
}

pub(crate) struct ContextDependentShapeRepresentationHandler;

impl SimpleEntityHandler for ContextDependentShapeRepresentationHandler {
    const NAME: &'static str = "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6Cdsr;
    type WriteInput = ContextDependentShapeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            2,
            entity_id,
            "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION",
        )?;
        let rr_ref = read_entity_ref(attrs, 0, entity_id, "representation_relation")?;
        let pdef_shape_ref = read_entity_ref(attrs, 1, entity_id, "represented_product_relation")?;

        // Only NAUO-tagged CDSRs — product-level CDSRs skip silently.
        let Some(&nauo_ref) = ctx.pdef_shape_to_nauo.get(&pdef_shape_ref) else {
            return Ok(());
        };

        // Look up the RR complex. Must carry all three part types.
        let Some(RawEntity::Complex { parts, .. }) = graph.get(rr_ref) else {
            return Ok(());
        };
        if !crate::reader::has_all_parts(
            parts,
            &[
                "REPRESENTATION_RELATIONSHIP",
                "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
                "SHAPE_REPRESENTATION_RELATIONSHIP",
            ],
        ) {
            return Ok(());
        }
        let rrwt_attrs = crate::reader::require_part_attrs(
            parts,
            "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
            rr_ref,
        )?;
        let transform_ref = read_entity_ref(rrwt_attrs, 0, rr_ref, "transform_operator")?;
        let Some(&transform) = ctx.transform_map.get(&transform_ref) else {
            return Err(ConvertError::MissingReference {
                from: rr_ref,
                to: transform_ref,
                field_name: "transform_operator",
            });
        };
        ctx.nauo_transform_map.insert(nauo_ref, transform);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        ContextDependentShapeRepresentationWriteInput { rrwt, nauo_pds }: ContextDependentShapeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION",
            vec![Attribute::EntityRef(rrwt), Attribute::EntityRef(nauo_pds)],
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static CDSR_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: ContextDependentShapeRepresentationHandler::NAME,
    pass_level: ContextDependentShapeRepresentationHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: ContextDependentShapeRepresentationHandler::read,
    },
};
