//! Visualization data — `STYLED_ITEM` chain + colours + style metadata.
//!
//! Colours migrated to `Arena<Colour>` with `ColourId` references per the
//! ir.toml blueprint (`pool = "visualization"`, `enum_of = "colour"`).
//! Other styling types (`FillAreaStyle`, `SurfaceStyleRendering`, etc.)
//! remain tree-inline pending later phases of the blueprint migration.

use super::arena::Arena;
use super::id::{
    ColourId, CurveStyleId, DraughtingPreDefinedTextFontId, FoundedItemId, MeasureWithUnitId,
    Placement2dId, Placement3dId, PlanarExtentId, PointId, PreDefinedCurveFontId,
    PreDefinedMarkerId, PresentationStyleAssignmentId, StyledItemId, SurfaceStyleRenderingId,
    SymbolColourId, TextLiteralId, TextStyleForDefinedFontId,
};
use super::representation_item::RepresentationItemRef;
use super::shape_rep::Mdgpr;
use step_io_macros::StepSelect;

/// Top-level container for visualization data extracted from
/// `MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION` (MDGPR)
/// entities. Empty when the source file had no visualization chain or
/// when a kernel adapter built the IR without one.
///
/// `Mdgpr` itself lives in [`crate::ir::shape_rep`] per the ir.toml
/// blueprint (its arena is `representation`), but its container stays
/// here alongside the styled-item chain that the MDGPR wraps.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VisualizationPool {
    /// Top-level MDGPR roots — emit order preserved.
    pub mdgprs: Vec<Mdgpr>,
    /// `COLOUR` arena — one entry per source `COLOUR_RGB` or
    /// `DRAUGHTING_PRE_DEFINED_COLOUR`. Consumers reference by [`ColourId`].
    pub colours: Arena<Colour>,
    /// `SYMBOL_COLOUR` arena (phase symbol-colour). Wraps a `Colour`
    /// reference for use as a `symbol_style_select` SELECT member.
    /// Referenced by `SymbolStyle` in a future sub-phase.
    pub symbol_colours: Arena<SymbolColour>,
    /// `TEXT_STYLE_FOR_DEFINED_FONT` arena (phase text-style-font). Wraps
    /// a `Colour` ref for `character_style_select` SELECT use.
    pub text_styles_for_defined_font: Arena<TextStyleForDefinedFont>,
    /// `pre_defined_marker` enum arena (phase pre-defined-marker).
    pub pre_defined_markers: Arena<PreDefinedMarker>,
    /// `pre_defined_curve_font` arena per ir.toml — holds both the
    /// abstract self variant (`Plain`, corpus 0) and the
    /// `DRAUGHTING_PRE_DEFINED_CURVE_FONT` subtype. Referenced by
    /// [`CurveStyle::curve_font`] via [`PreDefinedCurveFontId`].
    pub pre_defined_curve_fonts: Arena<PreDefinedCurveFont>,
    /// `pre_defined_symbol` arena per ir.toml — holds the abstract self
    /// variant (`Plain`, corpus 0) and the `PRE_DEFINED_TERMINATOR_SYMBOL`
    /// subtype. No step-io consumer references this arena directly yet;
    /// the id newtype exists for blueprint symmetry and future leader /
    /// terminator handlers.
    pub pre_defined_symbols: Arena<PreDefinedSymbol>,
    /// `CURVE_STYLE` arena — referenced from `PresentationStyleAssignment`
    /// when a PSA carries curve styling alongside surface styling.
    pub curve_styles: Arena<CurveStyle>,
    /// `STYLED_ITEM` arena — referenced from `Mdgpr.items` by
    /// [`crate::ir::id::StyledItemId`]. Phase C migrates `STYLED_ITEM`
    /// out of the previously-inline tree storage so future
    /// `OverRidingStyledItem` / `ContextDependentOverRidingStyledItem`
    /// variants can reference existing entries through the same id.
    pub styled_items: Arena<StyledItem>,
    /// `PRESENTATION_STYLE_ASSIGNMENT` arena per the ir.toml blueprint
    /// (`arena = "presentation_style_assignment"`). `STYLED_ITEM` /
    /// `OVER_RIDING_STYLED_ITEM` consumers hold
    /// [`PresentationStyleAssignmentId`] refs into this arena so a PSA
    /// shared by multiple styled items round-trips as a single STEP entity
    /// instead of being duplicated per occurrence.
    pub presentation_style_assignments: Arena<PresentationStyleAssignment>,
    /// `surface_style_rendering` arena per ir.toml. Holds both the base
    /// `SURFACE_STYLE_RENDERING` (`Itself` variant) and its
    /// `SURFACE_STYLE_RENDERING_WITH_PROPERTIES` subtype as enum variants
    /// of the same id namespace. `SurfaceSideStyleEntry::Rendering` carries
    /// a [`SurfaceStyleRenderingId`] into this arena.
    pub surface_style_renderings: Arena<SurfaceStyleRendering>,
    /// `founded_item` arena per ir.toml (`enum_base`). Holds the AP214
    /// "founded item" subtype hierarchy as enum variants of one shared
    /// id namespace. Phase E1 covers `FillAreaStyle` and
    /// `SurfaceStyleFillArea`; `SurfaceSideStyle` and `SurfaceStyleUsage`
    /// land in E2. Unsupported source-side variants (`PointStyle`,
    /// `SurfaceStyleBoundary`, `SurfaceStyleParameterLine`, `SymbolStyle`,
    /// `ViewVolume`) are silently dropped at read.
    pub founded_items: Arena<FoundedItem>,
    /// `PRESENTATION_LAYER_ASSIGNMENT` arena per ir.toml
    /// (`pool = "visualization"`). Groups `STYLED_ITEM` entries into named
    /// display layers ("VISIBLE", "HIDDEN", numeric ids, ...). Top-level —
    /// no other entity references this arena, so the id newtype exists
    /// only for blueprint symmetry.
    pub presentation_layer_assignments: Arena<PresentationLayerAssignment>,
    /// `camera_model` `enum_base` arena. Phase camera-model-d3 fills the
    /// `CameraModelD3` variant only.
    pub camera_models: Arena<CameraModel>,
    /// `text_style` enum arena (phase text-style-box). Holds the base
    /// `TEXT_STYLE` (`Itself`, corpus 0) and its
    /// `TEXT_STYLE_WITH_BOX_CHARACTERISTICS` subtype variant.
    pub text_styles: Arena<TextStyle>,
    /// `text_literal` arena (phase text-literal). `representation_item`
    /// subtype carrying placement / font / alignment metadata for a
    /// drafting label.
    pub text_literals: Arena<TextLiteral>,
    /// `INVISIBILITY` arena (phase invisibility). Each entry marks a set
    /// of presentation entities (styled items, callouts, whole
    /// representations) as not to render. `CONTEXT_DEPENDENT_INVISIBILITY`
    /// subtype not modelled.
    pub invisibilities: Arena<Invisibility>,
    /// `presentation_representation` enum arena (phase pr-core) — covers
    /// `PRESENTATION_VIEW` + `PRESENTATION_AREA`. Stored on the
    /// visualization pool for ergonomic colocation with other
    /// presentation-domain entities even though the blueprint pool tag
    /// is `shape_rep`.
    pub presentation_representations: Arena<PresentationRepresentation>,
    /// `PRESENTATION_SET` arena (phase pr-core). Blueprint omits this
    /// entity; step-io carries a minimal `name`-only struct so
    /// `AREA_IN_SET.in_set` references resolve.
    pub presentation_sets: Arena<PresentationSet>,
    /// `AREA_IN_SET` arena (phase pr-size). Binds a `PresentationArea`
    /// representation to a `PresentationSet`.
    pub area_in_sets: Arena<AreaInSet>,
    /// `PRESENTATION_SIZE` arena (phase pr-size). 2D extent box paired
    /// with a `presentation_size_assignment_select` (`PresentationView` /
    /// `PresentationArea` / `AreaInSet`).
    pub presentation_sizes: Arena<PresentationSize>,
    /// `PRESENTED_ITEM_REPRESENTATION` arena (phase pr-item). Binds a
    /// presentation representation (or set) to a `presented_item` SELECT
    /// member.
    pub presented_item_representations: Arena<PresentedItemRepresentation>,
    /// `APPLIED_PRESENTED_ITEM` arena (phase pr-item). SET of
    /// `presented_item` SELECT members (step-io models only
    /// `ProductDefinition`).
    pub applied_presented_items: Arena<AppliedPresentedItem>,
    /// `composite_text` arena (phase text-literal). Groups two or more
    /// `text_or_character` elements (currently narrowed to
    /// [`TextLiteral`] references) into a composite drafting label.
    pub composite_texts: Arena<CompositeText>,
}

