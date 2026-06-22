//! HEADER block construction.
//!
//! `FILE_DESCRIPTION` and `FILE_NAME` are emitted verbatim from
//! `StepModel.header` when present (read-modify-write path); when absent
//! (`None`), step-io substitutes its own signature — an honest
//! "synthetic IR was authored by step-io" statement rather than a lie
//! attributing the output to any other source.
//!
//! `FILE_SCHEMA` is driven by `StepSchema`: when the variant carries a
//! preserved raw string list (either a `Known` with `raw: Some(_)` or an
//! `Unknown`) it is emitted verbatim for byte-exact round-trip; a
//! synthetic `Known { raw: None }` falls back to the canonical string for
//! its `SchemaClass`. `SchemaClass::Ap242Is` shares AP214 IS's canonical
//! descriptor because no fixture guides a distinct one yet.

use super::entity::HeaderEntity;
use crate::ir::model::{FileHeader, ImplementationLevel, NonEmptyStringList, StepModel};
use crate::parser::entity::Attribute;
use crate::parser::schema::{SchemaClass, StepSchema};

pub(super) fn header_for(model: &StepModel, target: super::SchemaTarget) -> Vec<HeaderEntity> {
    let header_values = model
        .metadata
        .header
        .clone()
        .unwrap_or_else(step_io_signature_header);
    vec![
        file_description_entity(&header_values),
        file_name_entity(&header_values),
        HeaderEntity {
            name: "FILE_SCHEMA".into(),
            attrs: vec![Attribute::List(target_schema_strings(
                &model.metadata.schema,
                target,
            ))],
        },
    ]
}

/// `FILE_SCHEMA` strings for the chosen target: a non-`Universal` target emits
/// that target's baked descriptor(s); `Universal` keeps the model's own schema
/// (preserved raw, or canonical for synthetic IR).
fn target_schema_strings(schema: &StepSchema, target: super::SchemaTarget) -> Vec<Attribute> {
    match crate::early::profile::SchemaProfile::for_target(target).file_schema() {
        Some(strs) => strs
            .iter()
            .map(|s| Attribute::String((*s).to_string()))
            .collect(),
        None => file_schema_strings(schema),
    }
}

/// step-io's signature when producing synthetic output (IR had `header: None`).
/// Honestly attributes authorship rather than claiming to be another source.
fn step_io_signature_header() -> FileHeader {
    FileHeader {
        description: NonEmptyStringList::single("step-io output".into()),
        implementation_level: ImplementationLevel::v2_1(),
        name: "step-io".into(),
        time_stamp: "1970-01-01T00:00:00".into(),
        author: NonEmptyStringList::default(),
        organization: NonEmptyStringList::default(),
        preprocessor_version: " ".into(),
        originating_system: "step-io".into(),
        authorization: String::new(),
    }
}

fn file_description_entity(h: &FileHeader) -> HeaderEntity {
    HeaderEntity {
        name: "FILE_DESCRIPTION".into(),
        attrs: vec![
            Attribute::List(string_list_to_attrs(&h.description)),
            Attribute::String(h.implementation_level.as_str().into()),
        ],
    }
}

fn file_name_entity(h: &FileHeader) -> HeaderEntity {
    HeaderEntity {
        name: "FILE_NAME".into(),
        attrs: vec![
            Attribute::String(h.name.clone()),
            Attribute::String(h.time_stamp.clone()),
            Attribute::List(string_list_to_attrs(&h.author)),
            Attribute::List(string_list_to_attrs(&h.organization)),
            Attribute::String(h.preprocessor_version.clone()),
            Attribute::String(h.originating_system.clone()),
            Attribute::String(h.authorization.clone()),
        ],
    }
}

fn string_list_to_attrs(list: &NonEmptyStringList) -> Vec<Attribute> {
    list.iter().map(|s| Attribute::String(s.clone())).collect()
}

