//! plm-domain entity handlers. Phase plm-1a covers the Date/Time
//! primitives — the leaf entities used by the Date/Time assignment
//! chain that lands in plm-1b.

pub mod applied_approval_assignment;
pub mod applied_date_and_time_assignment;
pub mod applied_person_and_organization_assignment;
pub mod approval;
pub mod approval_date_time;
pub mod approval_person_organization;
pub mod approval_role;
pub mod approval_status;
pub mod calendar_date;
pub mod cc_design_approval;
pub mod cc_design_date_and_time_assignment;
pub mod cc_design_person_and_organization_assignment;
pub mod coordinated_universal_time_offset;
pub mod date_and_time;
pub mod date_time_role;
pub mod local_time;
pub mod organization;
pub mod person;
pub mod person_and_organization;
pub mod person_and_organization_role;

use crate::ir::ProductId;
use crate::reader::ReaderContext;

/// Resolve a `date_time_item` SELECT ref against step-io's product
/// chain. The blueprint allows `PRODUCT_DEFINITION` (and several other
/// targets); step-io currently models PD through the assembly pool, so
/// other variants drop silently. Future plm phases (Security/Approval)
/// will extend this lookup with additional arenas.
pub(crate) fn resolve_date_time_item(ctx: &ReaderContext, item_ref: u64) -> Option<ProductId> {
    if let Some(&product_step) = ctx.pdef_to_product.get(&item_ref) {
        if let Some(&pid) = ctx.product_arena_map.get(&product_step) {
            return Some(pid);
        }
    }
    if let Some(&pid) = ctx.product_arena_map.get(&item_ref) {
        return Some(pid);
    }
    None
}