/// `PRESENTATION_LAYER_ASSIGNMENT(name, description, assigned_items)`.
/// `assigned_items` is the AP214 `layered_item` SELECT — a broad
/// `representation_item` reference. step-io models the `STYLED_ITEM`
/// target (the corpus-dominant variant); other source-side variants
/// (direct geometry refs, etc.) are silently dropped on read.
#[derive(Debug, Clone, PartialEq)]
pub struct PresentationLayerAssignment {
    pub name: String,
    pub description: String,
    pub assigned_items: Vec<PresentationLayerAssignmentItem>,
}

/// One element of [`PresentationLayerAssignment::assigned_items`].
/// Scoped to `STYLED_ITEM` today; future kernel adapters that produce
/// direct-geometry layered items can grow this enum.
#[derive(Debug, Clone, Copy, PartialEq, StepSelect)]
pub enum PresentationLayerAssignmentItem {
    StyledItem(StyledItemId),
}

/// `founded_item` enum arena per ir.toml. Each variant wraps a concrete
/// "founded item" subtype that the source `STYLED_ITEM` chain references
/// through a [`FoundedItemId`]. Phase E1 carries the inner pair
/// (`FillAreaStyle` and its wrapper `SurfaceStyleFillArea`); E2 adds the
/// outer pair (`SurfaceSideStyle` / `SurfaceStyleUsage`).
#[derive(Debug, Clone, PartialEq)]
pub enum FoundedItem {
    FillAreaStyle(FillAreaStyle),
    SurfaceStyleFillArea(SurfaceStyleFillArea),
    SurfaceSideStyle(SurfaceSideStyle),
    SurfaceStyleUsage(SurfaceStyleUsage),
    /// `VIEW_VOLUME` — a `founded_item` subtype referenced by `CAMERA_MODEL_D3`.
    ViewVolume(ViewVolume),
    /// `SYMBOL_STYLE` — a `founded_item` subtype carrying a
    /// [`SymbolColour`] reference (phase symbol-style).
    SymbolStyle(SymbolStyle),
    /// `POINT_STYLE` — a `founded_item` subtype carrying marker / size /
    /// colour attributes (phase point-style).
    PointStyle(PointStyle),
    /// `SURFACE_STYLE_BOUNDARY` — `founded_item` subtype with one
    /// `curve_or_render` SELECT (phase ssb).
    SurfaceStyleBoundary(SurfaceStyleBoundary),
    /// `SURFACE_STYLE_PARAMETER_LINE` — `founded_item` subtype carrying
    /// a `curve_or_render` SELECT plus a SET[1:2] of `direction_count_select`
    /// (phase sspl).
    SurfaceStyleParameterLine(SurfaceStyleParameterLine),
}

/// `SURFACE_STYLE_PARAMETER_LINE(style_of_parameter_lines, direction_counts)`.
/// Phase sspl. EXPRESS WHERE rule: HIINDEX = 1 or the two members have
/// different types. Not enforced on read (lenient); empty SETs drop the
/// entity on round-trip.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceStyleParameterLine {
    pub style_of_parameter_lines: CurveOrRender,
    pub direction_counts: Vec<DirectionCount>,
}

