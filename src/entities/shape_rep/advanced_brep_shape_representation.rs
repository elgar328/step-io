//! `ADVANCED_BREP_SHAPE_REPRESENTATION` handler.
//!
//! Reader resolves every `items` ref into a typed `RepresentationItemRef`
//! (usually `MANIFOLD_SOLID_BREP`s + an `AXIS2_PLACEMENT_3D` frame, but an
//! assembly ABSR lists `MAPPED_ITEM`s) and stores them in source order on
//! `AdvancedBrepRepr.items`; the legacy `absr_solid_map` / `absr_ref_frame_map`
//! side maps are derived from the resolved items. The arena writer
//! (`emit_representation`) re-emits items in order, deferring assembly ABSRs
//! (`MAPPED_ITEM` forward-ref) via reserve-then-fill.

use crate::entities::SimpleEntityHandler;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::assembly::Product;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::representation_item::RepresentationItemRef;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AdvancedBrepShapeRepresentationWriteInput {
    pub(crate) product: Product,
    pub(crate) solid_refs: Vec<u64>,
    pub(crate) unit_ctx: u64,
}

pub(crate) struct AdvancedBrepShapeRepresentationHandler;

#[step_entity(name = "ADVANCED_BREP_SHAPE_REPRESENTATION")]
impl SimpleEntityHandler for AdvancedBrepShapeRepresentationHandler {
    type WriteInput = AdvancedBrepShapeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "ADVANCED_BREP_SHAPE_REPRESENTATION")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let items = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        let context = ctx.resolve_repr_context(ctx_ref);
        if let Some(crate::ir::shape_rep::RepresentationContextRef::Unitful(ctx_id)) = context {
            ctx.repr_context_map.insert(entity_id, ctx_id);
        }

        // `items` is `SET OF representation_item`: usually MANIFOLD_SOLID_BREPs
        // + an AXIS2_PLACEMENT_3D, but an assembly ABSR lists MAPPED_ITEMs.
        // Resolve each into a typed ref, preserving source order.
        let resolved: Vec<RepresentationItemRef> = items
            .iter()
            .filter_map(|r| resolve_representation_item_ref(ctx, *r))
            .collect();

        // Derive the legacy solid / ref-frame side maps (consumed by SDR
        // product-geometry / SRR / GISU) from the resolved items.
        let solid_ids: Vec<_> = resolved
            .iter()
            .filter_map(|it| match it {
                RepresentationItemRef::Solid(id) => Some(*id),
                _ => None,
            })
            .collect();
        if let Some(placement_id) = resolved.iter().find_map(|it| match it {
            RepresentationItemRef::Placement3d(id) => Some(*id),
            _ => None,
        }) {
            ctx.absr_ref_frame_map.insert(entity_id, placement_id);
        }
        if !solid_ids.is_empty() {
            ctx.absr_solid_map.insert(entity_id, solid_ids);
        }
        // Warn only when nothing resolved at all (a genuinely empty ABSR) —
        // an assembly ABSR (MAPPED_ITEM items) is not a defect.
        if resolved.is_empty() {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: String::from("ADVANCED_BREP_SHAPE_REPRESENTATION with no resolvable items"),
            });
        }

        // representation-refactor A-1: dual-write into the unified arena.
        let repr_id = ctx
            .representations
            .push(crate::ir::shape_rep::Representation::AdvancedBrep(
                crate::ir::shape_rep::AdvancedBrepRepr {
                    name,
                    context,
                    items: resolved,
                },
            ));
        ctx.id_cache.insert(entity_id, repr_id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        AdvancedBrepShapeRepresentationWriteInput {
            product,
            solid_refs,
            unit_ctx,
        }: AdvancedBrepShapeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        let axis_ref = buf.emit_axis2_placement_3d(product.shape_ref_frame)?;
        let mut items = Vec::with_capacity(1 + solid_refs.len());
        items.push(Attribute::EntityRef(axis_ref));
        items.extend(solid_refs.into_iter().map(Attribute::EntityRef));
        Ok(buf.push_simple(
            "ADVANCED_BREP_SHAPE_REPRESENTATION",
            vec![
                Attribute::String(String::new()),
                Attribute::List(items),
                Attribute::EntityRef(unit_ctx),
            ],
        ))
    }
}
