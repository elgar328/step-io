use super::arena::Arena;
use super::assembly::AssemblyTree;
use super::geometry::{
    Axis1Placement, Axis2Placement2d, Axis2Placement3d, Curve, Curve2d, Direction2, Direction3,
    Point2, Point3, Surface,
};
use super::id::Placement3dId;
use super::topology::{Edge, Face, Shell, Solid, Vertex, Wire};
use crate::parser::schema::StepSchema;

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

/// Part 21 implementation level (e.g. `"2;1"`). Guaranteed non-empty.
///
/// The Part 21 spec requires `FILE_DESCRIPTION.implementation_level` to be
/// a non-empty string identifying the serialization level (today virtually
/// all files use `"2;1"`). Wrapping this as a newtype makes it impossible
/// to construct a `FileHeader` with an empty `implementation_level` by
/// accident. Format validation (the `"N;N"` shape) is intentionally not
/// enforced so future Part 21 editions can introduce new values without
/// breaking older consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplementationLevel(String);

impl ImplementationLevel {
    /// Part 21 ed1 standard, `"2;1"` — used by virtually all files.
    #[must_use]
    pub fn v2_1() -> Self {
        Self("2;1".into())
    }

    /// Lift a `String` to `ImplementationLevel`; returns `None` for empty
    /// input.
    #[must_use]
    pub fn try_from_string(s: String) -> Option<Self> {
        if s.is_empty() { None } else { Some(Self(s)) }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ImplementationLevel {
    fn default() -> Self {
        Self::v2_1()
    }
}

/// HEADER-section metadata preserved from the source STEP file.
///
/// The four fields that Part 21 forbids from being literal-empty
/// (`description`, `implementation_level`, `author`, `organization`)
/// use type-level guarantees ([`NonEmptyStringList`] / [`ImplementationLevel`])
/// so that constructing a spec-violating `FileHeader` is impossible at
/// the type system level. The remaining fields are plain `String` because
/// Part 21 accepts empty strings there.
///
/// On [`StepModel`], the `header` field is `Option<FileHeader>`:
/// - `None` means "synthetic IR — writer substitutes step-io's signature"
/// - `Some(_)` means reader captured these from the source file (or user
///   supplied them) and writer emits verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileHeader {
    pub description: NonEmptyStringList,
    pub implementation_level: ImplementationLevel,
    pub name: String,
    pub time_stamp: String,
    pub author: NonEmptyStringList,
    pub organization: NonEmptyStringList,
    pub preprocessor_version: String,
    pub originating_system: String,
    pub authorization: String,
}

/// The complete result of converting a STEP file into typed IR.
#[derive(Debug, Clone, Default)]
pub struct StepModel {
    pub geometry: GeometryPool,
    pub topology: TopologyPool,
    /// Units declared in the STEP file's `GLOBAL_UNIT_ASSIGNED_CONTEXT`.
    /// `None` when the file has no such context or uses unsupported units
    /// (e.g. `CONVERSION_BASED_UNIT` for inches).
    pub units: Option<UnitContext>,
    /// Assembly tree. `None` when the STEP file contains no `PRODUCT`
    /// entities (single-part files). Phase A populates `products` but
    /// leaves `AssemblyTree.root` as `None`; Phase B resolves the root
    /// and wires instances.
    pub assembly: Option<AssemblyTree>,
    /// AP schema this model targets, including the raw `FILE_SCHEMA` text
    /// when preserved from a source file. `StepSchema::Known { raw: None }`
    /// marks synthetic IR — the writer emits a canonical string for the
    /// `class`; `StepSchema::Known { raw: Some(_) }` or
    /// `StepSchema::Unknown { raw }` carry the original text, which the
    /// writer emits verbatim. Defaults to canonical AP214 IS.
    pub schema: StepSchema,
    /// HEADER-section metadata preserved from the source file. `None` for
    /// synthetic IR (writer substitutes a step-io-branded signature);
    /// `Some(_)` is emitted verbatim on round-trip so author / organisation /
    /// timestamp / description aren't overwritten with step-io's defaults.
    pub header: Option<FileHeader>,
}

/// Arena-based storage for all topology objects.
#[derive(Debug, Clone, Default)]
pub struct TopologyPool {
    pub solids: Arena<Solid>,
    pub shells: Arena<Shell>,
    pub faces: Arena<Face>,
    pub wires: Arena<Wire>,
    pub edges: Arena<Edge>,
    pub vertices: Arena<Vertex>,
}

/// Arena-based storage for all geometry objects.
///
/// 2D arenas (`points_2d`, `directions_2d`, `curves_2d`) carry PCURVE
/// parametric-space geometry. They are empty for files without PCURVE
/// content.
#[derive(Debug, Clone, Default)]
pub struct GeometryPool {
    pub surfaces: Arena<Surface>,
    pub curves: Arena<Curve>,
    pub points: Arena<Point3>,
    pub directions: Arena<Direction3>,
    pub placements: Arena<Axis2Placement3d>,
    pub placements_1d: Arena<Axis1Placement>,
    pub points_2d: Arena<Point2>,
    pub directions_2d: Arena<Direction2>,
    pub curves_2d: Arena<Curve2d>,
    pub placements_2d: Arena<Axis2Placement2d>,
    /// Caches the arena id of a single identity `AXIS2_PLACEMENT_3D` for kernel
    /// callers that repeatedly request one via [`GeometryPool::identity_placement`].
    /// The reader never touches this cache — it pushes every on-disk placement
    /// as a distinct entry to stay loyal to the source file.
    identity_placement_cache: Option<Placement3dId>,
}

impl GeometryPool {
    /// Lazy singleton identity placement for kernel callers.
    ///
    /// Returns a [`Placement3dId`] pointing to an identity `AXIS2_PLACEMENT_3D`
    /// (`location = (0, 0, 0)`, `axis = None`, `ref_direction = None`). Repeat
    /// calls return the cached id so N products sharing an identity reference
    /// frame collapse to a single arena entry — and, thanks to the writer's
    /// placement cache, a single `#N` in the emitted STEP file.
    pub fn identity_placement(&mut self) -> Placement3dId {
        if let Some(id) = self.identity_placement_cache {
            return id;
        }
        let origin = self.points.push(Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
        let id = self.placements.push(Axis2Placement3d {
            location: origin,
            axis: None,
            ref_direction: None,
        });
        self.identity_placement_cache = Some(id);
        id
    }
}

/// Units declared in the STEP file's HEADER section.
///
/// The IR preserves original units — numeric values are **not** normalized.
/// Kernel adapters inspect `UnitContext` and convert if needed.
///
/// `length_uncertainty` is `Some` when the source file carried a
/// `distance_accuracy_value` via `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT`,
/// `None` otherwise. The value is in the source's length unit (mm / inch
/// / ...) — no normalization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitContext {
    pub length: LengthUnit,
    pub plane_angle: AngleUnit,
    pub solid_angle: SolidAngleUnit,
    pub length_uncertainty: Option<f64>,
    /// `true` when the source file wrapped the length unit in
    /// `CONVERSION_BASED_UNIT` even though the unit is SI (e.g. ABC tier
    /// emits `CBU('METRE', 1.0, base=METRE)` instead of plain SI METRE).
    /// Writer re-emits the CBU wrapper when set; non-SI units (Inch / Foot)
    /// always emit CBU regardless of this flag.
    pub length_cbu_wrapped: bool,
    /// `true` when the source file wrapped the plane-angle unit (Radian)
    /// in a self-conversion `CONVERSION_BASED_UNIT`. Degree is non-SI and
    /// always emits CBU regardless of this flag.
    pub plane_angle_cbu_wrapped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthUnit {
    Millimetre,
    Metre,
    Centimetre,
    Inch,
    Foot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AngleUnit {
    Radian,
    Degree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidAngleUnit {
    Steradian,
}

#[cfg(test)]
mod tests {
    use super::{FileHeader, ImplementationLevel, NonEmptyStringList};

    #[test]
    fn non_empty_string_list_try_from_empty_vec_is_none() {
        assert!(NonEmptyStringList::try_from_vec(vec![]).is_none());
    }

    #[test]
    fn non_empty_string_list_try_from_populated_vec_is_some() {
        let v = vec!["a".into(), "b".into()];
        let nel = NonEmptyStringList::try_from_vec(v).expect("non-empty");
        assert_eq!(nel.len(), 2);
        assert_eq!(nel.as_slice()[0], "a");
    }

    #[test]
    fn non_empty_string_list_default_is_single_empty_string() {
        let nel = NonEmptyStringList::default();
        assert_eq!(nel.len(), 1);
        assert_eq!(nel.as_slice()[0], "");
    }

    #[test]
    fn implementation_level_try_from_empty_is_none() {
        assert!(ImplementationLevel::try_from_string(String::new()).is_none());
    }

    #[test]
    fn implementation_level_try_from_non_empty_is_some() {
        let il = ImplementationLevel::try_from_string("2;1".into()).expect("non-empty");
        assert_eq!(il.as_str(), "2;1");
    }

    #[test]
    fn implementation_level_default_is_v2_1() {
        assert_eq!(ImplementationLevel::default().as_str(), "2;1");
    }

    #[test]
    fn file_header_default_passes_spec_constraints() {
        // Layer 1 invariant: FileHeader::default() must produce spec-compliant
        // values for the four Part 21 fields that forbid literal-empty.
        let h = FileHeader::default();
        assert_eq!(h.description.len(), 1);
        assert_eq!(h.author.len(), 1);
        assert_eq!(h.organization.len(), 1);
        assert!(!h.implementation_level.as_str().is_empty());
    }
}
