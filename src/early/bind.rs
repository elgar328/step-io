//! `bind`: `EntityGraph` (late-bound) → `EarlyModel` (L1). Mechanical
//! attribute extraction only — no SELECT resolution, no flatten, no arena
//! lookups (those are [`lower`](super::lower)'s job). In the fully-migrated
//! design this layer is codegen output; here it is hand-written for the two
//! pilot entities.

use crate::early::model::{EarlySurfaceSideStyle, EarlySurfaceStyleFillArea};
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
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