/// `direction_count_select` SELECT — typed integer wrapper for the U / V
/// parameter line counts on `SURFACE_STYLE_PARAMETER_LINE`. Phase sspl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectionCount {
    U(i64),
    V(i64),
}

/// `SURFACE_STYLE_BOUNDARY(style_of_boundary)` — `founded_item` subtype.
/// Phase ssb. The SELECT body lives in [`CurveOrRender`].
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceStyleBoundary {
    pub style_of_boundary: CurveOrRender,
}

/// `curve_or_render` SELECT — picks between a `curve_style` and a
/// `curve_style_rendering` (a `SurfaceStyleRendering` variant). Phase ssb.
///
/// Simple SELECT (all variants `Variant(XId)`) — `StepSelect` generates the
/// reader `resolve_select` / writer `emit_select`.
#[derive(Debug, Clone, Copy, PartialEq, StepSelect)]
pub enum CurveOrRender {
    CurveStyle(CurveStyleId),
    SurfaceStyleRendering(SurfaceStyleRenderingId),
}

/// `POINT_STYLE(name, marker, marker_size, marker_colour)` —
/// `founded_item` subtype. The `marker` SELECT covers either a
/// `MarkerType` ENUM token or a `PreDefinedMarker` reference; the
/// `marker_size` SELECT covers typed primitives
/// (`POSITIVE_LENGTH_MEASURE` / `DESCRIPTIVE_MEASURE`) and a
/// `MeasureWithUnit` reference. Any unresolved ref drops the occurrence
/// (symmetric on re-read).
#[derive(Debug, Clone, PartialEq)]
pub struct PointStyle {
    pub name: String,
    pub marker: Marker,
    pub marker_size: MarkerSize,
    pub marker_colour: ColourId,
}

/// `marker_select` SELECT — either a `marker_type` ENUM or a
/// `pre_defined_marker` reference.
#[derive(Debug, Clone, PartialEq)]
pub enum Marker {
    Predefined(PreDefinedMarkerId),
    Type(MarkerType),
}

/// `marker_type` ENUMERATION OF (dot, x, plus, asterisk, ring, square,
/// triangle). `Other` preserves any unmodelled token verbatim for
/// round-trip safety.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkerType {
    Dot,
    X,
    Plus,
    Asterisk,
    Ring,
    Square,
    Triangle,
    Other(String),
}

/// `size_select` SELECT — typed primitives or a `MeasureWithUnit`
/// reference.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkerSize {
    /// `POSITIVE_LENGTH_MEASURE(real)`.
    PositiveLength(f64),
    /// `ref_measure_with_unit`.
    MeasureWithUnit(MeasureWithUnitId),
    /// `DESCRIPTIVE_MEASURE(text)`.
    Descriptive(String),
}

/// `SYMBOL_STYLE(name, style_of_symbol)` — `founded_item` subtype
/// (phase symbol-style). `style_of_symbol` is the
/// `symbol_style_select` SELECT, narrowed to `SymbolColour` (the only
/// modelled SELECT member). Unresolved ref drops the occurrence.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolStyle {
    pub name: String,
    pub style_of_symbol: SymbolColourId,
}

/// EXPRESS `central_or_parallel` — a `VIEW_VOLUME`'s projection mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    Central,
    Parallel,
}

/// `VIEW_VOLUME(projection_type, projection_point, view_plane_distance,
/// front_plane_distance, front_plane_clipping, back_plane_distance,
/// back_plane_clipping, view_volume_sides_clipping, view_window)` — the
/// truncated pyramid / box a `CAMERA_MODEL_D3` projects through. A
/// `VIEW_VOLUME` whose `projection_point` / `view_window` does not resolve
/// is silently dropped, symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewVolume {
    pub projection_type: Projection,
    /// `projection_point` — a `ref_cartesian_point`.
    pub projection_point: PointId,
    pub view_plane_distance: f64,
    pub front_plane_distance: f64,
    pub front_plane_clipping: bool,
    pub back_plane_distance: f64,
    pub back_plane_clipping: bool,
    pub view_volume_sides_clipping: bool,
    /// `view_window` — a `ref_planar_box`.
    pub view_window: PlanarExtentId,
}

/// `camera_model` `enum_base` — a camera projection model referenced by
/// presentation views. Variants cover `CAMERA_MODEL_D3` (the dominant
/// form) plus its `_with_hlhsr` / `_multi_clipping` subtypes.
#[derive(Debug, Clone, PartialEq)]
pub enum CameraModel {
    CameraModelD3(CameraModelD3),
    CameraModelD3WithHlhsr(CameraModelD3WithHlhsr),
    CameraModelD3MultiClipping(CameraModelD3MultiClipping),
}

/// `CAMERA_MODEL_D3_WITH_HLHSR` — `camera_model_d3` SUBTYPE with one
/// boolean attribute. Phase cm-variants.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraModelD3WithHlhsr {
    pub inherited: CameraModelD3,
    pub hidden_line_surface_removal: bool,
}

/// `CAMERA_MODEL_D3_MULTI_CLIPPING` — `camera_model_d3` SUBTYPE with a
/// SET of `shape_clipping` SELECT entries (`camera_model_d3_multi_clipping_intersection`
/// — modelled via the `Plane` surface narrow; the
/// `camera_model_d3_multi_clipping_union` recursive variant is not
/// modelled and source refs to it are silently dropped on read).
#[derive(Debug, Clone, PartialEq)]
pub struct CameraModelD3MultiClipping {
    pub inherited: CameraModelD3,
    pub shape_clipping: Vec<ShapeClipping>,
}

/// `camera_model_d3_multi_clipping_interection_select` SELECT — partial
/// modelling: only the `plane` member is preserved.
/// Source refs to `camera_model_d3_multi_clipping_union` are silently
/// dropped on read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeClipping {
    Plane(crate::ir::id::SurfaceId),
}

