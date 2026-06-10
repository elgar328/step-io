//! `bind`: `EntityGraph` (late-bound) → `EarlyModel` (L1). Mechanical
//! attribute extraction only — no SELECT resolution, no flatten, no arena
//! lookups (those are [`lower`](super::lower)'s job). In the fully-migrated
//! design this layer is codegen output; here it is hand-written for the two
//! pilot entities.

use crate::early::model::{
    EarlyMarker, EarlyMarkerSize, EarlyPointStyle, EarlySurfaceSideStyle,
    EarlySurfaceStyleFillArea, EarlySurfaceStyleUsage, EarlyViewVolume,
};
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref, read_entity_ref_list, read_enum, read_real,
    read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{MarkerType, Projection, SurfaceSide};
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

/// `SURFACE_STYLE_USAGE(side, style)` → L1.
pub(crate) fn bind_surface_style_usage(
    entity_id: u64,
    attrs: &[Attribute],
) -> Result<EarlySurfaceStyleUsage, ConvertError> {
    check_count(attrs, 2, entity_id, "SURFACE_STYLE_USAGE")?;
    let side = match read_enum(attrs, 0, entity_id, "side")? {
        "POSITIVE" => SurfaceSide::Front,
        "NEGATIVE" => SurfaceSide::Back,
        _ => SurfaceSide::Both, // BOTH or unknown
    };
    let style = read_entity_ref(attrs, 1, entity_id, "style")?;
    Ok(EarlySurfaceStyleUsage { side, style })
}

/// `POINT_STYLE(name, marker, marker_size, marker_colour)` → L1.
///
/// Returns `Ok(None)` when a SELECT member has an unrecognized form (the
/// previous handler silently dropped these via `return Ok(())`); resolve-time
/// drops (unresolved refs) stay in [`lower`](super::lower).
pub(crate) fn bind_point_style(
    entity_id: u64,
    attrs: &[Attribute],
) -> Result<Option<EarlyPointStyle>, ConvertError> {
    check_count(attrs, 4, entity_id, "POINT_STYLE")?;
    let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let marker = match &attrs[1] {
        Attribute::EntityRef(n) => EarlyMarker::Predefined(*n),
        // `marker_select`'s `marker_type` member is P21 type-tagged as
        // `MARKER_TYPE(.PLUS.)` (NIST fixtures); a bare `.PLUS.` is also accepted.
        Attribute::Typed { type_name, value } if type_name == "MARKER_TYPE" => {
            let Attribute::Enum(token) = value.as_ref() else {
                return Ok(None);
            };
            EarlyMarker::Type(marker_type_from_token(token))
        }
        Attribute::Enum(token) => EarlyMarker::Type(marker_type_from_token(token)),
        _ => return Ok(None),
    };
    let marker_size = match &attrs[2] {
        Attribute::Typed { type_name, value } => match (type_name.as_str(), value.as_ref()) {
            ("POSITIVE_LENGTH_MEASURE", Attribute::Real(v)) => EarlyMarkerSize::PositiveLength(*v),
            ("POSITIVE_LENGTH_MEASURE", Attribute::Integer(v)) => {
                #[allow(clippy::cast_precision_loss)]
                let f = *v as f64;
                EarlyMarkerSize::PositiveLength(f)
            }
            ("DESCRIPTIVE_MEASURE", Attribute::String(s)) => {
                EarlyMarkerSize::Descriptive(s.clone())
            }
            _ => return Ok(None),
        },
        Attribute::EntityRef(n) => EarlyMarkerSize::MeasureWithUnit(*n),
        _ => return Ok(None),
    };
    let marker_colour = read_entity_ref(attrs, 3, entity_id, "marker_colour")?;
    Ok(Some(EarlyPointStyle {
        name,
        marker,
        marker_size,
        marker_colour,
    }))
}

/// STEP `marker_type` enum token → [`MarkerType`].
fn marker_type_from_token(token: &str) -> MarkerType {
    match token {
        "DOT" => MarkerType::Dot,
        "X" => MarkerType::X,
        "PLUS" => MarkerType::Plus,
        "ASTERISK" => MarkerType::Asterisk,
        "RING" => MarkerType::Ring,
        "SQUARE" => MarkerType::Square,
        "TRIANGLE" => MarkerType::Triangle,
        other => MarkerType::Other(other.to_owned()),
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
