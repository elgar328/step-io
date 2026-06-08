//! HEADER-section metadata extraction.
//!
//! Promotes `FILE_DESCRIPTION` and `FILE_NAME` entities from
//! [`EntityGraph.header`](crate::parser::entity::EntityGraph::header) into a
//! typed [`FileHeader`] on [`StepModel`](crate::ir::model::StepModel). If
//! the header is missing, structurally unexpected, or violates Part 21's
//! type constraints (empty `LIST[1:?]` fields or empty
//! `implementation_level`), the function returns `None` after pushing a
//! warning — the caller sets `model.header = None`, and the writer falls
//! back to step-io's synthetic signature.

use crate::ir::attr::{read_string, read_string_list};
use crate::ir::error::ConvertError;
use crate::ir::model::{FileHeader, ImplementationLevel, NonEmptyStringList};
use crate::parser::entity::{Attribute, RawEntity};

/// Read a `FILE_NAME` scalar `STRING` field.
///
/// [NS-filename-unset] grabcad (SO14/blower): a required Part 21 `FILE_NAME`
/// string (`''` denotes unspecified) is written `$` (Unset) → normalize `$`
/// to the empty string rather than discarding the whole header. See
/// `reader::nonstandard`.
fn read_header_string(
    attrs: &[Attribute],
    index: usize,
    pseudo_id: u64,
    field: &'static str,
    warnings: &mut Vec<ConvertError>,
) -> Result<String, ConvertError> {
    if matches!(attrs.get(index), Some(Attribute::Unset)) {
        warnings.push(ConvertError::NonStandardInput {
            field: format!("FILE_NAME.{field} (Unset)"),
            count: 1,
            normalized_to: "empty string".into(),
        });
        Ok(String::new())
    } else {
        Ok(read_string(attrs, index, pseudo_id, field)?.to_string())
    }
}

pub(super) fn extract_file_header(
    header: &[RawEntity],
    warnings: &mut Vec<ConvertError>,
) -> Option<FileHeader> {
    let fd = find_named(header, "FILE_DESCRIPTION");
    let fn_ = find_named(header, "FILE_NAME");
    let (Some(fd), Some(fn_)) = (fd, fn_) else {
        return None;
    };
    let parsed = (|| -> Result<FileHeader, ConvertError> {
        let description_vec = read_string_list(fd.attrs, 0, fd.pseudo_id, "description")?;
        let description = NonEmptyStringList::try_from_vec(description_vec).ok_or_else(|| {
            ConvertError::UnexpectedEntityForm {
                entity_id: fd.pseudo_id,
                detail: "FILE_DESCRIPTION.description must contain at least one element".into(),
            }
        })?;
        let impl_level_str = read_string(fd.attrs, 1, fd.pseudo_id, "implementation_level")?;
        let implementation_level = ImplementationLevel::try_from_string(impl_level_str.to_string())
            .ok_or_else(|| ConvertError::UnexpectedEntityForm {
                entity_id: fd.pseudo_id,
                detail: "FILE_DESCRIPTION.implementation_level must be non-empty".into(),
            })?;
        let name = read_header_string(fn_.attrs, 0, fn_.pseudo_id, "name", warnings)?;
        let time_stamp = read_header_string(fn_.attrs, 1, fn_.pseudo_id, "time_stamp", warnings)?;
        let author_vec = read_string_list(fn_.attrs, 2, fn_.pseudo_id, "author")?;
        let author = NonEmptyStringList::try_from_vec(author_vec).ok_or_else(|| {
            ConvertError::UnexpectedEntityForm {
                entity_id: fn_.pseudo_id,
                detail: "FILE_NAME.author must contain at least one element".into(),
            }
        })?;
        let organization_vec = read_string_list(fn_.attrs, 3, fn_.pseudo_id, "organization")?;
        let organization = NonEmptyStringList::try_from_vec(organization_vec).ok_or_else(|| {
            ConvertError::UnexpectedEntityForm {
                entity_id: fn_.pseudo_id,
                detail: "FILE_NAME.organization must contain at least one element".into(),
            }
        })?;
        let preprocessor_version = read_header_string(
            fn_.attrs,
            4,
            fn_.pseudo_id,
            "preprocessor_version",
            warnings,
        )?;
        let originating_system =
            read_header_string(fn_.attrs, 5, fn_.pseudo_id, "originating_system", warnings)?;
        let authorization =
            read_header_string(fn_.attrs, 6, fn_.pseudo_id, "authorization", warnings)?;
        Ok(FileHeader {
            description,
            implementation_level,
            name,
            time_stamp,
            author,
            organization,
            preprocessor_version,
            originating_system,
            authorization,
        })
    })();
    match parsed {
        Ok(h) => Some(h),
        Err(e) => {
            warnings.push(e);
            None
        }
    }
}

struct NamedEntity<'a> {
    pseudo_id: u64,
    attrs: &'a [crate::parser::entity::Attribute],
}

fn find_named<'a>(header: &'a [RawEntity], name: &str) -> Option<NamedEntity<'a>> {
    header.iter().find_map(|e| match e {
        RawEntity::Simple {
            id,
            name: n,
            attributes,
            ..
        } if n == name => Some(NamedEntity {
            pseudo_id: *id,
            attrs: attributes.as_slice(),
        }),
        _ => None,
    })
}