/// `CAMERA_MODEL_D3(name, view_reference_system, perspective_of_volume)` —
/// a 3D camera model. A `CAMERA_MODEL_D3` whose `view_reference_system` /
/// `perspective_of_volume` ref does not resolve is silently dropped,
/// symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraModelD3 {
    pub name: String,
    /// `view_reference_system` — a `ref_axis2_placement_3d`.
    pub view_reference_system: Placement3dId,
    /// `perspective_of_volume` — a `ref_view_volume`, resolved to the
    /// `founded_item` arena's [`FoundedItem::ViewVolume`] entry.
    pub perspective_of_volume: FoundedItemId,
}

/// `CURVE_STYLE(name, curve_font, curve_width, curve_colour)` —
/// reference-backed per the ir.toml blueprint
/// (`pool = "visualization"`, `arena = "curve_style"`).
#[derive(Debug, Clone, PartialEq)]
pub struct CurveStyle {
    pub name: String,
    /// `curve_font` SELECT. `OPTIONAL` in AP242 (required in AP203/AP214); an
    /// omitted `$` round-trips as `None`. `Some` points at a modelled
    /// `pre_defined_curve_font` arena entry.
    pub curve_font: Option<PreDefinedCurveFontId>,
    pub curve_width: CurveWidth,
    pub curve_colour: ColourId,
}

/// `pre_defined_curve_font` arena enum (`concrete_supertype`-style). Holds
/// the abstract self variant (`Plain`, corpus 0 in observed AP242 files)
/// and the concrete `DRAUGHTING_PRE_DEFINED_CURVE_FONT` subtype. Other
/// `curve_font_select` SELECT members (`CURVE_STYLE_FONT`,
/// `EXTERNALLY_DEFINED_CURVE_FONT`) are not modelled and are silently
/// dropped at read.
#[derive(Debug, Clone, PartialEq)]
pub enum PreDefinedCurveFont {
    Plain(PreDefinedCurveFontData),
    Draughting(DraughtingPreDefinedCurveFont),
}

/// `PRE_DEFINED_CURVE_FONT(name)` — abstract supertype body. Currently
/// unreachable from corpus instances; populated only by hand-built IR.
#[derive(Debug, Clone, PartialEq)]
pub struct PreDefinedCurveFontData {
    pub name: String,
}

/// `DRAUGHTING_PRE_DEFINED_CURVE_FONT(name)` — stock font reference.
/// Common names: `"continuous"`, `"dashed"`, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct DraughtingPreDefinedCurveFont {
    pub name: String,
}

/// `pre_defined_symbol` arena enum. Mirrors the curve-font family —
/// abstract self variant (`Plain`, corpus 0) plus the concrete
/// `PRE_DEFINED_TERMINATOR_SYMBOL` subtype.
#[derive(Debug, Clone, PartialEq)]
pub enum PreDefinedSymbol {
    Plain(PreDefinedSymbolData),
    Terminator(PreDefinedTerminatorSymbol),
}

/// `PRE_DEFINED_SYMBOL(name)` — abstract supertype body.
#[derive(Debug, Clone, PartialEq)]
pub struct PreDefinedSymbolData {
    pub name: String,
}

/// `PRE_DEFINED_TERMINATOR_SYMBOL(name)` — stock terminator symbol
/// (arrow heads, slashes, etc.) referenced from leader / dimension
/// annotation chains. step-io does not yet have a consumer arena, but
/// the leaf is round-tripped so corpus instances are preserved.
#[derive(Debug, Clone, PartialEq)]
pub struct PreDefinedTerminatorSymbol {
    pub name: String,
}

/// `size_select` SELECT supertype (the `curve_width` attribute of
/// `CURVE_STYLE`) — `(positive_length_measure, measure_with_unit,
/// descriptive_measure)`. Mirrors [`MarkerSize`]: the inline TYPE-alias
/// members (`POSITIVE_LENGTH_MEASURE(0.13)` / `DESCRIPTIVE_MEASURE('thick')`)
/// are stored inline; `measure_with_unit` is an entity ref into the units
/// pool (`mwu_id_map` on read, `mwu_step_ids` on write).
#[derive(Debug, Clone, PartialEq)]
pub enum CurveWidth {
    /// `POSITIVE_LENGTH_MEASURE(real)`.
    PositiveLengthMeasure(f64),
    /// `ref_measure_with_unit`.
    MeasureWithUnit(MeasureWithUnitId),
    /// `DESCRIPTIVE_MEASURE(text)`.
    Descriptive(String),
}

/// `COLOUR` supertype per the AP214/AP242 schema. Only the variants
/// step-io currently round-trips are listed; unsupported forms are
/// silently dropped at read.
#[derive(Debug, Clone, PartialEq)]
pub enum Colour {
    Rgb(ColourRgb),
    PreDefined(DraughtingPreDefinedColour),
    /// Bare `COLOUR()` — the attributeless base entity (`ENTITY colour;
    /// END_ENTITY;`, ISO 10303-46) instantiated directly as an unspecified
    /// colour placeholder. Fieldless because the base carries no attributes
    /// (unlike the data-bearing `Itself` variants of other enums).
    Itself,
}

/// `DRAUGHTING_PRE_DEFINED_COLOUR(name)` — predefined colour reference.
/// Common names: `"white"`, `"black"`, `"red"`, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct DraughtingPreDefinedColour {
    pub name: String,
}

/// `STYLED_ITEM` enum per the ir.toml blueprint
/// (`enum_of = "styled_item"`). `Plain` covers the base `STYLED_ITEM`
/// entity; `OverRiding` covers `OVER_RIDING_STYLED_ITEM`, which decorates
/// another `StyledItem` with replacement styles.
#[derive(Debug, Clone, PartialEq)]
pub enum StyledItem {
    Plain(PlainStyledItem),
    OverRiding(OverRidingStyledItem),
    ContextDependent(ContextDependentOverRidingStyledItem),
}

