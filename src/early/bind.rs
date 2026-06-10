//! `bind`: `EntityGraph` (late-bound) → `EarlyModel` (L1). Mechanical
//! attribute extraction only — no SELECT resolution, no flatten, no arena
//! lookups (those are [`lower`](super::lower)'s job). In the fully-migrated
//! design this layer is codegen output; here it is hand-written for the two
//! pilot entities.

use crate::early::model::{EarlySurfaceSideStyle, EarlySurfaceStyleFillArea, EarlyViewVolume};
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref, read_entity_ref_list, read_enum, read_real,
    read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::visualization::Projection;
use crate::parser::entity::Attribute;

/// `SURFACE_STYLE_FILL_AREA(fill_area)` → L1.
pub(crate) fn bind_surface_style_fill_area(
    entity_id: u64,
    attrs: &[Attribute],
) -> Result<EarlySurfaceStyleFillArea, ConvertError> {
    check_count(attrs, 1, entity_id, "SURFACE_STYLE_FILL_AREA")?;
    let fill_area = read_entity_ref(attrs, 0, entity_id, "fill_area")?;
    Ok(EarlySurfaceStyleFillArea { fill_area })
}

/// `SURFACE_SIDE_STYLE(name, styles)` → L1.
pub(crate) fn bind_surface_side_style(
    entity_id: u64,
    attrs: &[Attribute],
) -> Result<EarlySurfaceSideStyle, ConvertError> {
    check_count(attrs, 2, entity_id, "SURFACE_SIDE_STYLE")?;
    let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let styles = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
    Ok(EarlySurfaceSideStyle { name, styles })
}

/// Map a STEP `central_or_parallel` enum value to [`Projection`].
fn bind_projection(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Projection, ConvertError> {
    match read_enum(attrs, index, entity_id, field_name)? {
        "CENTRAL" => Ok(Projection::Central),
        "PARALLEL" => Ok(Projection::Parallel),
        other => Err(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!("{field_name}: unknown central_or_parallel '.{other}.'"),
        }),
    }
}

/// `VIEW_VOLUME(...)` (9 attrs) → L1.
pub(crate) fn bind_view_volume(
    entity_id: u64,
    attrs: &[Attribute],
) -> Result<EarlyViewVolume, ConvertError> {
    check_count(attrs, 9, entity_id, "VIEW_VOLUME")?;
    Ok(EarlyViewVolume {
        projection_type: bind_projection(attrs, 0, entity_id, "projection_type")?,
        projection_point: read_entity_ref(attrs, 1, entity_id, "projection_point")?,
        view_plane_distance: read_real(attrs, 2, entity_id, "view_plane_distance")?,
        front_plane_distance: read_real(attrs, 3, entity_id, "front_plane_distance")?,
        front_plane_clipping: read_bool(attrs, 4, entity_id, "front_plane_clipping")?,
        back_plane_distance: read_real(attrs, 5, entity_id, "back_plane_distance")?,
        back_plane_clipping: read_bool(attrs, 6, entity_id, "back_plane_clipping")?,
        view_volume_sides_clipping: read_bool(attrs, 7, entity_id, "view_volume_sides_clipping")?,
        view_window: read_entity_ref(attrs, 8, entity_id, "view_window")?,
    })
}
