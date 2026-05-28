//! `GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION` handler —
//! Pass 6-4g.
//!
//! Hosts the shared reader/writer body that the GBSSR sister imports.
//! Both wrappers share the same `(name, items, context)` shape: the items
//! list contains an axis placement (often omitted by CATIA in the SURFACE
//! flavour) plus one or more `GEOMETRIC_(CURVE_)SET`s. The reader collapses
//! the curve sets into a single `WireframeContent` and stamps the
//! `repr_kind` flag so the writer can re-emit the original wrapper name.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::{Product, WireframeContent, WireframeReprKind};
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use crate::entities::geometry::geometric_curve_set::{
    CurveSetWriteInput, GeometricCurveSetHandler,
};
use crate::entities::geometry::geometric_set::GeometricSetHandler;
use step_io_macros::step_entity;

pub(crate) struct WireframeRepresentationWriteInput {
    pub(crate) product: Product,
    pub(crate) wireframe: WireframeContent,
    pub(crate) unit_ctx: u64,
}

/// Shared reader body for GBWSR and GBSSR. `repr_kind` lets the writer
/// re-emit the same wrapper the source file used.
pub(crate) fn read_wireframe_representation_body(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    repr_kind: WireframeReprKind,
) -> Result<(), ConvertError> {
    check_count(
        attrs,
        3,
        entity_id,
        "GEOMETRICALLY_BOUNDED_*_SHAPE_REPRESENTATION",
    )?;
    let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let items = read_entity_ref_list(attrs, 1, entity_id, "items")?;
    let ctx_ref = read_entity_ref(attrs, 2, entity_id, "context_of_items")?;
    let context = ctx.resolve_repr_context(ctx_ref);
    if let Some(crate::ir::shape_rep::RepresentationContextRef::Unitful(ctx_id)) = context {
        ctx.repr_context_map.insert(entity_id, ctx_id);
    }

    let ref_frame = items.iter().find_map(|r| ctx.placement_map.get(r).copied());
    if let Some(placement_id) = ref_frame {
        ctx.wireframe_ref_frame_map.insert(entity_id, placement_id);
    }
    let mut curves = Vec::new();
    let mut points = Vec::new();
    for r in &items {
        if let Some((c, p)) = ctx.curve_set_map.get(r) {
            curves.extend_from_slice(c);
            points.extend_from_slice(p);
        } else if let Some(&cid) = ctx.curve_map.get(r) {
            // Some producers attach curves directly without a wrapping
            // GEOMETRIC_CURVE_SET — accept that form too.
            curves.push(cid);
        }
    }
    let wireframe = WireframeContent {
        curves,
        points,
        repr_kind,
    };
    ctx.wireframe_data_map.insert(entity_id, wireframe.clone());

    // Preserve the child GCS/GS's unified GRI id so the writer can route
    // emit through the GRI cache (phase gcs-cluster). GBWSR/GBSSR carry
    // at most one GCS in the corpus — take the first matching item.
    let gcs_id = items
        .iter()
        .find_map(|r| ctx.curve_set_id_map.get(r).copied());

    // representation-refactor A-1: dual-write into the unified arena.
    let repr_id = ctx
        .representations
        .push(crate::ir::shape_rep::Representation::Wireframe(
            crate::ir::shape_rep::WireframeRepr {
                name,
                context,
                ref_frame,
                content: wireframe,
                gcs_id,
            },
        ));
    ctx.repr_id_map.insert(entity_id, repr_id);
    Ok(())
}

/// Shared writer body. Picks the requested wrapper name and dispatches
/// the inner curve set through `GeometricCurveSetHandler` /
/// `GeometricSetHandler` based on whether loose points coexist with
/// curves.
pub(crate) fn write_wireframe_representation(
    buf: &mut WriteBuffer,
    repr_name: &'static str,
    input: WireframeRepresentationWriteInput,
) -> Result<u64, WriteError> {
    let WireframeRepresentationWriteInput {
        product,
        wireframe,
        unit_ctx,
    } = input;
    let axis_ref = buf.emit_axis2_placement_3d(product.shape_ref_frame)?;
    let set_input = CurveSetWriteInput {
        curves: wireframe.curves.clone(),
        points: wireframe.points.clone(),
    };
    let set_ref = if wireframe.points.is_empty() {
        GeometricCurveSetHandler::write(buf, set_input)?
    } else {
        GeometricSetHandler::write(buf, set_input)?
    };
    Ok(buf.push_simple(
        repr_name,
        vec![
            Attribute::String(String::new()),
            Attribute::List(vec![
                Attribute::EntityRef(axis_ref),
                Attribute::EntityRef(set_ref),
            ]),
            Attribute::EntityRef(unit_ctx),
        ],
    ))
}

pub(crate) struct GbwsrHandler;

#[step_entity(name = "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION", pass = Pass6Gbsr)]
impl SimpleEntityHandler for GbwsrHandler {
    type WriteInput = WireframeRepresentationWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_wireframe_representation_body(ctx, entity_id, attrs, WireframeReprKind::Wireframe)
    }

    fn write(
        buf: &mut WriteBuffer,
        input: WireframeRepresentationWriteInput,
    ) -> Result<u64, WriteError> {
        write_wireframe_representation(
            buf,
            "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION",
            input,
        )
    }
}