/// `STYLED_ITEM(name, styles, item)` — binds presentation styles to a
/// geometry/topology IR object.
#[derive(Debug, Clone, PartialEq)]
pub struct PlainStyledItem {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    pub item: RepresentationItemRef,
}

/// `OVER_RIDING_STYLED_ITEM(name, styles, item, over_ridden_style)` —
/// inherits from `STYLED_ITEM` with one extra field referencing the
/// `StyledItem` whose styles this entity replaces.
#[derive(Debug, Clone, PartialEq)]
pub struct OverRidingStyledItem {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    pub item: RepresentationItemRef,
    pub over_ridden_style: StyledItemId,
}

/// `CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM(name, styles, item,
/// over_ridden_style, style_context)` — extends `OVER_RIDING_STYLED_ITEM`
/// with a non-empty SELECT list designating the contexts (`shape` /
/// `representation` / `representation_item` / `shape_aspect`) where the
/// style override applies.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextDependentOverRidingStyledItem {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    pub item: RepresentationItemRef,
    pub over_ridden_style: StyledItemId,
    pub style_context: Vec<StyleContextRef>,
}

/// SELECT target for [`ContextDependentOverRidingStyledItem::style_context`].
/// `RepresentationItem` covers geometry, topology, a geometry representation,
/// or a 3D placement via [`RepresentationItemRef`]. `RepresentationRelationship`
/// covers the assembly placement complex (the dominant `style_context` target):
/// a style override scoped to one specific assembly instance, referenced by its
/// materialised [`RepresentationRelationshipId`] (the
/// `RepresentationRelationshipWithTransformation` arena entry). Other SELECT
/// members (`shape`, `shape_aspect`) are dropped at read time with a warning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StyleContextRef {
    RepresentationItem(RepresentationItemRef),
    RepresentationRelationship(crate::ir::id::RepresentationRelationshipId),
}

/// `PRESENTATION_STYLE_ASSIGNMENT` enum per the ir.toml blueprint
/// (`enum_of = "presentation_style_assignment"`). `Itself` covers the
/// base `PRESENTATION_STYLE_ASSIGNMENT`; `PresentationStyleByContext`
/// covers the `PRESENTATION_STYLE_BY_CONTEXT` subtype whose
/// `style_context` field references `REPRESENTATION` / `REPRESENTATION_ITEM`
/// — step-io does not yet model representations as first-class IR objects,
/// so the reader leaves the by-context handler unregistered (silent skip).
/// The enum variant exists to keep the IR aligned with the blueprint for
/// the eventual Representation-IR phase.
#[derive(Debug, Clone, PartialEq)]
pub enum PresentationStyleAssignment {
    Itself(PresentationStyleAssignmentData),
    PresentationStyleByContext(PresentationStyleByContext),
}

/// `PRESENTATION_STYLE_ASSIGNMENT(styles)` body. Source PSA carries a
/// mixed list of styling variants ([`SurfaceStyleUsage`] and
/// [`CurveStyle`] are currently round-tripped; `POINT_STYLE` and other
/// SELECT members are silently dropped on read).
#[derive(Debug, Clone, PartialEq)]
pub struct PresentationStyleAssignmentData {
    pub styles: Vec<PsaStyle>,
}

/// `PRESENTATION_STYLE_BY_CONTEXT(styles, style_context)` body. Phase psbc.
/// `style_context` is partial: only `Representation` and `Item` SELECT
/// members are modelled; other members (`Group`, `PresentationLayerAssignment`,
/// `PresentationSet`) drop the carrier on read.
#[derive(Debug, Clone, PartialEq)]
pub struct PresentationStyleByContext {
    pub styles: Vec<PsaStyle>,
    pub style_context: StyleContext,
}

/// `style_context_select` SELECT — partial enum (Representation + Item only).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StyleContext {
    Representation(crate::ir::id::RepresentationId),
    Item(crate::ir::representation_item::RepresentationItemRef),
}

/// One element of [`PresentationStyleAssignmentData::styles`]. Both
/// variants reference their target through arena ids — `Surface` carries
/// a [`FoundedItemId`] aimed at a `FoundedItem::SurfaceStyleUsage` entry,
/// `Point` carries a [`FoundedItemId`] aimed at a `FoundedItem::PointStyle`
/// entry, and `Curve` carries a [`CurveStyleId`].
#[derive(Debug, Clone, PartialEq)]
pub enum PsaStyle {
    Surface(FoundedItemId),
    Point(FoundedItemId),
    Curve(CurveStyleId),
    /// `null_style` ENUMERATION placeholder — emitted by NIST AP242
    /// fixtures as `NULL_STYLE(.NULL.)` typed parameter. Phase psa-typed.
    Null,
}

/// `SURFACE_STYLE_USAGE(side, style)`. The `style` field references a
/// `FoundedItem::SurfaceSideStyle` entry through its [`FoundedItemId`].
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceStyleUsage {
    pub side: SurfaceSide,
    pub style: FoundedItemId,
}

/// `surface_side` enum from `SURFACE_STYLE_USAGE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceSide {
    Front,
    Back,
    Both,
}

/// `SURFACE_SIDE_STYLE(name, styles)`.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceSideStyle {
    pub name: String,
    pub styles: Vec<SurfaceSideStyleEntry>,
}

/// One element of a `SURFACE_SIDE_STYLE.styles` list. STEP allows several
/// style entry types here; this scope covers `SURFACE_STYLE_FILL_AREA`
/// (color fill) and the `SURFACE_STYLE_RENDERING` arena
/// (color + transparency / other rendering hints).
// Not `StepSelect`: `FillArea` resolves via the `viz_ssfa_id_map` named field
// (TypeId-collision with `id_cache`), not `id_cache.get`. See `ir::select`.
#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceSideStyleEntry {
    FillArea(FoundedItemId),
    Rendering(SurfaceStyleRenderingId),
}

/// `SURFACE_STYLE_FILL_AREA(fill_area)`. The `fill_area` field points at
/// a `FoundedItem::FillAreaStyle` arena entry.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceStyleFillArea {
    pub fill_area: FoundedItemId,
}

