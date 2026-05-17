//! Visualization data — `STYLED_ITEM` chain + colours + style metadata.
//!
//! Colours migrated to `Arena<Colour>` with `ColourId` references per the
//! ir.toml blueprint (`pool = "visualization"`, `enum_of = "colour"`).
//! Other styling types (`FillAreaStyle`, `SurfaceStyleRendering`, etc.)
//! remain tree-inline pending later phases of the blueprint migration.

use super::arena::Arena;
use super::id::{ColourId, CurveId, FaceId, PointId, SolidId};
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

/// `STYLED_ITEM(name, styles, item)` — binds presentation styles to a
/// geometry IR object.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledItem {
    pub name: String,
    pub styles: Vec<PresentationStyleAssignment>,
    pub item: StyledItemTarget,
}

/// What the `STYLED_ITEM.item` ref resolved to in step-io's geometry IR.
/// Fusion 360 commonly styles individual `ADVANCED_FACE` entities (per-face
/// coloring); CATIA mostly styles whole solids and wire fragments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StyledItemTarget {
    Solid(SolidId),
    Face(FaceId),
    Curve(CurveId),
    Point(PointId),
}

/// `PRESENTATION_STYLE_ASSIGNMENT(styles)`. Source PSA may carry
/// `CURVE_STYLE` / `POINT_STYLE` / etc. in addition to
/// `SURFACE_STYLE_USAGE`; this scope only models the surface variant
/// (other entries are silently dropped on read — symmetric ignorance
/// keeps round-trip equality intact).
#[derive(Debug, Clone, PartialEq)]
pub struct PresentationStyleAssignment {
    pub styles: Vec<SurfaceStyleUsage>,
}

/// `SURFACE_STYLE_USAGE(side, style)`.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceStyleUsage {
    pub side: SurfaceSide,
    pub style: SurfaceSideStyle,
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
/// (color fill) and `SURFACE_STYLE_RENDERING_WITH_PROPERTIES`
/// (color + transparency / other rendering hints).
#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceSideStyleEntry {
    FillArea(SurfaceStyleFillArea),
    Rendering(SurfaceStyleRendering),
}

/// `SURFACE_STYLE_FILL_AREA(fill_area)`.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceStyleFillArea {
    pub fill_area: FillAreaStyle,
}

/// `SURFACE_STYLE_RENDERING_WITH_PROPERTIES(rendering_method, surface_colour,
/// properties)`. The first attribute is the `shading_surface_method` enum
/// per the AP214 schema, but Fusion 360 commonly emits `$` (unset) for it —
/// we accept both, storing `None` for the unset case to round-trip the
/// distinction. `properties` is a list of additional rendering hints; we
/// currently model `SURFACE_STYLE_TRANSPARENT` as the only
/// [`RenderingProperty`] variant.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceStyleRendering {
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
