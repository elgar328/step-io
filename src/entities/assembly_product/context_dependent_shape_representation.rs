//! `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION` handler.
//!
//! Binds each NAUO to a `Transform3d` by walking the RR-complex sub-entity
//! that the CDSR's first attribute references. Reader body needs `&graph`
//! to resolve the complex parts (`REPRESENTATION_RELATIONSHIP` +
//! `REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION` +
//! `SHAPE_REPRESENTATION_RELATIONSHIP`). Writer emits the two-attr form:
//! `CDSR(rr_complex_ref, pdef_shape_ref)`.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph, RawEntity};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

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

        // Stash the base REPRESENTATION_RELATIONSHIP payload so the placement
        // can be materialised into the `representation_relationships` arena
        // (blueprint-faithful identity for `style_context` to reference). The
        // arena push happens in `resolve_nauo_instances` in canonical order so
        // the resulting id is round-trip stable. If either rep is not a
        // modelled `Representation`, skip materialisation (the transform is
        // still recorded above; `style_context` then drops with a warning).
        let rr_attrs =
            crate::reader::require_part_attrs(parts, "REPRESENTATION_RELATIONSHIP", rr_ref)?;
        let name = read_string_or_unset(rr_attrs, 0, rr_ref, "name")?.to_owned();
        let description = read_string_or_unset(rr_attrs, 1, rr_ref, "description")?.to_owned();
        let rep_1_ref = read_entity_ref(rr_attrs, 2, rr_ref, "rep_1")?;
        let rep_2_ref = read_entity_ref(rr_attrs, 3, rr_ref, "rep_2")?;
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
        ContextDependentShapeRepresentationWriteInput { rrwt, nauo_pds }: ContextDependentShapeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION",
            vec![Attribute::EntityRef(rrwt), Attribute::EntityRef(nauo_pds)],
        ))
    }
}