/// `surface_style_rendering` arena enum per ir.toml. `Itself` covers the
/// base `SURFACE_STYLE_RENDERING(rendering_method, surface_colour)`;
/// `SurfaceStyleRenderingWithProperties` covers the
/// `SURFACE_STYLE_RENDERING_WITH_PROPERTIES` subtype that adds a
/// `properties` list. Both share the same [`SurfaceStyleRenderingId`].
#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceStyleRendering {
    Itself(SurfaceStyleRenderingData),
    SurfaceStyleRenderingWithProperties(SurfaceStyleRenderingWithProperties),
}

/// `SURFACE_STYLE_RENDERING(rendering_method, surface_colour)` body. Both
/// fields are non-optional in the schema; the reader normalizes any
/// non-standard `$` (Unset) input to a standard default, so the IR always
/// holds a schema-valid value.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceStyleRenderingData {
    pub rendering_method: ShadingMethod,
    pub surface_colour: ColourId,
}

/// `SURFACE_STYLE_RENDERING_WITH_PROPERTIES(rendering_method, surface_colour,
/// properties)` body. Same fields as the base entity plus a list of
/// additional rendering hints — only `SURFACE_STYLE_TRANSPARENT` is
/// currently modelled (see [`RenderingProperty`]); other property entries
/// are silently dropped on read.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceStyleRenderingWithProperties {
    pub rendering_method: ShadingMethod,
    pub surface_colour: ColourId,
    pub properties: Vec<RenderingProperty>,
}

/// `shading_surface_method` enum from `SURFACE_STYLE_RENDERING`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadingMethod {
    Constant,
    Colour,
    Dot,
    Normal,
}

/// One element of a `SURFACE_STYLE_RENDERING_WITH_PROPERTIES.properties`
/// list. Other variants (e.g. `SURFACE_STYLE_REFLECTANCE_AMBIENT`) are
/// out of scope — symmetric ignorance keeps round-trip equality intact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderingProperty {
    /// `SURFACE_STYLE_TRANSPARENT(transparency)` — value in `[0.0, 1.0]`.
    Transparent(f64),
}

/// `FILL_AREA_STYLE(name, fill_styles)`.
#[derive(Debug, Clone, PartialEq)]
pub struct FillAreaStyle {
    pub name: String,
    pub fill_styles: Vec<FillAreaStyleColour>,
}

/// `FILL_AREA_STYLE_COLOUR(name, colour)`.
#[derive(Debug, Clone, PartialEq)]
pub struct FillAreaStyleColour {
    pub name: String,
    pub colour: ColourId,
}

/// `pre_defined_marker` enum arena (phase pre-defined-marker).
/// [`PreDefinedCurveFont`] / [`PreDefinedSymbol`] 동형. `Plain` covers direct
/// `PRE_DEFINED_MARKER`
/// instances (corpus 64). `PointMarkerSymbol` is reserved for the future
/// complex-MI sub-phase (6 corpus instances, not handled yet — drop on
/// read).
#[derive(Debug, Clone, PartialEq)]
pub enum PreDefinedMarker {
    Plain(PreDefinedMarkerData),
    PointMarkerSymbol(PreDefinedPointMarkerSymbol),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreDefinedMarkerData {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreDefinedPointMarkerSymbol {
    pub name: String,
}

/// `geometric_representation_item` `enum_base` — abstract supertype that
/// step-io currently models with a unified arena holding only the
/// symbol-domain `in_enum` members (phase ds-st). The other 6 corpus
/// `in_enum` members (`direction`, `vector`, `vertex_point`,
/// `geometric_curve_set`, `geometric_set`, `shell_based_surface_model`)
/// keep their dedicated arenas and are not migrated here yet.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometricRepresentationItem {
    DefinedSymbol(DefinedSymbol),
    SymbolTarget(SymbolTarget),
    ShellBasedSurfaceModel(ShellBasedSurfaceModel),
    GeometricCurveSet(GeometricCurveSet),
    GeometricSet(GeometricSet),
}

/// `SHELL_BASED_SURFACE_MODEL(name, sbsm_boundary)` — collects one or more
/// shells (open or closed) into a single `representation_item`. Most
/// commercial fixtures fold their SBSMs into an MSSR's `items` list, but
/// AP242 also allows SBSMs to be referenced directly from a `STYLED_ITEM`
/// (standalone usage), which is what brings this variant into the
/// `GeometricRepresentationItem` arena.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellBasedSurfaceModel {
    pub name: String,
    pub shells: Vec<crate::ir::id::ShellId>,
}

/// `GEOMETRIC_CURVE_SET(name, elements)` — bundles curves (and the few
/// loose points some producers attach) into a single `representation_item`.
/// `GBWSR` flattens these into a `WireframeContent`; `STYLED_ITEM` can also
/// target a GCS directly (phase gcs-cluster).
#[derive(Debug, Clone, PartialEq)]
pub struct GeometricCurveSet {
    pub name: String,
    pub curves: Vec<crate::ir::id::CurveId>,
    pub points: Vec<crate::ir::id::PointId>,
}

/// `GEOMETRIC_SET(name, elements)` — `GEOMETRIC_CURVE_SET`'s supertype that
/// additionally allows surface refs. step-io currently only models curves
/// and points (surfaces are not produced by the corpus in `GEOMETRIC_SET`
/// form); the variant exists for round-trip name fidelity.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometricSet {
    pub name: String,
    pub curves: Vec<crate::ir::id::CurveId>,
    pub points: Vec<crate::ir::id::PointId>,
}

/// `DEFINED_SYMBOL(name, definition, target)` — `geometric_representation_item`
/// SUBTYPE pairing a `defined_symbol_select` payload with a
/// `symbol_target` placement frame. Phase ds-st.
#[derive(Debug, Clone, PartialEq)]
pub struct DefinedSymbol {
    pub name: String,
    pub definition: DefinedSymbolDefinition,
    /// Narrows to a `SymbolTarget` variant of the same arena.
    pub target: crate::ir::id::GeometricRepresentationItemId,
}

