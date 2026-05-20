//! Visualization data — `STYLED_ITEM` chain + colours + style metadata.
//!
//! Colours migrated to `Arena<Colour>` with `ColourId` references per the
//! ir.toml blueprint (`pool = "visualization"`, `enum_of = "colour"`).
//! Other styling types (`FillAreaStyle`, `SurfaceStyleRendering`, etc.)
//! remain tree-inline pending later phases of the blueprint migration.

use super::arena::Arena;
use super::id::{
    ColourId, CurveFontId, CurveId, CurveStyleId, EdgeId, FaceId, FoundedItemId, PointId,
    PresentationStyleAssignmentId, ShellId, SolidId, StyledItemId, SurfaceId,
    SurfaceStyleRenderingId, VertexId,
};
use super::shape_rep::Mdgpr;

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
    /// `curve_font` SELECT arena — one entry per source curve-font entity
    /// (currently only `DRAUGHTING_PRE_DEFINED_CURVE_FONT`). Referenced by
    /// [`CurveStyle::curve_font`] via [`CurveFontId`].
    pub curve_fonts: Arena<CurveFont>,
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
}

/// `CURVE_STYLE(name, curve_font, curve_width, curve_colour)` —
/// reference-backed per the ir.toml blueprint
/// (`pool = "visualization"`, `arena = "curve_style"`).
#[derive(Debug, Clone, PartialEq)]
pub struct CurveStyle {
    pub name: String,
    pub curve_font: CurveFontId,
    pub curve_width: CurveWidth,
    pub curve_colour: ColourId,
}

/// `curve_font_select` SELECT supertype. Only the variants step-io
/// currently round-trips are listed; unsupported forms (composite
/// `CURVE_STYLE_FONT`, etc.) are silently dropped at read.
#[derive(Debug, Clone, PartialEq)]
pub enum CurveFont {
    PreDefined(DraughtingPreDefinedCurveFont),
}

/// `DRAUGHTING_PRE_DEFINED_CURVE_FONT(name)` — stock font reference.
/// Common names: `"continuous"`, `"dashed"`, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct DraughtingPreDefinedCurveFont {
    pub name: String,
}

/// `curve_width_select` SELECT supertype. STEP TYPE alias members
/// (typed-real syntax like `POSITIVE_LENGTH_MEASURE(0.13)`); not
/// entities, so stored inline with no arena.
#[derive(Debug, Clone, PartialEq)]
pub enum CurveWidth {
    PositiveLengthMeasure(f64),
}

/// `COLOUR` SELECT supertype per the AP214/AP242 schema. Only the variants
/// step-io currently round-trips are listed; unsupported forms are
/// silently dropped at read.
#[derive(Debug, Clone, PartialEq)]
pub enum Colour {
    Rgb(ColourRgb),
    PreDefined(DraughtingPreDefinedColour),
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
    pub item: StyledItemTarget,
}

/// `OVER_RIDING_STYLED_ITEM(name, styles, item, over_ridden_style)` —
/// inherits from `STYLED_ITEM` with one extra field referencing the
/// `StyledItem` whose styles this entity replaces.
#[derive(Debug, Clone, PartialEq)]
pub struct OverRidingStyledItem {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignmentId>,
    pub item: StyledItemTarget,
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
    pub item: StyledItemTarget,
    pub over_ridden_style: StyledItemId,
    pub style_context: Vec<StyleContextRef>,
}

/// SELECT target for [`ContextDependentOverRidingStyledItem::style_context`].
/// Initial coverage: `RepresentationItem` (geometry / topology via
/// [`StyledItemTarget`]). Other variants (`representation`,
/// `shape_representation`, `shape_aspect`) silently dropped at read time
/// with a warning; future phases expand the enum once representation IR
/// and `ShapeAspect` pass-ordering are addressed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StyleContextRef {
    RepresentationItem(StyledItemTarget),
}

/// What the `STYLED_ITEM.item` ref resolved to in step-io's IR. The STEP
/// `representation_item` SELECT is broad; this enum lists the variants
/// step-io currently round-trips. Fusion 360 commonly styles individual
/// `ADVANCED_FACE` entities (per-face colouring); ABC-corpus files
/// frequently style `EDGE_CURVE` entities for line decoration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StyledItemTarget {
    Solid(SolidId),
    Face(FaceId),
    Edge(EdgeId),
    Curve(CurveId),
    Point(PointId),
    Surface(SurfaceId),
    Vertex(VertexId),
    Shell(ShellId),
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

/// `PRESENTATION_STYLE_BY_CONTEXT(styles, style_context)` body. The
/// `style_context` field maps to `presentation_style_context_select`
/// (`REPRESENTATION` / `REPRESENTATION_ITEM`); step-io does not model
/// representations yet, so this struct is a placeholder pending that
/// phase. No reader handler is currently registered.
#[derive(Debug, Clone, PartialEq)]
pub struct PresentationStyleByContext {
    pub styles: Vec<PsaStyle>,
}

/// One element of [`PresentationStyleAssignmentData::styles`]. Both
/// variants reference their target through arena ids — `Surface` carries
/// a [`FoundedItemId`] aimed at a `FoundedItem::SurfaceStyleUsage` entry,
/// and `Curve` carries a [`CurveStyleId`].
#[derive(Debug, Clone, PartialEq)]
pub enum PsaStyle {
    Surface(FoundedItemId),
    Curve(CurveStyleId),
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

/// `SURFACE_STYLE_RENDERING(rendering_method, surface_colour)` body. The
/// `rendering_method` field is non-optional in the schema, but Fusion 360
/// commonly emits `$` (Unset); the `Option<ShadingMethod>` preserves that
/// distinction.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceStyleRenderingData {
    pub rendering_method: Option<ShadingMethod>,
    pub surface_colour: ColourId,
}

/// `SURFACE_STYLE_RENDERING_WITH_PROPERTIES(rendering_method, surface_colour,
/// properties)` body. Same fields as the base entity plus a list of
/// additional rendering hints — only `SURFACE_STYLE_TRANSPARENT` is
/// currently modelled (see [`RenderingProperty`]); other property entries
/// are silently dropped on read.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceStyleRenderingWithProperties {
    pub rendering_method: Option<ShadingMethod>,
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

/// `COLOUR_RGB(name, red, green, blue)` — RGB triple in `[0.0, 1.0]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourRgb {
    pub name: String,
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}
