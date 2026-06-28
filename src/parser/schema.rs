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

/// AP family recognised from `FILE_SCHEMA`. `Other` = step-io did not
/// recognise the schema name (the raw text is still preserved in [`SchemaId`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApFamily {
    Ap203,
    #[default]
    Ap214,
    Ap242,
    Other,
}

/// ISO publication stage of a schema edition.
///
/// `edition` and `stage` are independent axes: a single AP edition passes
/// through `Cd` → `Dis` → `Is`. Use `stage == Is` (not the presence of an
/// edition) to test whether the file declares a published standard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stage {
    /// International Standard (published).
    Is,
    /// Draft International Standard.
    Dis,
    /// Committee Draft.
    Cd,
    /// Stage could not be determined from the `FILE_SCHEMA`.
    #[default]
    Unknown,
}

/// Structured identity of a file's `FILE_SCHEMA`: AP family, edition, and
/// stage, plus the raw text.
///
/// Derived by [`identify_schema`] from the `FILE_SCHEMA` descriptor — the
/// schema-version token `{ ... 10303 <part> <version> ... }` is parsed
/// generically (whitespace/arity tolerant) and mapped through the
/// per-family catalog; an unrecognised name falls back to
/// [`ApFamily::Other`] and an unrecognised version to `edition: None`, so
/// future editions are still recognised by family.
///
/// - `raw: Some(_)` — preserved verbatim from a source file (read path);
///   byte-exact round-trip relies on this.
/// - `raw: None` — a synthetic model (kernel-built); the writer supplies a
///   canonical `FILE_SCHEMA` for the chosen target.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SchemaId {
    pub family: ApFamily,
    /// AP edition (1, 2, 3, …); `None` when the token does not pin it
    /// (e.g. AP214 CD/DIS, or an unrecognised AP242 module version).
    pub edition: Option<u8>,
    pub stage: Stage,
    pub raw: Option<NonEmptyStringList>,
}

impl SchemaId {
    /// A synthetic identity with no preserved raw text (kernel-built model).
    #[must_use]
    pub fn synthetic(family: ApFamily, edition: Option<u8>, stage: Stage) -> Self {
        Self {
            family,
            edition,
            stage,
            raw: None,
        }
    }

    /// Whether step-io recognised the AP family.
    #[must_use]
    pub fn is_recognized(&self) -> bool {
        self.family != ApFamily::Other
    }

    /// Preserved raw `FILE_SCHEMA` text, if any (`None` for synthetic).
    #[must_use]
    pub fn raw(&self) -> Option<&NonEmptyStringList> {
        self.raw.as_ref()
    }
}

impl std::fmt::Display for SchemaId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fam = match self.family {
            ApFamily::Ap203 => "AP203",
            ApFamily::Ap214 => "AP214",
            ApFamily::Ap242 => "AP242",
            ApFamily::Other => return write!(f, "unrecognized schema"),
        };
        let stage = match self.stage {
            Stage::Is => "IS",
            Stage::Dis => "DIS",
            Stage::Cd => "CD",
            Stage::Unknown => "?",
        };
        match self.edition {
            Some(e) => write!(f, "{fam} ed{e} ({stage})"),
            None => write!(f, "{fam} ed? ({stage})"),
        }
    }
}

/// Identify the [`SchemaId`] from the raw `FILE_SCHEMA` string list.
///
/// The schema-version token is parsed generically and mapped via the catalog
/// (`internal/SCHEMA_IDENTIFIER_CATALOG.md`); names without a usable token
/// fall back to family recognition. The raw text is always preserved.
#[must_use]
pub fn identify_schema(file_schema: &[String]) -> SchemaId {
    let Some(raw) = NonEmptyStringList::try_from_vec(file_schema.to_vec()) else {
        // Spec violation: FILE_SCHEMA is LIST[1:?]. Lenient recovery.
        return SchemaId::default();
    };
    let (family, edition, stage) = classify(file_schema);
    SchemaId {
        family,
        edition,
        stage,
        raw: Some(raw),
    }
}

/// Classify the `FILE_SCHEMA` list. Returns the first entry that identifies a
/// family; `(Other, None, Unknown)` if none match.
fn classify(file_schema: &[String]) -> (ApFamily, Option<u8>, Stage) {
    for s in file_schema {
        if let Some(hit) = classify_one(&s.to_uppercase()) {
            return hit;
        }
    }
    (ApFamily::Other, None, Stage::Unknown)
}