/// `defined_symbol_select` SELECT — `pre_defined_symbol` only.
/// `externally_defined_symbol` is corpus 0 (no `ir.toml` entry) and
/// silently dropped on read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DefinedSymbolDefinition {
    PreDefinedSymbol(crate::ir::id::PreDefinedSymbolId),
}

/// `SYMBOL_TARGET(name, placement, x_scale, y_scale)` —
/// `geometric_representation_item` SUBTYPE that supplies the placement
/// + uniform scale for a `defined_symbol`. Phase ds-st.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolTarget {
    pub name: String,
    pub placement: SymbolPlacement,
    pub x_scale: f64,
    pub y_scale: f64,
}

/// `axis2_placement` SELECT — 3D only in step-io.
/// 2D `AXIS2_PLACEMENT_2D` inputs are dropped on read because step-io
/// has no 2D placement id map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolPlacement {
    Placement3d(crate::ir::id::Placement3dId),
}

/// `text_style` enum arena (phase text-style-box). `Itself` covers the base
/// abstract `TEXT_STYLE` (corpus 0 — reserved, never reached at read);
/// `WithBoxCharacteristics` covers the `TEXT_STYLE_WITH_BOX_CHARACTERISTICS`
/// subtype (corpus 116).
#[derive(Debug, Clone, PartialEq)]
pub enum TextStyle {
    Itself(TextStyleData),
    WithBoxCharacteristics(TextStyleWithBoxCharacteristics),
}

/// `TEXT_STYLE(name, character_appearance)` carrier body — shared by the
/// base entity and the `_with_box_characteristics` subtype via the
/// `concrete_supertype` + carrier shape in ir.toml.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyleData {
    pub name: String,
    pub character_appearance: CharacterStyle,
}

/// `TEXT_STYLE_WITH_BOX_CHARACTERISTICS(name, character_appearance,
/// characteristics)` — adds a `SET[1:4]` of typed box-characteristic
/// primitives. The schema `WHERE` enforces typeof-uniqueness inside the
/// set; corpus instances already comply, so step-io preserves whatever
/// order the source provided.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyleWithBoxCharacteristics {
    pub inherited: TextStyleData,
    pub characteristics: Vec<BoxCharacteristic>,
}

/// `character_style_select` SELECT — step-io models only the
/// `text_style_for_defined_font` member; the two glyph-style members
/// (`character_glyph_style_stroke`, `character_glyph_style_outline`) are
/// not modelled and are silently dropped on read (`ApprovalItem` precedent).
#[derive(Debug, Clone, Copy, PartialEq, StepSelect)]
pub enum CharacterStyle {
    TextStyleForDefinedFont(TextStyleForDefinedFontId),
}

/// `box_characteristic_select` SELECT — typed REAL primitives carried by
/// `TEXT_STYLE_WITH_BOX_CHARACTERISTICS.characteristics`. Both Real and
/// Integer source forms are accepted on read (Integer is cast to f64),
/// mirroring the `POSITIVE_LENGTH_MEASURE` handling in `POINT_STYLE`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoxCharacteristic {
    /// `BOX_HEIGHT(positive_ratio_measure)`.
    Height(f64),
    /// `BOX_WIDTH(positive_ratio_measure)`.
    Width(f64),
    /// `BOX_SLANT_ANGLE(plane_angle_measure)`.
    SlantAngle(f64),
    /// `BOX_ROTATE_ANGLE(plane_angle_measure)`.
    RotateAngle(f64),
}

/// `presentation_representation` blueprint enum (phase pr-core) —
/// abstract supertype with two `in_enum` variants. Both variants share
/// the inherited `representation` body (name + items + context).
#[derive(Debug, Clone, PartialEq)]
pub enum PresentationRepresentation {
    /// `PRESENTATION_VIEW(name, items, context_of_items)`.
    View(PresentationReprData),
    /// `PRESENTATION_AREA(name, items, context_of_items)`.
    Area(PresentationReprData),
}

/// Shared body for `PRESENTATION_VIEW` / `PRESENTATION_AREA` simple
/// instances.
#[derive(Debug, Clone, PartialEq)]
pub struct PresentationReprData {
    pub name: String,
    pub items: Vec<crate::ir::representation_item::RepresentationItemRef>,
    pub context: Option<crate::ir::shape_rep::RepresentationContextRef>,
}

/// `PRESENTATION_SET` — has no direct attributes in the schema (only an
/// INVERSE `areas` SET); instantiated as `PRESENTATION_SET()`. Carried as a
/// marker so `AREA_IN_SET.in_set` / `PRESENTED_ITEM_REPRESENTATION` refs
/// resolve during read.
#[derive(Debug, Clone, PartialEq)]
pub struct PresentationSet;

/// `AREA_IN_SET(area, in_set)` — phase pr-size. Both refs must resolve
/// at read time (otherwise the carrier is dropped). `area` narrows to a
/// `PresentationRepresentation::Area` variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaInSet {
    pub area: crate::ir::id::PresentationRepresentationId,
    pub in_set: crate::ir::id::PresentationSetId,
}

/// `PRESENTATION_SIZE(unit, size)` — phase pr-size. `size` references a
/// `planar_box` (`PlanarExtent` arena's `PlanarBox` variant); `unit` is
/// a 3-member SELECT classified into a `PresentationView` /
/// `PresentationArea` / `AreaInSet` ref.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresentationSize {
    pub unit: PresentationSizeAssignment,
    pub size: PlanarExtentId,
}

/// `presentation_size_assignment_select` SELECT — 3 members.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PresentationSizeAssignment {
    /// `presentation_view` member (narrow View variant of
    /// `PresentationRepresentation`).
    View(crate::ir::id::PresentationRepresentationId),
    /// `presentation_area` member.
    Area(crate::ir::id::PresentationRepresentationId),
    /// `area_in_set` member.
    AreaInSet(crate::ir::id::AreaInSetId),
}

