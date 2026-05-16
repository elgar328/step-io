//! `MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION` handler —
//! Pass 7-11. Top-level visualization wrapper holding a list of
//! `STYLED_ITEM`s plus an optional unit context.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::Mdgpr;
use crate::ir::visualization::VisualizationPool;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use crate::entities::visualization::styled_item::StyledItemHandler;
use step_io_macros::step_entity;

pub(crate) struct MdgprHandler;

#[step_entity(name = "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION", pass = Pass7Mdgpr)]
impl SimpleEntityHandler for MdgprHandler {
    type WriteInput = Mdgpr;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
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