/// Classify a single (already upper-cased) `FILE_SCHEMA` descriptor. Token
/// parsing is primary (pins the edition); the name fallback handles
/// descriptors without a usable `{ ... }` token.
fn classify_one(upper: &str) -> Option<(ApFamily, Option<u8>, Stage)> {
    if let Some((part, version)) = parse_token(upper) {
        match part {
            214 => {
                use std::cmp::Ordering;
                let (ed, stage) = match version.cmp(&0) {
                    Ordering::Less => (None, Stage::Cd),
                    Ordering::Equal => (None, Stage::Dis),
                    Ordering::Greater => (u8::try_from(version).ok(), Stage::Is),
                };
                return Some((ApFamily::Ap214, ed, stage));
            }
            403 | 203 => {
                let (ed, stage) = if version >= 1 {
                    (u8::try_from(version).ok(), Stage::Is)
                } else {
                    (None, Stage::Unknown)
                };
                return Some((ApFamily::Ap203, ed, stage));
            }
            442 => {
                // AP242: the module (ISO/TS 10303-442) version is non-linear
                // with the AP242 edition, and the `1 0` prefix does not mark
                // the stage — so map version → (edition, stage) explicitly.
                let (ed, stage) = match version {
                    1 => (Some(1), Stage::Is),
                    2 => (Some(2), Stage::Dis),
                    3 => (Some(2), Stage::Is),
                    4 => (Some(3), Stage::Is),
                    _ => (None, Stage::Unknown),
                };
                return Some((ApFamily::Ap242, ed, stage));
            }
            _ => {} // unknown part — fall through to name matching
        }
    }
    // Name fallback (no usable token).
    if upper.contains("AUTOMOTIVE_DESIGN_CC1") || upper.contains("AUTOMOTIVE_DESIGN_CC2") {
        return Some((ApFamily::Ap214, None, Stage::Cd));
    }
    if upper.contains("AUTOMOTIVE_DESIGN") {
        return Some((ApFamily::Ap214, None, Stage::Unknown));
    }
    if upper.contains("AP242_MANAGED_MODEL_BASED_3D_ENGINEERING") {
        return Some((ApFamily::Ap242, None, Stage::Is));
    }
    if upper.contains("AP203_CONFIGURATION_CONTROLLED") {
        // Modular ed2 long form without a token — edition not pinnable.
        return Some((ApFamily::Ap203, None, Stage::Is));
    }
    if upper.contains("CONFIG_CONTROL_DESIGN")
        || upper.contains("CONFIGURATION_CONTROLLED_3D_DESIGN")
    {
        // AP203 ed1 short/long form (no token).
        return Some((ApFamily::Ap203, Some(1), Stage::Is));
    }
    None
}