fn file_schema_strings(schema: &StepSchema) -> Vec<Attribute> {
    match schema {
        // Synthetic IR: no preserved raw text, emit the canonical
        // string for the recognised class.
        StepSchema::Known { class, raw: None } => canonical_schema_strings(*class),
        // Preserved text — emit verbatim so byte-level differences
        // (whitespace, ed1 vs ed2 naming, trailing dots) survive round-trip.
        StepSchema::Known { raw: Some(raw), .. } | StepSchema::Unknown { raw } => {
            raw.iter().map(|s| Attribute::String(s.clone())).collect()
        }
    }
}

fn canonical_schema_strings(class: SchemaClass) -> Vec<Attribute> {
    match class {
        SchemaClass::Ap203 => vec![
            Attribute::String("CONFIG_CONTROL_DESIGN".into()),
            Attribute::String("SHAPE_APPEARANCE_LAYER_MIM".into()),
        ],
        SchemaClass::Ap214Cd => vec![Attribute::String(
            "AUTOMOTIVE_DESIGN_CC2 { 1 2 10303 214 -1 1 5 4 }".into(),
        )],
        SchemaClass::Ap214Dis => vec![Attribute::String(
            "AUTOMOTIVE_DESIGN { 1 2 10303 214 0 1 1 1 }".into(),
        )],
        SchemaClass::Ap242Dis => vec![Attribute::String(
            "AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF {1 0 10303 442 1 1 4 }".into(),
        )],
        SchemaClass::Ap214Is | SchemaClass::Ap242Is => vec![Attribute::String(
            "AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }".into(),
        )],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper: header for the Universal target (as-is, no projection).
    fn hf(model: &StepModel) -> Vec<HeaderEntity> {
        header_for(model, crate::writer::SchemaTarget::Universal)
    }

    fn model_for_schema(schema: StepSchema) -> StepModel {
        StepModel {
            metadata: crate::ir::FileMetadata {
                schema,
                ..Default::default()
            },
            ..StepModel::default()
        }
    }

    #[test]
    fn header_for_always_returns_three_entities_in_order() {
        for class in [
            SchemaClass::Ap203,
            SchemaClass::Ap214Cd,
            SchemaClass::Ap214Dis,
            SchemaClass::Ap214Is,
            SchemaClass::Ap242Dis,
        ] {
            let headers = hf(&model_for_schema(StepSchema::canonical(class)));
            assert_eq!(headers.len(), 3, "{class:?}");
            assert_eq!(headers[0].name, "FILE_DESCRIPTION");
            assert_eq!(headers[1].name, "FILE_NAME");
            assert_eq!(headers[2].name, "FILE_SCHEMA");
        }
    }

    #[test]
    fn ap214_is_canonical_schema_descriptor() {
        let headers = hf(&model_for_schema(StepSchema::canonical(
            SchemaClass::Ap214Is,
        )));
        let Attribute::List(inner) = &headers[2].attrs[0] else {
            panic!("expected list");
        };
        let Attribute::String(s) = &inner[0] else {
            panic!("expected string");
        };
        assert_eq!(s, "AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }");
    }

    #[test]
    fn ap203_canonical_schema_has_two_strings() {
        let headers = hf(&model_for_schema(StepSchema::canonical(SchemaClass::Ap203)));
        let Attribute::List(inner) = &headers[2].attrs[0] else {
            panic!("expected list");
        };
        assert_eq!(inner.len(), 2);
    }

    #[test]
    fn emits_preserved_raw_verbatim() {
        // AP203 ed2 long form — writer must emit it byte-for-byte rather
        // than normalise to the ed1 short form that would be the
        // canonical choice for `SchemaClass::Ap203`.
        let raw = NonEmptyStringList::single(
            "AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF \
             { 1 0 10303 403 3 1 4 }"
                .into(),
        );
        let schema = StepSchema::preserved(SchemaClass::Ap203, raw.clone());
        let headers = hf(&model_for_schema(schema));
        let Attribute::List(inner) = &headers[2].attrs[0] else {
            panic!("expected list");
        };
        assert_eq!(inner.len(), 1);
        let Attribute::String(s) = &inner[0] else {
            panic!("expected string");
        };
        assert_eq!(s, &raw.as_slice()[0]);
    }

    #[test]
    fn emits_canonical_when_raw_none() {
        // Synthetic `Known { raw: None }` must produce the canonical
        // ed1 short form for AP203.
        let headers = hf(&model_for_schema(StepSchema::canonical(SchemaClass::Ap203)));
        let Attribute::List(inner) = &headers[2].attrs[0] else {
            panic!("expected list");
        };
        assert_eq!(inner.len(), 2);
        let Attribute::String(first) = &inner[0] else {
            panic!()
        };
        let Attribute::String(second) = &inner[1] else {
            panic!()
        };
        assert_eq!(first, "CONFIG_CONTROL_DESIGN");
        assert_eq!(second, "SHAPE_APPEARANCE_LAYER_MIM");
    }

    #[test]
    fn unknown_emits_raw_verbatim() {
        // Regression guard for commit 435431e: Unknown schemas retain
        // their raw FILE_SCHEMA text rather than being rewritten to an
        // AP214 IS canonical.
        let raw =
            NonEmptyStringList::single("AP209_MULTIDISCIPLINARY_ANALYSIS_AND_DESIGN_MIM_LF".into());
        let headers = hf(&model_for_schema(StepSchema::Unknown { raw: raw.clone() }));
        let Attribute::List(inner) = &headers[2].attrs[0] else {
            panic!("expected list");
        };
        assert_eq!(inner.len(), 1);
        let Attribute::String(s) = &inner[0] else {
            panic!("expected string");
        };
        assert_eq!(s, &raw.as_slice()[0]);
    }

    #[test]
    fn ap242_is_still_falls_back_to_ap214_is() {
        let fallback = hf(&model_for_schema(StepSchema::canonical(
            SchemaClass::Ap242Is,
        )));
        let reference = hf(&model_for_schema(StepSchema::canonical(
            SchemaClass::Ap214Is,
        )));
        assert_eq!(fallback[2], reference[2]);
    }

    #[test]
    fn writes_step_io_signature_when_model_header_is_none() {
        let headers = hf(&model_for_schema(StepSchema::canonical(
            SchemaClass::Ap214Is,
        )));
        let Attribute::List(desc_inner) = &headers[0].attrs[0] else {
            panic!("expected list");
        };
        assert!(matches!(
            &desc_inner[0],
            Attribute::String(s) if s == "step-io output"
        ));
        assert!(matches!(
            &headers[1].attrs[0],
            Attribute::String(s) if s == "step-io"
        ));
    }

    #[test]
    fn writes_preserved_file_header_when_some() {
        let model = StepModel {
            metadata: crate::ir::FileMetadata {
                header: Some(FileHeader {
                    description: NonEmptyStringList::single("User Description".into()),
                    implementation_level: ImplementationLevel::v2_1(),
                    name: "user_part.step".into(),
                    time_stamp: "2024-08-15T12:34:56".into(),
                    author: NonEmptyStringList::single("Alice".into()),
                    organization: NonEmptyStringList::single("Acme".into()),
                    preprocessor_version: "CAD 2024".into(),
                    originating_system: "AcmeCAD".into(),
                    authorization: "manager".into(),
                }),
                ..Default::default()
            },
            ..StepModel::default()
        };
        let headers = hf(&model);
        let Attribute::List(desc) = &headers[0].attrs[0] else {
            panic!()
        };
        assert!(matches!(&desc[0], Attribute::String(s) if s == "User Description"));
        // FILE_NAME.name
        assert!(matches!(
            &headers[1].attrs[0],
            Attribute::String(s) if s == "user_part.step"
        ));
        // FILE_NAME.author
        let Attribute::List(author) = &headers[1].attrs[2] else {
            panic!()
        };
        assert!(matches!(&author[0], Attribute::String(s) if s == "Alice"));
    }

    #[test]
    fn step_io_signature_is_itself_spec_compliant() {
        // The signature helper must produce type-level non-empty values;
        // if any default falls back to an empty collection we break the
        // Layer 1 invariant. This guards regressions.
        let sig = step_io_signature_header();
        assert_eq!(sig.description.len(), 1);
        assert_eq!(sig.author.len(), 1);
        assert_eq!(sig.organization.len(), 1);
        assert!(!sig.implementation_level.as_str().is_empty());
    }
}
