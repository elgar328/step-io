//! `STYLED_ITEM` handler.
//!
//! Pairs a list of `PRESENTATION_STYLE_ASSIGNMENT` entries with a target
//! geometry/topology object. The reader pushes the resolved
//! `StyledItem::Plain(...)` into `VisualizationPool::styled_items` and
//! records the `StyledItemId` in `viz_styled_item_id_map` so the MDGPR
//! reader can build its `items: Vec<StyledItemId>` list. Writer pulls the
//! cached STEP id from `WriteBuffer::styled_item_step_ids` and emits the
//! body fresh per call.
//!
//! `STYLED_ITEM.item` is a `representation_item` ref, resolved through the
//! shared [`resolve_representation_item_ref`] into a [`RepresentationItemRef`]
//! (geometry, topology, geometry representation, or 3D placement). Targets
//! that resolve to an unmodelled kind are silently dropped to preserve
//! round-trip equality on the supported subset.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::representation_item::RepresentationItemRef;
use crate::ir::shape_rep::Representation;
use crate::ir::visualization::{PlainStyledItem, StyledItem, VisualizationPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use step_io_macros::step_entity;

pub(crate) struct StyledItemHandler;

#[step_entity(name = "STYLED_ITEM")]
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
            if let Some(psa_id) = ctx
                .id_cache
                .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
            {
                styles.push(psa_id);
            }
        }

        let Some(item) = resolve_representation_item_ref(ctx, item_ref) else {
            // If the target was dropped as a dangling-reference cascade (its
            // geometry transitively required an undefined entity), drop this
            // STYLED_ITEM as the same normalization rather than a defect — the
            // resolver returns `None` (not a `MissingReference`), so the
            // dispatcher's general rule cannot see it. [NS-dangling-reference-drop]
            if ctx.nonstandard_dropped_refs.contains(&item_ref) {
                ctx.record_nonstandard(
                    "STYLED_ITEM".to_string(),
                    "dropped (dangling/cascade reference)",
                );
                ctx.nonstandard_dropped_refs.insert(entity_id);
                return Ok(());
            }
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "STYLED_ITEM target #{item_ref} did not resolve to a modelled \
                     representation_item kind (likely cascade from a dropped dependency)"
                ),
            });
            return Ok(());
        };

        let pool = ctx
            .visualization
            .get_or_insert_with(VisualizationPool::default);
        let id = pool
            .styled_items
            .push(StyledItem::Plain(PlainStyledItem { name, styles, item }));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, si: PlainStyledItem) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(si.item)?;
        let mut style_refs = Vec::with_capacity(si.styles.len());
        for psa_id in si.styles {
            style_refs.push(Attribute::EntityRef(buf.step_id(psa_id)));
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

/// Resolve a `representation_item` ref against the geometry / topology /
/// placement / representation reader maps, returning the matching
/// [`RepresentationItemRef`] variant. Each STEP `#N` is exactly one entity,
/// so at most one map matches; the lookup order is for readability only.
/// Returns `None` when the ref points at a representation-item kind step-io
/// does not model as a `RepresentationItemRef` variant.
///
/// MDGPR guard: `repr_id_map` also holds `MDGPR` entries, but only
/// non-`Mdgpr` representations become a `Representation` ref; an MDGPR
/// target falls through to `None`.
#[allow(clippy::too_many_lines)]
pub(crate) fn resolve_representation_item_ref(
    ctx: &ReaderContext,
    item_ref: u64,
) -> Option<RepresentationItemRef> {
    if let Some(sid) = ctx.id_cache.get::<crate::ir::id::SolidId>(item_ref) {
        return Some(RepresentationItemRef::Solid(sid));
    }
    if let Some(&fid) = ctx.face_map.get(&item_ref) {
        return Some(RepresentationItemRef::Face(fid));
    }
    if let Some(&eid) = ctx.edge_map.get(&item_ref) {
        return Some(RepresentationItemRef::Edge(eid));
    }
    if let Some(&cid) = ctx.curve_map.get(&item_ref) {
        return Some(RepresentationItemRef::Curve(cid));
    }
    if let Some(&pid) = ctx.point_map.get(&item_ref) {
        return Some(RepresentationItemRef::Point(pid));
    }
    if let Some(&sid) = ctx.surface_map.get(&item_ref) {
        return Some(RepresentationItemRef::Surface(sid));
    }
    if let Some(&vid) = ctx.vertex_map.get(&item_ref) {
        return Some(RepresentationItemRef::Vertex(vid));
    }
    if let Some(&shid) = ctx.shell_map.get(&item_ref) {
        return Some(RepresentationItemRef::Shell(shid));
    }
    if let Some(&plid) = ctx.placement_map.get(&item_ref) {
        return Some(RepresentationItemRef::Placement3d(plid));
    }
    if let Some(&pl2id) = ctx.placement_2d_map.get(&item_ref) {
        return Some(RepresentationItemRef::Placement2d(pl2id));
    }
    if let Some(peid) = ctx.id_cache.get::<crate::ir::id::PlanarExtentId>(item_ref) {
        return Some(RepresentationItemRef::PlanarExtent(peid));
    }
    if let Some(rid) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(item_ref)
    {
        if !matches!(ctx.representations[rid], Representation::Mdgpr(_)) {
            return Some(RepresentationItemRef::Representation(rid));
        }
    }
    // representation_item arena (phase repr-item-arena-1) — last-resort
    // fallback after the typed per-type arenas above.
    if let Some(rid) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationItemId>(item_ref)
    {
        return Some(RepresentationItemRef::RepresentationItem(rid));
    }
    // geometric_representation_item arena (phase sbsm-cluster) — covers
    // standalone SBSMs targeted directly by STYLED_ITEM. MSSR writer now
    // also resolves its child SBSMs through the same cache, so the SBSM
    // emits exactly once even when referenced from both ends.
    if let Some(&gri_id) = ctx.sbsm_id_map.get(&item_ref) {
        return Some(RepresentationItemRef::GeometricRepresentationItem(gri_id));
    }
    // Same role for GEOMETRIC_(CURVE_)SET — GBWSR / GBSSR writer also
    // routes through the GRI cache (phase gcs-cluster).
    if let Some(&gri_id) = ctx.curve_set_id_map.get(&item_ref) {
        return Some(RepresentationItemRef::GeometricRepresentationItem(gri_id));
    }
    // DEFINED_SYMBOL (GRI arena) — a styled LEADER_TERMINATOR targets the
    // terminator symbol content (phase styled-annotation-symbol).
    if let Some(&gri_id) = ctx.defined_symbol_id_map.get(&item_ref) {
        return Some(RepresentationItemRef::GeometricRepresentationItem(gri_id));
    }
    // tessellated_item arena — STYLED_ITEM can target a TESSELLATED_SOLID
    // (or any tessellation item) directly (phase tessellation-repr-item).
    if let Some(tid) = ctx
        .id_cache
        .get::<crate::ir::id::TessellatedItemId>(item_ref)
    {
        return Some(RepresentationItemRef::TessellatedItem(tid));
    }
    // tessellated_face arena — STYLED_ITEM styles a COMPLEX_TRIANGULATED_FACE
    // per-face (phase styled-item-tess-face).
    if let Some(fid) = ctx
        .id_cache
        .get::<crate::ir::id::TessellatedFaceId>(item_ref)
    {
        return Some(RepresentationItemRef::TessellatedFace(fid));
    }
    // mapped_items arena (phase si-mapped-item) — STYLED_ITEM / CDORSI
    // routinely target a MAPPED_ITEM in grabcad-style assemblies (PMI
    // annotation instance entry point).
    if let Some(mi_id) = ctx.id_cache.get::<crate::ir::id::MappedItemId>(item_ref) {
        return Some(RepresentationItemRef::MappedItem(mi_id));
    }
    // PMI entries reachable from DRAUGHTING_MODEL.items (phase
    // rir-pmi-variants): AP242 MBD pattern.
    if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::AnnotationOccurrenceId>(item_ref)
    {
        return Some(RepresentationItemRef::AnnotationOccurrence(id));
    }
    // annotation_curve_occurrence arena (plain ACO / LEADER_CURVE) — CIWR /
    // STYLED_ITEM can target it (phase plain-aco).
    if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::AnnotationCurveOccurrenceId>(item_ref)
    {
        return Some(RepresentationItemRef::AnnotationCurveOccurrence(id));
    }
    if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::DraughtingCalloutId>(item_ref)
    {
        return Some(RepresentationItemRef::DraughtingCallout(id));
    }
    if let Some(id) = ctx.id_cache.get::<crate::ir::id::CameraModelId>(item_ref) {
        return Some(RepresentationItemRef::CameraModel(id));
    }
    // text content (annotation_text_occurrence_item SELECT) — a styled
    // ANNOTATION_TEXT_OCCURRENCE targets these (phase styled-annotation-text).
    if let Some(id) = ctx.id_cache.get::<crate::ir::id::TextLiteralId>(item_ref) {
        return Some(RepresentationItemRef::TextLiteral(id));
    }
    if let Some(id) = ctx.id_cache.get::<crate::ir::id::CompositeTextId>(item_ref) {
        return Some(RepresentationItemRef::CompositeText(id));
    }
    // STYLED_ITEM is itself a representation_item — DRAUGHTING_MODEL.items lists
    // styled annotations directly (phase dm-styled-item). Probed last so a
    // styled item never shadows a more specific geometry/annotation target.
    if let Some(id) = ctx.id_cache.get::<crate::ir::id::StyledItemId>(item_ref) {
        return Some(RepresentationItemRef::StyledItem(id));
    }
    None
}
