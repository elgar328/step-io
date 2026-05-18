//! `STYLED_ITEM` handler — Pass 7-10.
//!
//! Pairs a list of `PRESENTATION_STYLE_ASSIGNMENT` entries with a target
//! geometry/topology object. The reader pushes the resolved
//! `StyledItem::Plain(...)` into `VisualizationPool::styled_items` and
//! records the `StyledItemId` in `viz_styled_item_id_map` so the MDGPR
//! reader can build its `items: Vec<StyledItemId>` list. Writer pulls the
//! cached STEP id from `WriteBuffer::styled_item_step_ids` and emits the
//! body fresh per call.
//!
//! `representation_item` is a broad SELECT in the schema. step-io models
//! Solid / Face / Edge / Curve / Point — covering the variants observed
//! in the reference-check corpus (Fusion-style per-face colour,
//! ABC-style per-edge curve decoration, plus solid / curve / point
//! styling seen elsewhere). Targets resolving to other variants are
//! silently dropped to preserve round-trip equality on the supported
//! subset.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{PlainStyledItem, StyledItem, StyledItemTarget, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use step_io_macros::step_entity;

pub(crate) struct StyledItemHandler;

#[step_entity(name = "STYLED_ITEM", pass = Pass7StyledItem)]
impl SimpleEntityHandler for StyledItemHandler {
    type WriteInput = PlainStyledItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "STYLED_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(&psa_id) = ctx.viz_psa_id_map.get(&r) {
                styles.push(psa_id);
            }
        }

        let Some(item) = resolve_styled_item_target(ctx, item_ref) else {
            return Ok(());
        };

        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .styled_items
            .push(StyledItem::Plain(PlainStyledItem { name, styles, item }));
        ctx.viz_styled_item_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, si: PlainStyledItem) -> Result<u64, WriteError> {
        let item_id = match si.item {
            StyledItemTarget::Solid(sid) => buf.emit_solid(sid)?,
            StyledItemTarget::Face(fid) => buf.emit_face(fid)?,
            StyledItemTarget::Edge(eid) => buf.emit_edge(eid)?,
            StyledItemTarget::Curve(cid) => buf.emit_curve(cid)?,
            StyledItemTarget::Point(pid) => buf.emit_point(pid)?,
        };
        let mut style_refs = Vec::with_capacity(si.styles.len());
        for psa_id in si.styles {
            style_refs.push(Attribute::EntityRef(buf.psa_step_ids[psa_id.0 as usize]));
        }
        Ok(buf.push_simple(
            "STYLED_ITEM",
            vec![
                Attribute::String(si.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(item_id),
            ],
        ))
    }
}

/// Resolve a `representation_item` ref against the geometry/topology
/// reader maps, returning the matching [`StyledItemTarget`] variant. The
/// lookup order mirrors the variant declaration order in
/// `StyledItemTarget`. Returns `None` when the ref points at an
/// unsupported representation-item kind.
pub(crate) fn resolve_styled_item_target(
    ctx: &ReaderContext,
    item_ref: u64,
) -> Option<StyledItemTarget> {
    if let Some(&sid) = ctx.solid_map.get(&item_ref) {
        return Some(StyledItemTarget::Solid(sid));
    }
    if let Some(&fid) = ctx.face_map.get(&item_ref) {
        return Some(StyledItemTarget::Face(fid));
    }
    if let Some(&eid) = ctx.edge_map.get(&item_ref) {
        return Some(StyledItemTarget::Edge(eid));
    }
    if let Some(&cid) = ctx.curve_map.get(&item_ref) {
        return Some(StyledItemTarget::Curve(cid));
    }
    if let Some(&pid) = ctx.point_map.get(&item_ref) {
        return Some(StyledItemTarget::Point(pid));
    }
    None
}
