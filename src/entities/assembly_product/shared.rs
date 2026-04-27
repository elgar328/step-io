//! Shared helpers for the `assembly_product` entity handlers.

use crate::ir::attr::read_string_or_unset;
use crate::ir::error::ConvertError;
use crate::parser::entity::Attribute;

/// Read an optional text attribute that should be `None` for both `$`
/// (Unset) and the empty string `''`. Used for `PRODUCT_CATEGORY.description`
/// and similar metadata fields where many producers emit `''` to mean
/// "no description" rather than the strictly correct `$`.
pub(crate) fn optional_text(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Option<String>, ConvertError> {
    let raw = read_string_or_unset(attrs, index, entity_id, field_name)?;
    if raw.is_empty() {
        Ok(None)
    } else {
        Ok(Some(raw.to_owned()))
    }
}

/// Mirror of [`optional_text`] for the writer side: `Some(s)` becomes
/// `Attribute::String(s)`, `None` becomes `Attribute::Unset`.
pub(crate) fn optional_text_attr(value: Option<String>) -> Attribute {
    value.map_or(Attribute::Unset, Attribute::String)
}