/// Parse the schema-version token `{ ... 10303 <part> <version> ... }`.
///
/// Whitespace- and arity-tolerant: splits the `{ ... }` content on
/// whitespace, finds the exact integer token `10303`, and returns the next
/// two integers `(part, version)` (`version` may be negative). Returns `None`
/// when there is no brace group or no `10303` marker.
fn parse_token(upper: &str) -> Option<(i64, i64)> {
    let start = upper.find('{')?;
    let rel_end = upper[start..].find('}')?;
    let inner = &upper[start + 1..start + rel_end];
    let toks: Vec<&str> = inner.split_whitespace().collect();
    let pos = toks.iter().position(|t| *t == "10303")?;
    let part: i64 = toks.get(pos + 1)?.parse().ok()?;
    let version: i64 = toks.get(pos + 2)?.parse().ok()?;
    Some((part, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(input: &str) -> SchemaId {
        identify_schema(&[input.into()])
    }

    // --- AP214 (version = edition, linear) ---
    #[test]
    fn ap214_is_editions() {
        assert_eq!(
            (
                id("AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }").family,
                id("AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }").edition,
                id("AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }").stage
            ),
            (ApFamily::Ap214, Some(1), Stage::Is),
        );
        let e2 = id("AUTOMOTIVE_DESIGN { 1 0 10303 214 2 1 1 }");
        assert_eq!(
            (e2.family, e2.edition, e2.stage),
            (ApFamily::Ap214, Some(2), Stage::Is)
        );
        let e3 = id("AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }");
        assert_eq!(
            (e3.family, e3.edition, e3.stage),
            (ApFamily::Ap214, Some(3), Stage::Is)
        );
    }

    #[test]
    fn ap214_cd_dis() {
        let cc2 = id("AUTOMOTIVE_DESIGN_CC2 { 1 2 10303 214 -1 1 5 4 }");
        assert_eq!(
            (cc2.family, cc2.edition, cc2.stage),
            (ApFamily::Ap214, None, Stage::Cd)
        );
        let cc1 = id("AUTOMOTIVE_DESIGN_CC1 { 1 2 10303 214 -1 1 3 2 }");
        assert_eq!(
            (cc1.family, cc1.edition, cc1.stage),
            (ApFamily::Ap214, None, Stage::Cd)
        );
        let dis = id("AUTOMOTIVE_DESIGN { 1 2 10303 214 0 1 1 1 }");
        assert_eq!(
            (dis.family, dis.edition, dis.stage),
            (ApFamily::Ap214, None, Stage::Dis)
        );
    }

    #[test]
    fn ap214_bare_name_and_whitespace_variants() {
        let bare = id("AUTOMOTIVE_DESIGN");
        assert_eq!(
            (bare.family, bare.edition, bare.stage),
            (ApFamily::Ap214, None, Stage::Unknown)
        );
        let cc2_bare = id("AUTOMOTIVE_DESIGN_CC2");
        assert_eq!(
            (cc2_bare.family, cc2_bare.stage),
            (ApFamily::Ap214, Stage::Cd)
        );
        // No-space and 7-number arity variants both parse to ed3 IS.
        let nospace = id("AUTOMOTIVE_DESIGN {1 0 10303 214 3 1 1}");
        assert_eq!(
            (nospace.family, nospace.edition),
            (ApFamily::Ap214, Some(3))
        );
        let seven = id("AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 1 }");
        assert_eq!((seven.family, seven.edition), (ApFamily::Ap214, Some(3)));
    }

    // --- AP242 (version != edition, non-linear) ---
    #[test]
    fn ap242_version_to_edition_nonlinear() {
        let cases = [
            (1, Some(1), Stage::Is),
            (2, Some(2), Stage::Dis),
            (3, Some(2), Stage::Is),
            (4, Some(3), Stage::Is),
        ];
        for (v, ed, st) in cases {
            let s = id(&format!(
                "AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF {{ 1 0 10303 442 {v} 1 4 }}"
            ));
            assert_eq!(
                (s.family, s.edition, s.stage),
                (ApFamily::Ap242, ed, st),
                "version {v}"
            );
        }
    }

    #[test]
    fn ap242_unknown_version_falls_back_by_name() {
        // ed4-style: unknown module version → recognised as AP242, edition unknown.
        let s = id("AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 5 1 4 }");
        assert_eq!(
            (s.family, s.edition, s.stage),
            (ApFamily::Ap242, None, Stage::Unknown)
        );
    }

    #[test]
    fn ap242_trailing_dot_and_no_space() {
        // Real FreeCAD output: trailing `.` after name, no space after `{`.
        let s = id("AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF. {1 0 10303 442 1 1 4 }");
        assert_eq!(
            (s.family, s.edition, s.stage),
            (ApFamily::Ap242, Some(1), Stage::Is)
        );
    }

    // --- AP203 ---
    #[test]
    fn ap203_ed1_short_form() {
        let s = id("CONFIG_CONTROL_DESIGN");
        assert_eq!(
            (s.family, s.edition, s.stage),
            (ApFamily::Ap203, Some(1), Stage::Is)
        );
    }

    #[test]
    fn ap203_ed2_long_form_and_part_variant() {
        let e2 = id(
            "AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF { 1 0 10303 403 2 1 2 }",
        );
        assert_eq!(
            (e2.family, e2.edition, e2.stage),
            (ApFamily::Ap203, Some(2), Stage::Is)
        );
        // PART variant: AP203 long form carrying part `203` instead of `403` (seen in corpus).
        let p203 = id(
            "AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF { 1 0 10303 203 3 1 4 }",
        );
        assert_eq!(p203.family, ApFamily::Ap203);
    }

    #[test]
    fn ap203_two_element_list_first_matches() {
        let s = identify_schema(&[
            "CONFIG_CONTROL_DESIGN".into(),
            "SHAPE_APPEARANCE_LAYER_MIM".into(),
        ]);
        assert_eq!(s.family, ApFamily::Ap203);
        assert_eq!(s.raw().map(NonEmptyStringList::len), Some(2));
    }

    // --- fallback / edge ---
    #[test]
    fn unrecognized_is_other_with_raw() {
        let s = id("SOMETHING_ELSE");
        assert_eq!(s.family, ApFamily::Other);
        assert!(!s.is_recognized());
        assert_eq!(
            s.raw().map(|r| r.as_slice()[0].as_str()),
            Some("SOMETHING_ELSE")
        );
    }

    #[test]
    fn empty_list_falls_back_to_default() {
        // Spec violation (empty FILE_SCHEMA) → lenient default.
        assert_eq!(identify_schema(&[]), SchemaId::default());
    }

    #[test]
    fn raw_is_always_preserved_on_read() {
        let s = id("AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }");
        assert_eq!(
            s.raw().map(|r| r.as_slice()[0].as_str()),
            Some("AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }")
        );
    }

    #[test]
    fn display_formats() {
        assert_eq!(
            id("AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }")
                .to_string(),
            "AP242 ed2 (IS)"
        );
        assert_eq!(id("AUTOMOTIVE_DESIGN").to_string(), "AP214 ed? (?)");
        assert_eq!(id("SOMETHING_ELSE").to_string(), "unrecognized schema");
    }
}
