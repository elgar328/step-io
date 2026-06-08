//! `SHAPE_ASPECT` subtype handlers.
//!
//! `COMPOSITE_GROUP_SHAPE_ASPECT` / `CENTRE_OF_SYMMETRY` /
//! `ALL_AROUND_SHAPE_ASPECT` share `SHAPE_ASPECT`'s 4-attr shape
//! (name, description, `of_shape`, `product_definitional`). The ir.toml
//! blueprint gives each its own arena, so each round-trips under its own
//! STEP entity name. The reader/writer bodies are shared here — only the
//! entity name and target arena differ per handler.

use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::ProductId;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::{
    AllAroundShapeAspect, CentreOfSymmetry, CompositeGroupShapeAspect, CompositeShapeAspectKind,
};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, has_all_parts, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::{step_entity, step_entity_complex};

/// Resolved write input shared by all three subtype handlers.
pub(crate) struct ShapeAspectSubtypeWriteInput {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) pds_step_id: u64,
    pub(crate) product_definitional: bool,
}

/// Read the shared `SHAPE_ASPECT` 4-attr body and resolve `of_shape` to a
/// `ProductId`. `Ok(None)` when the target ref does not resolve — the
/// subtype is dropped, mirroring `ShapeAspectHandler`'s policy.
fn read_shape_aspect_subtype(
    ctx: &ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    entity_name: &'static str,
) -> Result<Option<(String, String, ProductId, bool)>, ConvertError> {
    check_count(attrs, 4, entity_id, entity_name)?;
    let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
    let of_shape_ref = read_entity_ref(attrs, 2, entity_id, "of_shape")?;
    let product_definitional = read_bool(attrs, 3, entity_id, "product_definitional")?;

    // of_shape → PRODUCT_DEFINITION_SHAPE → PRODUCT_DEFINITION → ProductId.
    let Some(&pdef_step_id) = ctx.pdef_shape_to_pdef.get(&of_shape_ref) else {
        return Ok(None);
    };
    let Some(&product_step_id) = ctx.pdef_to_product.get(&pdef_step_id) else {
        return Ok(None);
    };
    let Some(&product_id) = ctx.product_arena_map.get(&product_step_id) else {
        return Ok(None);
    };
    Ok(Some((name, description, product_id, product_definitional)))
}

/// Emit a `SHAPE_ASPECT`-shaped subtype line under `entity_name`.
fn write_shape_aspect_subtype(
    buf: &mut WriteBuffer,
    entity_name: &str,
    input: ShapeAspectSubtypeWriteInput,
) -> u64 {
    let bool_attr = if input.product_definitional { "T" } else { "F" };
    buf.push_simple(
        entity_name,
        vec![
            Attribute::String(input.name),
            Attribute::String(input.description),
            Attribute::EntityRef(input.pds_step_id),
            Attribute::Enum(bool_attr.into()),
        ],
    )
}

pub(crate) struct CompositeGroupShapeAspectHandler;

#[step_entity(name = "COMPOSITE_GROUP_SHAPE_ASPECT")]
impl SimpleEntityHandler for CompositeGroupShapeAspectHandler {
    type WriteInput = ShapeAspectSubtypeWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some((name, description, target, product_definitional)) =
            read_shape_aspect_subtype(ctx, entity_id, attrs, "COMPOSITE_GROUP_SHAPE_ASPECT")?
        else {
            return Ok(());
        };
        let id = ctx
            .composite_group_shape_aspects
            .push(CompositeGroupShapeAspect {
                name,
                description,
                target,
                product_definitional,
                kind: CompositeShapeAspectKind::Group,
                datum_feature: false,
            });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ShapeAspectSubtypeWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(write_shape_aspect_subtype(
            buf,
            "COMPOSITE_GROUP_SHAPE_ASPECT",
            input,
        ))
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some((name, description, target, product_definitional)) =
            read_shape_aspect_subtype(ctx, entity_id, attrs, "COMPOSITE_SHAPE_ASPECT")?
        else {
            return Ok(());
        };
        let id = ctx
            .composite_group_shape_aspects
            .push(CompositeGroupShapeAspect {
                name,
                description,
                target,
                product_definitional,
                kind: CompositeShapeAspectKind::Composite,
                datum_feature: false,
            });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ShapeAspectSubtypeWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(write_shape_aspect_subtype(
            buf,
            "COMPOSITE_SHAPE_ASPECT",
            input,
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some((name, description, target, product_definitional)) =
            read_shape_aspect_subtype(ctx, entity_id, attrs, "CENTRE_OF_SYMMETRY")?
        else {
            return Ok(());
        };
        let id = ctx.centre_of_symmetries.push(CentreOfSymmetry {
            name,
            description,
            target,
            product_definitional,
        });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ShapeAspectSubtypeWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(write_shape_aspect_subtype(buf, "CENTRE_OF_SYMMETRY", input))
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some((name, description, target, product_definitional)) =
            read_shape_aspect_subtype(ctx, entity_id, attrs, "ALL_AROUND_SHAPE_ASPECT")?
        else {
            return Ok(());
        };
        let id = ctx.all_around_shape_aspects.push(AllAroundShapeAspect {
            name,
            description,
            target,
            product_definitional,
        });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: ShapeAspectSubtypeWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(write_shape_aspect_subtype(
            buf,
            "ALL_AROUND_SHAPE_ASPECT",
            input,
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
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // All four attributes live on the SHAPE_ASPECT part; the other leaves
        // are `()`. Reuse the shared subtype body reader.
        let attrs = require_part_attrs(parts, "SHAPE_ASPECT", entity_id)?;
        let Some((name, description, target, product_definitional)) =
            read_shape_aspect_subtype(ctx, entity_id, attrs, "SHAPE_ASPECT")?
        else {
            return Ok(());
        };
        let kind = if has_all_parts(parts, &["COMPOSITE_GROUP_SHAPE_ASPECT"]) {
            CompositeShapeAspectKind::Group
        } else {
            CompositeShapeAspectKind::Composite
        };
        let id = ctx
            .composite_group_shape_aspects
            .push(CompositeGroupShapeAspect {
                name,
                description,
                target,
                product_definitional,
                kind,
                datum_feature: true,
            });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        input: CompositeDatumShapeAspectWriteInput,
    ) -> Result<u64, WriteError> {
        let bool_attr = if input.product_definitional { "T" } else { "F" };
        // Composite leaf tags first (alphabetical, matching the source), then
        // the data-bearing SHAPE_ASPECT part last.
        let mut parts: Vec<(String, Vec<Attribute>)> = match input.kind {
            CompositeShapeAspectKind::Composite => vec![
                ("COMPOSITE_SHAPE_ASPECT".into(), vec![]),
                ("DATUM_FEATURE".into(), vec![]),
            ],
            CompositeShapeAspectKind::Group => vec![
                ("COMPOSITE_GROUP_SHAPE_ASPECT".into(), vec![]),
                ("COMPOSITE_SHAPE_ASPECT".into(), vec![]),
                ("DATUM_FEATURE".into(), vec![]),
            ],
        };
        parts.push((
            "SHAPE_ASPECT".into(),
            vec![
                Attribute::String(input.name),
                Attribute::String(input.description),
                Attribute::EntityRef(input.pds_step_id),
                Attribute::Enum(bool_attr.into()),
            ],
        ));
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex { parts },
        });
        Ok(n)
    }
}
