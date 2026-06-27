/// A Part 21 `LIST[1:?] OF STRING` — guaranteed to hold at least one element.
///
/// STEP's `FILE_DESCRIPTION.description`, `FILE_NAME.author`, and
/// `FILE_NAME.organization` fields are typed `LIST[1:?] OF STRING`; an empty
/// list is a spec violation. Encoding that constraint at the type level
/// prevents construction of spec-violating `FileHeader` values: any attempt
/// to build a `NonEmptyStringList` from an empty `Vec<String>` returns
/// `None` rather than an invalid value.
///
/// STEP convention for "no meaningful content" is a single-element list
/// holding `""`, which is what [`NonEmptyStringList::default`] produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyStringList(Vec<String>);

impl NonEmptyStringList {
    /// A single-element list. Use `single(String::new())` for the
    /// spec-compliant "no content" form `('')`.
    #[must_use]
    pub fn single(s: String) -> Self {
        Self(vec![s])
    }

    /// Lift a `Vec<String>` to `NonEmptyStringList`; returns `None` for an
    /// empty input.
    #[must_use]
    pub fn try_from_vec(v: Vec<String>) -> Option<Self> {
        if v.is_empty() { None } else { Some(Self(v)) }
    }

    #[must_use]
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    pub fn iter(&self) -> std::slice::Iter<'_, String> {
        self.0.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        // Invariant: never empty. Provided for `clippy::len_without_is_empty`.
        false
    }

    pub fn push(&mut self, s: String) {
        self.0.push(s);
    }
}

impl Default for NonEmptyStringList {
    /// Single empty-string element (`[""]`) — the STEP convention for
    /// "no meaningful content" while remaining spec-compliant.
    fn default() -> Self {
        Self::single(String::new())
    }
}

impl<'a> IntoIterator for &'a NonEmptyStringList {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// step-io's semantic classification axis for `FILE_SCHEMA`.
///
/// Independent of the raw text — two files can share the same
/// [`SchemaClass`] yet carry different whitespace, edition markers, or MIM
/// Long Form names. Used by the per-schema writer to pick APD metadata and
/// canonical `FILE_SCHEMA` strings for a synthetic model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SchemaClass {
    Ap203,
    Ap214Cd,
    Ap214Dis,
    /// AP214 IS is the most widely produced flavour today and serves as the
    /// writer default when the model doesn't carry an explicit schema choice.
    #[default]
    Ap214Is,
    Ap242Dis,
    /// Placeholder — no fixture available yet, [`identify_schema`] never
    /// returns this. Kept in the enum for future expansion.
    Ap242Is,
}

/// A file's `FILE_SCHEMA` as carried by the model.
///
/// Co-locates the semantic classification (`class`) and the raw text
/// (`raw`) in a single type so that writer round-trip is byte-exact and
/// invalid combinations are impossible to construct.
///
/// # Invariants (enforced by the type system)
///
/// - `Unknown { raw }` holds a non-`Option` raw list — there is no
///   "unknown schema without any raw text" state.
/// - `Known { raw: None }` means a synthetic model; the writer emits a
///   canonical string derived from `class`. `raw: Some(_)` means the
///   text was preserved from a source file or supplied by the user;
///   the writer emits it verbatim.
/// - Swapping `class` on a `Known` value automatically drops the
///   preserved raw text (via variant re-assignment with
///   [`StepSchema::canonical`]), so "class changed but raw still
///   reflects the old class" can't happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepSchema {
    /// step-io recognised the schema family. `raw: None` = synthetic model;
    /// `raw: Some(_)` = preserved verbatim from a source file or user.
    Known {
        class: SchemaClass,
        raw: Option<NonEmptyStringList>,
    },
    /// step-io did not recognise the schema. The raw text is always
    /// retained so the writer can still emit the original `FILE_SCHEMA`;
    /// APD metadata falls back to AP214 IS conventions.
    Unknown { raw: NonEmptyStringList },
}

impl StepSchema {
    /// Build a synthetic schema with no preserved raw text. The writer
    /// will emit the canonical `FILE_SCHEMA` string for `class`.
    #[must_use]
    pub fn canonical(class: SchemaClass) -> Self {
        StepSchema::Known { class, raw: None }
    }

