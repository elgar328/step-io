//! plm-domain entity handlers. Phase plm-1a covers the Date/Time
//! primitives — the leaf entities used by the Date/Time assignment
//! chain that lands in plm-1b.

pub mod address;
pub mod application_context;
pub mod application_protocol_definition;
pub mod applied_approval_assignment;
pub mod applied_date_and_time_assignment;
pub mod applied_document_reference;
pub mod applied_external_identification_assignment;
pub mod applied_group_assignment;
pub mod applied_person_and_organization_assignment;
pub mod applied_security_classification_assignment;
pub mod approval;
pub mod approval_date_time;
pub mod approval_person_organization;
pub mod approval_role;
pub mod approval_status;
pub mod calendar_date;
pub mod cc_design_approval;
pub mod cc_design_date_and_time_assignment;
pub mod cc_design_person_and_organization_assignment;
pub mod cc_design_security_classification;
pub mod coordinated_universal_time_offset;
pub mod date_and_time;
pub mod date_time_role;
pub mod document;
pub mod document_file;
pub mod document_product_equivalence;
pub mod document_representation_type;
pub mod document_type;
pub mod external_source;
pub mod group;
pub mod identification_role;
pub mod local_time;
pub mod object_role;
pub mod organization;
pub mod person;
pub mod person_and_organization;
pub mod person_and_organization_role;
pub mod personal_address;
pub mod role_association;
pub mod security_classification;
pub mod security_classification_level;

use crate::ir::AddressData;
use crate::ir::ProductId;
use crate::ir::attr::read_optional_string;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;

/// Resolve a `date_time_item` / `approval_item` /
/// `security_classification_item` / `person_organization_item` SELECT
/// ref against step-io's product chain. The blueprints allow several
/// targets (`PRODUCT_DEFINITION`, `PRODUCT_DEFINITION_FORMATION` and its
/// `_WITH_SPECIFIED_SOURCE` subtype, plain `PRODUCT`, plus
/// classification/document targets we do not yet model). step-io models
/// only the product chain, so any of the three product-side variants
/// collapses to the underlying `ProductId`; everything else drops
/// silently. The helper name predates Approval / Security / P&O
/// adoption and is retained pending a rename phase.
pub(crate) fn resolve_date_time_item(ctx: &ReaderContext, item_ref: u64) -> Option<ProductId> {
    if let Some(pid) = ctx.product_of_pdef(item_ref) {
        return Some(pid);
    }
    if let Some(pid) = ctx.product_of_formation(item_ref) {
        return Some(pid);
    }
    if let Some(pid) = ctx.id_cache.get::<crate::ir::id::ProductId>(item_ref) {
        return Some(pid);
    }
    None
}

/// Read the 12 inherited `ADDRESS` `opt_string` fields starting at
/// `attrs[start]`. Shared by the `ADDRESS` and `PERSONAL_ADDRESS`
/// readers (the latter inherits these 12 fields before its own
/// `people` / `description`).
pub(super) fn read_address_data(
    attrs: &[Attribute],
    start: usize,
    entity_id: u64,
    entity_name: &'static str,
) -> Result<AddressData, ConvertError> {
    let _ = entity_name;
    Ok(AddressData {
        internal_location: read_optional_string(attrs, start, entity_id, "internal_location")?,
        street_number: read_optional_string(attrs, start + 1, entity_id, "street_number")?,
        street: read_optional_string(attrs, start + 2, entity_id, "street")?,
        postal_box: read_optional_string(attrs, start + 3, entity_id, "postal_box")?,
        town: read_optional_string(attrs, start + 4, entity_id, "town")?,
        region: read_optional_string(attrs, start + 5, entity_id, "region")?,
        postal_code: read_optional_string(attrs, start + 6, entity_id, "postal_code")?,
        country: read_optional_string(attrs, start + 7, entity_id, "country")?,
        facsimile_number: read_optional_string(attrs, start + 8, entity_id, "facsimile_number")?,
        telephone_number: read_optional_string(attrs, start + 9, entity_id, "telephone_number")?,
        electronic_mail_address: read_optional_string(
            attrs,
            start + 10,
            entity_id,
            "electronic_mail_address",
        )?,
        telex_number: read_optional_string(attrs, start + 11, entity_id, "telex_number")?,
    })
}

/// Write the 12 inherited `ADDRESS` `opt_string` fields into the
/// supplied attribute vector. Used by both `ADDRESS` and
/// `PERSONAL_ADDRESS` writers so the inherited shape stays in sync.
pub(super) fn write_address_data(attrs: &mut Vec<Attribute>, data: AddressData) {
    let push = |attrs: &mut Vec<Attribute>, v: Option<String>| {
        attrs.push(match v {
            Some(s) => Attribute::String(s),
            None => Attribute::Unset,
        });
    };
    push(attrs, data.internal_location);
    push(attrs, data.street_number);
    push(attrs, data.street);
    push(attrs, data.postal_box);
    push(attrs, data.town);
    push(attrs, data.region);
    push(attrs, data.postal_code);
    push(attrs, data.country);
    push(attrs, data.facsimile_number);
    push(attrs, data.telephone_number);
    push(attrs, data.electronic_mail_address);
    push(attrs, data.telex_number);
}
