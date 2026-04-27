//! `STYLED_ITEM` handler — Pass 7-10.
//!
//! Pairs a list of `PRESENTATION_STYLE_ASSIGNMENT` entries with an item
//! ref. Item resolution is multi-pool — Fusion 360 styles individual
//! `ADVANCED_FACE` entities, CATIA / `FreeCAD` typically style solids /
//! wires / loose points. Targets that don't resolve to one of the four
//! supported pools (Solid / Face / Curve / Point) are dropped at read
//! time so the writer's symmetric drop preserves round-trip equality.

use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{StyledItem, StyledItemTarget};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::presentation_style_assignment::PresentationStyleAssignmentHandler;

pub(crate) struct StyledItemHandler;

impl SimpleEntityHandler for StyledItemHandler {
    const NAME: &'static str = "STYLED_ITEM";
    const PASS_LEVEL: PassLevel = PassLevel::Pass7StyledItem;
    type WriteInput = StyledItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "STYLED_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(psa) = ctx.viz_psa_map.get(&r).cloned() {
                styles.push(psa);
            }
        }

        let item = if let Some(&sid) = ctx.solid_map.get(&item_ref) {
            StyledItemTarget::Solid(sid)
        } else if let Some(&fid) = ctx.face_map.get(&item_ref) {
            StyledItemTarget::Face(fid)
        } else if let Some(&cid) = ctx.curve_map.get(&item_ref) {
            StyledItemTarget::Curve(cid)
        } else if let Some(&pid) = ctx.point_map.get(&item_ref) {
            StyledItemTarget::Point(pid)
        } else {
            return Ok(());
        };

        ctx.viz_styled_item_map
            .insert(entity_id, StyledItem { name, styles, item });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, si: StyledItem) -> Result<u64, WriteError> {
        let item_id = match si.item {
            StyledItemTarget::Solid(sid) => buf.emit_solid(sid)?,
            StyledItemTarget::Face(fid) => buf.emit_face(fid)?,
            StyledItemTarget::Curve(cid) => buf.emit_curve(cid)?,
            StyledItemTarget::Point(pid) => buf.emit_point(pid)?,
        };
        let mut style_refs = Vec::with_capacity(si.styles.len());
        for psa in si.styles {
            style_refs.push(Attribute::EntityRef(
                PresentationStyleAssignmentHandler::write(buf, psa)?,
            ));
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

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static STYLED_ITEM_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: StyledItemHandler::NAME,
    pass_level: StyledItemHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: StyledItemHandler::read,
    },
};