    /// Build a schema that preserves raw `FILE_SCHEMA` text alongside the
    /// recognised `class`. The writer emits `raw` verbatim.
    #[must_use]
    pub fn preserved(class: SchemaClass, raw: NonEmptyStringList) -> Self {
        StepSchema::Known {
            class,
            raw: Some(raw),
        }
    }

    /// Recognised classification, or `None` for [`StepSchema::Unknown`].
    #[must_use]
    pub fn class(&self) -> Option<SchemaClass> {
        match self {
            StepSchema::Known { class, .. } => Some(*class),
            StepSchema::Unknown { .. } => None,
        }
    }

    /// Preserved raw `FILE_SCHEMA` text, if any. `None` only for synthetic
    /// `Known { raw: None }` values.
    #[must_use]
    pub fn raw(&self) -> Option<&NonEmptyStringList> {
        match self {
            StepSchema::Known { raw, .. } => raw.as_ref(),
            StepSchema::Unknown { raw } => Some(raw),
        }
    }
}

impl Default for StepSchema {
    fn default() -> Self {
        Self::canonical(SchemaClass::default())
    }
}

/// Identify the [`StepSchema`] from the raw `FILE_SCHEMA` string list.
///
/// Matching is case-insensitive to tolerate rare non-standard files that
/// use lower-case schema names. The raw text is always preserved alongside
/// the classification — see [`StepSchema`] for invariants.
///
/// **Checking order matters** — AP203 is checked first because its keyword
/// is the most unambiguous; AP214 CD (with `_CC2` suffix) is checked
/// before DIS/IS to avoid false positives; AP242 is last.
#[must_use]
pub fn identify_schema(file_schema: &[String]) -> StepSchema {
    let raw_opt = NonEmptyStringList::try_from_vec(file_schema.to_vec());
    let class_opt = classify(file_schema);
    match (class_opt, raw_opt) {
        (Some(class), Some(raw)) => StepSchema::preserved(class, raw),
        // Matched a class but the input list was empty — impossible in
        // practice (classify only inspects non-empty strings), but handle
        // it symmetrically.
        (Some(class), None) => StepSchema::canonical(class),
        (None, Some(raw)) => StepSchema::Unknown { raw },
        // Spec violation (empty FILE_SCHEMA) — fall back to the default
        // synthetic schema rather than propagate the violation.
        (None, None) => StepSchema::default(),
    }
}