/// `PRESENTED_ITEM_REPRESENTATION(presentation, item)` — phase pr-item.
/// `presentation` is a `presentation_representation_select` (modelled
/// `PresentationRepresentationId` / `PresentationSetId`); `item` is a
/// `presented_item` — an abstract entity whose only concrete subtype is
/// `applied_presented_item`, so it references the `applied_presented_items`
/// arena (not a `presented_item_select` member).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresentedItemRepresentation {
    pub presentation: PresentationReprSelect,
    pub item: crate::ir::id::AppliedPresentedItemId,
}

/// `presentation_representation_select` SELECT — modelled members.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PresentationReprSelect {
    Representation(crate::ir::id::PresentationRepresentationId),
    Set(crate::ir::id::PresentationSetId),
}

/// `presented_item_select` SELECT — only the `ProductDefinition` member
/// is modelled. Other 8 PLM members (`action` / `product_concept` / ...) are
/// silently dropped on read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PresentedItem {
    ProductDefinition(crate::ir::id::ProductId),
}

/// `APPLIED_PRESENTED_ITEM(items)` — phase pr-item. `items` is a SET of
/// `presented_item` SELECT members.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedPresentedItem {
    pub items: Vec<PresentedItem>,
}

/// `INVISIBILITY(invisible_items)` — phase invisibility. Marks a set of
/// presentation entities as not to render. `presentation_context` is
/// reserved for the `CONTEXT_DEPENDENT_INVISIBILITY` subtype (not yet
/// modelled — always `None` in this phase).
#[derive(Debug, Clone, PartialEq)]
pub struct Invisibility {
    pub invisible_items: Vec<InvisibleItem>,
    pub presentation_context: Option<InvisibilityContext>,
}

/// `invisible_item` SELECT — the four schema members
/// (`draughting_callout`, `presentation_layer_assignment`, `representation`,
/// `styled_item`), plus `AnnotationOccurrence`. The latter is an EXPRESS
/// `styled_item` subtype (`annotation_occurrence SUBTYPE OF styled_item`) but
/// step-io models annotation occurrences in a separate arena
/// (`AnnotationOccurrenceId`) from plain styled items (`StyledItemId`), so the
/// `styled_item` SELECT member is split into two variants here. Source refs to
/// dropped target instances are skipped on read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InvisibleItem {
    AnnotationOccurrence(crate::ir::id::AnnotationOccurrenceId),
    DraughtingCallout(crate::ir::id::DraughtingCalloutId),
    PresentationLayerAssignment(crate::ir::id::PresentationLayerAssignmentId),
    Representation(crate::ir::id::RepresentationId),
    StyledItem(StyledItemId),
}

/// `invisibility_context` SELECT — only `draughting_model` is modelled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InvisibilityContext {
    DraughtingModel(crate::ir::id::RepresentationId),
}

/// `TEXT_LITERAL(name, literal, placement, alignment, path, font)` —
/// phase text-literal. `geometric_representation_item` subtype carrying a
/// drafting label's content + 2D/3D placement + font.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLiteral {
    pub name: String,
    /// `presentable_text` body — free-form unicode text.
    pub literal: String,
    pub placement: Axis2Placement,
    /// `text_alignment` label (free-form string like `"baseline left"`).
    pub alignment: String,
    pub path: TextPath,
    pub font: FontSelect,
}

/// `axis2_placement` SELECT — either a 2D or 3D axis placement.
/// Unresolved refs drop the carrier instance, symmetric on re-read.
// Not `StepSelect`: `D3` resolves via the `placement_map` named field
// (TypeId-collision with `id_cache`), not `id_cache.get`. See `ir::select`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis2Placement {
    D2(Placement2dId),
    D3(Placement3dId),
}

/// `font_select` SELECT — step-io models only the
/// `draughting_pre_defined_text_font` member; the
/// `externally_defined_text_font` member is silently dropped on read
/// (`ApprovalItem` precedent, `feedback_partial_select_enum`).
#[derive(Debug, Clone, Copy, PartialEq, StepSelect)]
pub enum FontSelect {
    DraughtingPreDefined(DraughtingPreDefinedTextFontId),
}

/// `text_path` ENUMERATION OF (left, right, up, down). `Other` preserves
/// any extension token verbatim for round-trip safety.
#[derive(Debug, Clone, PartialEq)]
pub enum TextPath {
    Left,
    Right,
    Up,
    Down,
    Other(String),
}

/// `COMPOSITE_TEXT(name, collected_text)` — phase text-literal. Groups
/// `SET[2:?]` of `text_or_character` SELECT entries.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositeText {
    pub name: String,
    pub collected_text: Vec<TextOrCharacter>,
}

/// `text_or_character` SELECT — narrowed to the `text_literal` member.
/// `annotation_text_character` / `composite_text` / `character_glyph` are
/// not modelled in step-io (`feedback_partial_select_enum`); element
/// references to those types are silently dropped on read.
#[derive(Debug, Clone, Copy, PartialEq, StepSelect)]
pub enum TextOrCharacter {
    TextLiteral(TextLiteralId),
}

/// `TEXT_STYLE_FOR_DEFINED_FONT(text_colour)` — phase text-style-font.
/// Wraps a `Colour` reference for use as a `character_style_select`
/// SELECT member. `text_colour` unresolved drops the occurrence,
/// symmetric on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyleForDefinedFont {
    pub text_colour: ColourId,
}

/// `SYMBOL_COLOUR(colour_of_symbol)` — phase symbol-colour.
/// Wraps a `Colour` reference for use as a `symbol_style_select` SELECT
/// member. `colour_of_symbol` unresolved drops the occurrence, symmetric
/// on re-read.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolColour {
    pub colour_of_symbol: ColourId,
}

/// `COLOUR_RGB(name, red, green, blue)` — RGB triple in `[0.0, 1.0]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourRgb {
    pub name: String,
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}
