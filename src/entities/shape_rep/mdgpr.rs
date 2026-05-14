//! `MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION` handler —
//! Pass 7-11. Top-level visualization wrapper holding a list of
//! `STYLED_ITEM`s plus an optional unit context.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::Mdgpr;
use crate::ir::visualization::VisualizationPool;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use crate::entities::visualization::styled_item::StyledItemHandler;

pub(crate) struct MdgprHandler;

impl SimpleEntityHandler for MdgprHandler {
    const NAME: &'static str = "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION";
    const PASS_LEVEL: PassLevel = PassLevel::Pass7Mdgpr;
    type WriteInput = Mdgpr;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            3,
            entity_id,
            "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION",
        )?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
        let context = ctx.context_id_map.get(&ctx_ref).copied();

        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(si) = ctx.viz_styled_item_map.get(&r).cloned() {
                items.push(si);
            }
        }

        ctx.visualization
            .get_or_insert_with(VisualizationPool::default)
            .mdgprs
            .push(Mdgpr {
                name,
                items,
                context,
            });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, mdgpr: Mdgpr) -> Result<u64, WriteError> {
        let mut item_refs = Vec::with_capacity(mdgpr.items.len());
        for si in mdgpr.items {
            let id = StyledItemHandler::write(buf, si)?;
            item_refs.push(Attribute::EntityRef(id));
        }
        // MDGPR's `context_of_items` is required by the spec but the IR
        // accepts `None` for kernel-built fragments. Some(id) → resolve via
        // cached `unit_context_ids`; None → emit `Unset`.
        let context = match mdgpr.context {
            Some(id) => Attribute::EntityRef(buf.unit_context_ids[id.0 as usize]),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION",
            vec![
                Attribute::String(mdgpr.name),
                Attribute::List(item_refs),
                context,
            ],
        ))
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static MDGPR_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: MdgprHandler::NAME,
    pass_level: MdgprHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: MdgprHandler::read,
    },
};