/// Classify the `FILE_SCHEMA` text against the official constants defined
/// in OCCT's `StepAP214_Protocol.cxx`. Returns `None` when no known
/// pattern matches.
fn classify(file_schema: &[String]) -> Option<SchemaClass> {
    for s in file_schema {
        let upper = s.to_uppercase();
        // Covers both the ed1 short form (`CONFIG_CONTROL_DESIGN`, as
        // emitted by OCCT/FreeCAD) and the ed2 modular long form
        // (`AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF`,
        // as emitted by CATIA, Creo, NX, Inventor, etc.).
        if upper.contains("CONFIG_CONTROL_DESIGN")
            || upper.contains("CONFIGURATION_CONTROLLED_3D_DESIGN")
        {
            return Some(SchemaClass::Ap203);
        }
    }
    for s in file_schema {
        let upper = s.to_uppercase();
        if upper.contains("AUTOMOTIVE_DESIGN_CC2") {
            return Some(SchemaClass::Ap214Cd);
        }
    }
    for s in file_schema {
        let upper = s.to_uppercase();
        if upper.contains("AUTOMOTIVE_DESIGN") {
            if upper.contains("{ 1 2 10303 214 0") {
                return Some(SchemaClass::Ap214Dis);
            }
            if upper.contains("{ 1 0 10303 214 1") {
                return Some(SchemaClass::Ap214Is);
            }
        }
    }
    for s in file_schema {
        let upper = s.to_uppercase();
        if upper.contains("AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF") {
            return Some(SchemaClass::Ap242Dis);
            // TODO: SchemaClass::Ap242Is — no fixture yet; add discrimination rule when available
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preserved_single(class: SchemaClass, s: &str) -> StepSchema {
        StepSchema::preserved(class, NonEmptyStringList::single(s.into()))
    }

    #[test]
    fn identify_ap203() {
        assert_eq!(
            identify_schema(&["CONFIG_CONTROL_DESIGN".into()]),
            preserved_single(SchemaClass::Ap203, "CONFIG_CONTROL_DESIGN"),
        );
    }

    #[test]
    fn identify_ap203_with_extra_mim() {
        let raw = NonEmptyStringList::try_from_vec(vec![
            "CONFIG_CONTROL_DESIGN".into(),
            "SHAPE_APPEARANCE_LAYER_MIM".into(),
        ])
        .expect("non-empty");
        assert_eq!(
            identify_schema(&[
                "CONFIG_CONTROL_DESIGN".into(),
                "SHAPE_APPEARANCE_LAYER_MIM".into(),
            ]),
            StepSchema::preserved(SchemaClass::Ap203, raw),
        );
    }

    #[test]
    fn identify_ap203_ed2_full_name() {
        let input = "AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF { 1 0 10303 403 2 1 2 }";
        assert_eq!(
            identify_schema(&[input.into()]),
            preserved_single(SchemaClass::Ap203, input),
        );
    }

    #[test]
    fn identify_ap203_ed2_double_space_variant() {
        // Observed in NIST AP203 fixtures: double space around the ISO
        // identifier and edition `3 1 4` (instead of `2 1 2`).
        let input = "AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF  { 1 0 10303 403 3 1 4}";
        assert_eq!(
            identify_schema(&[input.into()]),
            preserved_single(SchemaClass::Ap203, input),
        );
    }

    #[test]
    fn identify_ap203_ed1_full_express_name() {
        // AP203 ed1 full EXPRESS schema name without the `AP203_` prefix —
        // defensive case for files that emit the bare long form.
        let input = "CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES";
        assert_eq!(
            identify_schema(&[input.into()]),
            preserved_single(SchemaClass::Ap203, input),
        );
    }

    #[test]
    fn identify_ap214_cd() {
        let input = "AUTOMOTIVE_DESIGN_CC2 { 1 2 10303 214 -1 1 5 4 }";
        assert_eq!(
            identify_schema(&[input.into()]),
            preserved_single(SchemaClass::Ap214Cd, input),
        );
    }

    #[test]
    fn identify_ap214_dis() {
        let input = "AUTOMOTIVE_DESIGN { 1 2 10303 214 0 1 1 1 }";
        assert_eq!(
            identify_schema(&[input.into()]),
            preserved_single(SchemaClass::Ap214Dis, input),
        );
    }

    #[test]
    fn identify_ap214_is() {
        let input = "AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }";
        assert_eq!(
            identify_schema(&[input.into()]),
            preserved_single(SchemaClass::Ap214Is, input),
        );
    }

    #[test]
    fn identify_ap242_dis_with_trailing_dot() {
        // Real FreeCAD AP242 output has a trailing `.` after the schema name.
        let input = "AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF. {1 0 10303 442 1 1 4 }";
        assert_eq!(
            identify_schema(&[input.into()]),
            preserved_single(SchemaClass::Ap242Dis, input),
        );
    }

    #[test]
    fn identify_unknown_preserves_raw() {
        assert_eq!(
            identify_schema(&["SOMETHING_ELSE".into()]),
            StepSchema::Unknown {
                raw: NonEmptyStringList::single("SOMETHING_ELSE".into()),
            },
        );
    }

    #[test]
    fn identify_empty_schema_list_falls_back_to_default() {
        // Spec violation: FILE_SCHEMA is `LIST[1:?]`. Lenient recovery
        // returns the default synthetic schema rather than propagate.
        assert_eq!(identify_schema(&[]), StepSchema::default());
    }

    #[test]
    fn default_is_canonical_ap214_is() {
        assert_eq!(
            StepSchema::default(),
            StepSchema::canonical(SchemaClass::Ap214Is),
        );
    }

    #[test]
    fn canonical_raw_is_none() {
        let s = StepSchema::canonical(SchemaClass::Ap203);
        assert_eq!(s.class(), Some(SchemaClass::Ap203));
        assert!(s.raw().is_none());
    }

    #[test]
    fn preserved_exposes_raw_and_class() {
        let raw = NonEmptyStringList::single("X".into());
        let s = StepSchema::preserved(SchemaClass::Ap214Is, raw.clone());
        assert_eq!(s.class(), Some(SchemaClass::Ap214Is));
        assert_eq!(s.raw(), Some(&raw));
    }

    #[test]
    fn unknown_has_no_class_but_has_raw() {
        let raw = NonEmptyStringList::single("X".into());
        let s = StepSchema::Unknown { raw: raw.clone() };
        assert!(s.class().is_none());
        assert_eq!(s.raw(), Some(&raw));
    }
}
