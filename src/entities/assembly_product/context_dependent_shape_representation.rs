//! `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION` handler.
//!
//! Binds each NAUO to a `Transform3d` by walking the RR-complex sub-entity
//! that the CDSR's first attribute references. Reader body needs `&graph`
//! to resolve the complex parts (`REPRESENTATION_RELATIONSHIP` +
//! `REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION` +
//! `SHAPE_REPRESENTATION_RELATIONSHIP`). Writer emits the two-attr form:
//! `CDSR(rr_complex_ref, pdef_shape_ref)`.

use crate::early::model::EarlyTransformation;
use crate::early::{bind, lift, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

#[derive(Clone, Copy)]
pub(crate) struct ContextDependentShapeRepresentationWriteInput {
    pub(crate) rrwt: u64,
    pub(crate) nauo_pds: u64,
}

pub(crate) struct ContextDependentShapeRepresentationHandler;

#[step_entity(name = "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION")]
impl SimpleEntityHandler for ContextDependentShapeRepresentationHandler {
    type WriteInput = ContextDependentShapeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_context_dependent_shape_representation(entity_id, attrs)?;
        let rr_ref = early.representation_relation;
        let pdef_shape_ref = early.represented_product_relation;

        // Only NAUO-tagged CDSRs — product-level CDSRs skip silently.
        let Some(nauo_ref) = ctx.nauo_pds_info.get(&pdef_shape_ref).map(|i| i.nauo) else {
            return Ok(());
        };

        // Read the RRWT complex through the L1 facade (it folds the Complex
        // guard + has_all_parts + strict bind). Outer `None` = not the RRWT
        // triple; `?` propagates a bind defect; inner `None` = unrecognized
        // transformation SELECT member → skip.
        let early = crate::early::EarlyGraph::new(graph);
        let Some(rrwt_res) = early.representation_relationship_with_transformation(rr_ref) else {
            return Ok(());
        };
        let Some(rrwt) = rrwt_res? else {
            return Ok(());
        };
        let transform_ref = match rrwt.transformation_operator {
            EarlyTransformation::EntityRef(n) => n,
            // SET-of-IDT form is not modelled (corpus 0) — skip the transform.
            EarlyTransformation::SetItemDefinedTransformation(_) => return Ok(()),
        };
        let Some(&transform) = ctx.transform_map.get(&transform_ref) else {
            return Err(ConvertError::MissingReference {
                from: rr_ref,
                to: transform_ref,
                field_name: "transform_operator",
            });
        };
        ctx.nauo_transform_map.insert(nauo_ref, transform);

        // Stash the base REPRESENTATION_RELATIONSHIP payload so the placement
        // can be materialised into the `representation_relationships` arena
        // (blueprint-faithful identity for `style_context` to reference). The
        // arena push happens in `resolve_nauo_instances` in canonical order so
        // the resulting id is round-trip stable. If either rep is not a
        // modelled `Representation`, skip materialisation (the transform is
        // still recorded above; `style_context` then drops with a warning).
        let name = rrwt.name;
        let description = rrwt.description.unwrap_or_default();
        let rep_1_ref = rrwt.rep_1;
        let rep_2_ref = rrwt.rep_2;
        if let (Some(rep_1), Some(rep_2)) = (
            ctx.id_cache
                .get::<crate::ir::id::RepresentationId>(rep_1_ref),
            ctx.id_cache
                .get::<crate::ir::id::RepresentationId>(rep_2_ref),
        ) {
            ctx.nauo_assembly_rr.insert(
                nauo_ref,
                crate::reader::AssemblyRrData {
                    name,
                    description,
                    rep_1,
                    rep_2,
                    rr_complex_entity: rr_ref,
                },
            );
        }
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ContextDependentShapeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_context_dependent_shape_representation(input);
        Ok(serialize::serialize_context_dependent_shape_representation(
            buf, &early,
        ))
    }
}
