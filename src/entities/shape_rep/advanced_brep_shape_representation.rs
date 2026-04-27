//! `ADVANCED_BREP_SHAPE_REPRESENTATION` handler — Pass 6-4a.
//!
//! Reader picks the first `MANIFOLD_SOLID_BREP` from the items list and
//! binds it to the ABSR id; the first `AXIS2_PLACEMENT_3D` becomes the coordinate
//! reference frame. Writer emits the ABSR line referring to the per-product
//! axis placement, the solid ref the chain orchestrator pre-emitted, and
//! the bound unit context.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::assembly::Product;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string};
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct AdvancedBrepShapeRepresentationWriteInput {
    pub(crate) product: Product,
    pub(crate) solid_ref: u64,
    pub(crate) unit_ctx: u64,
}

pub(crate) struct AdvancedBrepShapeRepresentationHandler;

impl SimpleEntityHandler for AdvancedBrepShapeRepresentationHandler {
    const NAME: &'static str = "ADVANCED_BREP_SHAPE_REPRESENTATION";
    const PASS_LEVEL: PassLevel = PassLevel::Pass6ShapeRep;
    type WriteInput = AdvancedBrepShapeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "ADVANCED_BREP_SHAPE_REPRESENTATION")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let items = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        if let Some(&ctx_id) = ctx.context_id_map.get(&ctx_ref) {
            ctx.repr_context_map.insert(entity_id, ctx_id);
        }

        let solid_refs: Vec<u64> = items
            .iter()
            .filter(|r| ctx.solid_map.contains_key(r))
            .copied()
            .collect();
        // Pick the first AXIS2_PLACEMENT_3D in the items list as the coordinate
        // reference frame. In practice commercial CAD output places it first.
        if let Some(&placement_id) = items.iter().find_map(|r| ctx.placement_map.get(r)) {
            ctx.absr_ref_frame_map.insert(entity_id, placement_id);
        }
        match solid_refs.as_slice() {
            [] => {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: String::from(
                        "ADVANCED_BREP_SHAPE_REPRESENTATION without a MANIFOLD_SOLID_BREP item",
                    ),
                });
                Ok(())
            }
            [solid_ref, ..] => {
                if solid_refs.len() > 1 {
                    ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                        entity_id,
                        detail: format!(
                            "ADVANCED_BREP_SHAPE_REPRESENTATION has {} MANIFOLD_SOLID_BREP items, using the first",
                            solid_refs.len()
                        ),
                    });
                }
                let solid_id = ctx.solid_map[solid_ref];
                ctx.absr_solid_map.insert(entity_id, solid_id);
                Ok(())
            }
        }
    }

    fn write(
        buf: &mut WriteBuffer,
        AdvancedBrepShapeRepresentationWriteInput {
            product,
            solid_ref,
            unit_ctx,
        }: AdvancedBrepShapeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        let axis_ref = buf.emit_axis2_placement_3d(product.shape_ref_frame)?;
        Ok(buf.push_simple(
            "ADVANCED_BREP_SHAPE_REPRESENTATION",
            vec![
                Attribute::String(String::new()),
                Attribute::List(vec![
                    Attribute::EntityRef(axis_ref),
                    Attribute::EntityRef(solid_ref),
                ]),
                Attribute::EntityRef(unit_ctx),
            ],
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static ABSR_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: AdvancedBrepShapeRepresentationHandler::NAME,
    pass_level: AdvancedBrepShapeRepresentationHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: AdvancedBrepShapeRepresentationHandler::read,
    },
};
