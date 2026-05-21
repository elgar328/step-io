//! `SHAPE_ASPECT_RELATIONSHIP` handler — Pass 8-pre-b.
//!
//! A directed relation between two shape aspects. Both endpoints are
//! `shape_aspect`-typed (an abstract supertype), resolved here through
//! [`resolve_shape_aspect_ref`] into a [`ShapeAspectRef`] — the unified
//! reference enum this phase introduces. An endpoint that does not resolve
//! (a `shape_aspect` subtype step-io does not model yet) drops the whole
//! relationship, symmetric on re-read.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_aspect_ref::ShapeAspectRef;
use crate::ir::shape_rep::{ShapeAspectRelationship, ShapeAspectRelationshipKind};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// Resolve a STEP `shape_aspect` reference into a [`ShapeAspectRef`] by
/// probing each shape-aspect-family id map. Returns `None` for a target
/// step-io does not model (e.g. `DATUM_FEATURE`, `COMPOSITE_SHAPE_ASPECT`).
pub(crate) fn resolve_shape_aspect_ref(
    ctx: &ReaderContext,
    item_ref: u64,
) -> Option<ShapeAspectRef> {
    if let Some(&id) = ctx.shape_aspect_id_map.get(&item_ref) {
        return Some(ShapeAspectRef::ShapeAspect(id));
    }
    if let Some(&id) = ctx.composite_shape_aspect_id_map.get(&item_ref) {
        return Some(ShapeAspectRef::CompositeGroupShapeAspect(id));
    }
    if let Some(&id) = ctx.centre_of_symmetry_id_map.get(&item_ref) {
        return Some(ShapeAspectRef::CentreOfSymmetry(id));
    }
    if let Some(&id) = ctx.all_around_shape_aspect_id_map.get(&item_ref) {
        return Some(ShapeAspectRef::AllAroundShapeAspect(id));
    }
    None
}

pub(crate) struct ShapeAspectRelationshipHandler;

#[step_entity(name = "SHAPE_ASPECT_RELATIONSHIP", pass = Pass8ShapeAspectRel)]
impl SimpleEntityHandler for ShapeAspectRelationshipHandler {
    type WriteInput = ShapeAspectRelationship;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "SHAPE_ASPECT_RELATIONSHIP")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let relating_ref = read_entity_ref(attrs, 2, entity_id, "relating_shape_aspect")?;
        let related_ref = read_entity_ref(attrs, 3, entity_id, "related_shape_aspect")?;

        let Some(relating_shape_aspect) = resolve_shape_aspect_ref(ctx, relating_ref) else {
            return Ok(()); // endpoint unresolved — drop the relationship
        };
        let Some(related_shape_aspect) = resolve_shape_aspect_ref(ctx, related_ref) else {
            return Ok(());
        };

        ctx.shape_aspect_relationships
            .push(ShapeAspectRelationship {
                name,
                description,
                relating_shape_aspect,
                related_shape_aspect,
                kind: ShapeAspectRelationshipKind::Plain,
            });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, rel: ShapeAspectRelationship) -> Result<u64, WriteError> {
        let relating = buf.emit_shape_aspect_ref(rel.relating_shape_aspect);
        let related = buf.emit_shape_aspect_ref(rel.related_shape_aspect);
        let name = match rel.kind {
            ShapeAspectRelationshipKind::Plain => "SHAPE_ASPECT_RELATIONSHIP",
        };
        Ok(buf.push_simple(
            name,
            vec![
                Attribute::String(rel.name),
                Attribute::String(rel.description),
                Attribute::EntityRef(relating),
                Attribute::EntityRef(related),
            ],
        ))
    }
}
