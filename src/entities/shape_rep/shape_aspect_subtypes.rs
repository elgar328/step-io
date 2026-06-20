//! `SHAPE_ASPECT` subtype handlers.
//!
//! `COMPOSITE_GROUP_SHAPE_ASPECT` / `CENTRE_OF_SYMMETRY` /
//! `ALL_AROUND_SHAPE_ASPECT` share `SHAPE_ASPECT`'s 4-attr shape
//! (name, description, `of_shape`, `product_definitional`). The ir.toml
//! blueprint gives each its own arena, so each round-trips under its own
//! STEP entity name. The reader/writer bodies are shared here — only the
//! entity name and target arena differ per handler.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::CompositeShapeAspectKind;
use crate::parser::entity::{Attribute, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::{step_entity, step_entity_complex};

/// Resolved write input shared by all three subtype handlers.
pub(crate) struct ShapeAspectSubtypeWriteInput {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) pds_step_id: u64,
    pub(crate) product_definitional: bool,
}

pub(crate) struct CompositeGroupShapeAspectHandler;

#[step_entity(name = "COMPOSITE_GROUP_SHAPE_ASPECT")]
impl SimpleEntityHandler for CompositeGroupShapeAspectHandler {
    type WriteInput = ShapeAspectSubtypeWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_composite_group_shape_aspect(entity_id, attrs)?;
        crate::early::lower::lower_composite_group_shape_aspect(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ShapeAspectSubtypeWriteInput,
    ) -> Result<u64, WriteError> {
        let early = crate::early::lift::lift_composite_group_shape_aspect(
            input.name,
            input.description,
            input.pds_step_id,
            input.product_definitional,
        );
        Ok(crate::early::serialize::serialize_composite_group_shape_aspect(buf, &early))
    }
}

pub(crate) struct CompositeShapeAspectHandler;

/// `COMPOSITE_SHAPE_ASPECT` — the supertype of `COMPOSITE_GROUP_SHAPE_ASPECT`,
/// sharing the same 4-attr `SHAPE_ASPECT` body. Stored in the shared
/// `composite_group_shape_aspects` arena (the ir.toml `composite_shape_aspect`
/// family arena) with `CompositeShapeAspectKind::Composite`, so every
/// `composite_shape_aspect_id_map` consumer (`resolve_shape_aspect_ref`,
/// `DRAUGHTING_MODEL_ITEM_ASSOCIATION`, `ID_ATTRIBUTE`,
/// `SHAPE_ASPECT_RELATIONSHIP`, …) resolves it.
#[step_entity(name = "COMPOSITE_SHAPE_ASPECT")]
impl SimpleEntityHandler for CompositeShapeAspectHandler {
    type WriteInput = ShapeAspectSubtypeWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_composite_shape_aspect(entity_id, attrs)?;
        crate::early::lower::lower_composite_shape_aspect(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ShapeAspectSubtypeWriteInput,
    ) -> Result<u64, WriteError> {
        let early = crate::early::lift::lift_composite_shape_aspect(
            input.name,
            input.description,
            input.pds_step_id,
            input.product_definitional,
        );
        Ok(crate::early::serialize::serialize_composite_shape_aspect(
            buf, &early,
        ))
    }
}

pub(crate) struct CentreOfSymmetryHandler;

#[step_entity(name = "CENTRE_OF_SYMMETRY")]
impl SimpleEntityHandler for CentreOfSymmetryHandler {
    type WriteInput = ShapeAspectSubtypeWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_centre_of_symmetry(entity_id, attrs)?;
        crate::early::lower::lower_centre_of_symmetry(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ShapeAspectSubtypeWriteInput,
    ) -> Result<u64, WriteError> {
        let early = crate::early::lift::lift_centre_of_symmetry(
            input.name,
            input.description,
            input.pds_step_id,
            input.product_definitional,
        );
        Ok(crate::early::serialize::serialize_centre_of_symmetry(
            buf, &early,
        ))
    }
}

pub(crate) struct AllAroundShapeAspectHandler;

#[step_entity(name = "ALL_AROUND_SHAPE_ASPECT")]
impl SimpleEntityHandler for AllAroundShapeAspectHandler {
    type WriteInput = ShapeAspectSubtypeWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_all_around_shape_aspect(entity_id, attrs)?;
        crate::early::lower::lower_all_around_shape_aspect(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ShapeAspectSubtypeWriteInput,
    ) -> Result<u64, WriteError> {
        let early = crate::early::lift::lift_all_around_shape_aspect(
            input.name,
            input.description,
            input.pds_step_id,
            input.product_definitional,
        );
        Ok(crate::early::serialize::serialize_all_around_shape_aspect(
            buf, &early,
        ))
    }
}

/// Resolved write input for the datum-composite complex forms — the shared
/// `SHAPE_ASPECT` body plus the [`CompositeShapeAspectKind`] that selects the
/// 3-part vs 4-part part list.
pub(crate) struct CompositeDatumShapeAspectWriteInput {
    pub(crate) kind: CompositeShapeAspectKind,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) pds_step_id: u64,
    pub(crate) product_definitional: bool,
}

pub(crate) struct CompositeDatumShapeAspectHandler;

/// AND-combined `SHAPE_ASPECT` subtype complexes that are simultaneously a
/// `COMPOSITE_(GROUP_)SHAPE_ASPECT` and a `DATUM_FEATURE`. Data lives only on
/// the `SHAPE_ASPECT` part; the other leaves are empty. Stored in the shared
/// `composite_group_shape_aspects` arena (the ir.toml `composite_shape_aspect`
/// family) with `datum_feature = true` so `resolve_shape_aspect_ref` consumers
/// resolve it and the writer re-emits the exact multi-leaf form.
#[step_entity_complex(
    name = "COMPOSITE_DATUM_SHAPE_ASPECT",
    cases = [
        ["COMPOSITE_SHAPE_ASPECT", "DATUM_FEATURE", "SHAPE_ASPECT"],
        ["COMPOSITE_GROUP_SHAPE_ASPECT", "COMPOSITE_SHAPE_ASPECT", "DATUM_FEATURE", "SHAPE_ASPECT"],
    ]
)]
impl ComplexEntityHandler for CompositeDatumShapeAspectHandler {
    type WriteInput = CompositeDatumShapeAspectWriteInput;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_composite_datum_shape_aspect(entity_id, parts)?;
        lower::lower_composite_datum_shape_aspect(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: CompositeDatumShapeAspectWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_composite_datum_shape_aspect(
            input.kind,
            input.name,
            input.description,
            input.pds_step_id,
            input.product_definitional,
        );
        Ok(serialize::serialize_composite_datum_shape_aspect(
            buf, &early,
        ))
    }
}
