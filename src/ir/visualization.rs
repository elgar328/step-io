//! Visualization data — `STYLED_ITEM` chain + `COLOUR_RGB` + style metadata.
//!
//! Preserved as a passive tree-inline IR — step-io does not interpret the
//! semantics (no per-Face color lookup, no shared color resolution). Each
//! [`StyledItem`] holds an inlined copy of its style chain down to
//! [`ColorRgb`], so color sharing in the source file is lost (15 styled
//! items referencing one `COLOUR_RGB` on disk become 15 inline `ColorRgb`
//! values in the IR). This is a transitional design — when step-io grows
//! a fully structured visualization layer (per-Face Color references,
//! PMI-style annotations), the tree-inline IR will be replaced wholesale.

use super::id::{CurveId, FaceId, PointId, SolidId, UnitContextId};

/// Top-level container for visualization data extracted from
/// `MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION` (MDGPR)
/// entities. Empty when the source file had no visualization chain or
/// when a kernel adapter built the IR without one.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VisualizationPool {
    /// Top-level MDGPR roots — emit order preserved.
    pub mdgprs: Vec<Mdgpr>,
}

/// `MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION(name, items, context)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Mdgpr {
    pub name: String,
    pub items: Vec<StyledItem>,
    /// Unit / uncertainty context referenced by this MDGPR. `Some(id)` indexes
    /// into [`crate::ir::model::StepModel::units`]. Fusion 360 typically uses
    /// a separate context here (different uncertainty than the geometry rep).
    /// `None` → writer emits `Attribute::Unset` for `context_of_items`
    /// (allowed by the spec for kernel-built IR with no context info).
    pub context: Option<UnitContextId>,
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
    pub surface_colour: ColorRgb,
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
    pub colour: ColorRgb,
}

/// `COLOUR_RGB(name, red, green, blue)` — RGB triple in `[0.0, 1.0]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorRgb {
    pub name: String,
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}
