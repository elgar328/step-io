//! `SHAPE_ASPECT_RELATIONSHIP` family handlers.
//!
//! A directed relation between two shape aspects. Both endpoints are
//! `shape_aspect`-typed (an abstract supertype), resolved here through
//! [`resolve_shape_aspect_ref`] into a [`ShapeAspectRef`]. An endpoint that
//! does not resolve (a `shape_aspect` subtype step-io does not model yet)
//! drops the whole relationship, symmetric on re-read.
//!
//! `shape_aspect_relationship` is a `concrete_supertype`: the plain entity
//! and its subtypes `SHAPE_ASPECT_ASSOCIATIVITY` /
//! `SHAPE_ASPECT_DERIVING_RELATIONSHIP` share the identical 4-attr shape and
//! differ only by STEP entity name. One arena covers the family — the name
//! is captured in [`ShapeAspectRelationshipKind`].

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
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
/// step-io does not model (e.g. `DATUM_SYSTEM`, `DATUM_TARGET`).
pub(crate) fn resolve_shape_aspect_ref(
    ctx: &ReaderContext,
    item_ref: u64,
) -> Option<ShapeAspectRef> {
    // Members + probe order are generated from the enum by `StepSelect`.
    ShapeAspectRef::resolve_select(ctx, item_ref)
}

/// Emit a `ShapeAspectRelationship` under the STEP entity name its `kind`
/// selects, returning the STEP id. Shared by all four family handlers (the
/// emit loop calls only this one). 2-layer write: resolve both endpoints, then
/// dispatch `kind` → the matching `lift_*` + `serialize_*` (each generated
/// serialize hardcodes its entity name).
fn write_shape_aspect_relationship(buf: &mut WriteBuffer, rel: ShapeAspectRelationship) -> u64 {
    let relating = buf.emit_shape_aspect_ref(rel.relating_shape_aspect);
    let related = buf.emit_shape_aspect_ref(rel.related_shape_aspect);
    match rel.kind {
        ShapeAspectRelationshipKind::Plain => serialize::serialize_shape_aspect_relationship(
            buf,
            &lift::lift_shape_aspect_relationship(rel.name, rel.description, relating, related),
        ),
        ShapeAspectRelationshipKind::Associativity => {
            serialize::serialize_shape_aspect_associativity(
                buf,
                &lift::lift_shape_aspect_associativity(
                    rel.name,
                    rel.description,
                    relating,
                    related,
                ),
            )
        }
        ShapeAspectRelationshipKind::DerivingRelationship => {
            serialize::serialize_shape_aspect_deriving_relationship(
                buf,
                &lift::lift_shape_aspect_deriving_relationship(
                    rel.name,
                    rel.description,
                    relating,
                    related,
                ),
            )
        }
        ShapeAspectRelationshipKind::FeatureForDatumTarget => {
            serialize::serialize_feature_for_datum_target_relationship(
                buf,
                &lift::lift_feature_for_datum_target_relationship(
                    rel.name,
                    rel.description,
                    relating,
                    related,
                ),
            )
        }
    }
}

pub(crate) struct ShapeAspectRelationshipHandler;

#[step_entity(name = "SHAPE_ASPECT_RELATIONSHIP")]
impl SimpleEntityHandler for ShapeAspectRelationshipHandler {
    type WriteInput = ShapeAspectRelationship;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_shape_aspect_relationship(entity_id, attrs)?;
        lower::lower_shape_aspect_relationship(
            ctx,
            early.name,
            early.description,
            early.relating_shape_aspect,
            early.related_shape_aspect,
            ShapeAspectRelationshipKind::Plain,
        );
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, rel: ShapeAspectRelationship) -> Result<u64, WriteError> {
        Ok(write_shape_aspect_relationship(buf, rel))
    }
}

pub(crate) struct ShapeAspectAssociativityHandler;

#[step_entity(name = "SHAPE_ASPECT_ASSOCIATIVITY")]
impl SimpleEntityHandler for ShapeAspectAssociativityHandler {
    type WriteInput = ShapeAspectRelationship;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_shape_aspect_associativity(entity_id, attrs)?;
        lower::lower_shape_aspect_relationship(
            ctx,
            early.name,
            early.description,
            early.relating_shape_aspect,
            early.related_shape_aspect,
            ShapeAspectRelationshipKind::Associativity,
        );
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, rel: ShapeAspectRelationship) -> Result<u64, WriteError> {
        Ok(write_shape_aspect_relationship(buf, rel))
    }
}

pub(crate) struct ShapeAspectDerivingRelationshipHandler;

#[step_entity(name = "SHAPE_ASPECT_DERIVING_RELATIONSHIP")]
impl SimpleEntityHandler for ShapeAspectDerivingRelationshipHandler {
    type WriteInput = ShapeAspectRelationship;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_shape_aspect_deriving_relationship(entity_id, attrs)?;
        lower::lower_shape_aspect_relationship(
            ctx,
            early.name,
            early.description,
            early.relating_shape_aspect,
            early.related_shape_aspect,
            ShapeAspectRelationshipKind::DerivingRelationship,
        );
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, rel: ShapeAspectRelationship) -> Result<u64, WriteError> {
        Ok(write_shape_aspect_relationship(buf, rel))
    }
}

pub(crate) struct FeatureForDatumTargetRelationshipHandler;

#[step_entity(name = "FEATURE_FOR_DATUM_TARGET_RELATIONSHIP")]
impl SimpleEntityHandler for FeatureForDatumTargetRelationshipHandler {
    type WriteInput = ShapeAspectRelationship;

    /// Mirrors the plain relationship reader but tags the entry with the
    /// `FeatureForDatumTarget` kind so the writer round-trips the right
    /// STEP entity name. The blueprint narrows `related_shape_aspect` to
    /// `ref_datum_target`, but step-io stores it as the unified
    /// [`ShapeAspectRef`]; the writer guards the variant before emitting.
    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_feature_for_datum_target_relationship(entity_id, attrs)?;
        lower::lower_shape_aspect_relationship(
            ctx,
            early.name,
            early.description,
            early.relating_shape_aspect,
            early.related_shape_aspect,
            ShapeAspectRelationshipKind::FeatureForDatumTarget,
        );
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, rel: ShapeAspectRelationship) -> Result<u64, WriteError> {
        Ok(write_shape_aspect_relationship(buf, rel))
    }
}
