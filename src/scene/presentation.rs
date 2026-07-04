//! Presentation — the colour, transparency, layer, and visibility of a shape.
//!
//! These surface as accessors on the geometry handles: `.color()`,
//! `.transparency()`, `.layer()`, and `.is_visible()`. A style the reader
//! cannot follow to a value — a complex style, or an unknown pre-defined
//! colour name — is recorded via [`Scene::warnings`](crate::scene::Scene)
//! rather than skipped silently.

use crate::generated::model as m;
use crate::scene::Ctx;

/// An RGB colour with components in `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}

/// The colour styled onto the entity `target` (a face or solid), if any.
pub(crate) fn colour_of(cx: Ctx<'_>, target: m::EntityKey) -> Option<Rgb> {
    let rg = cx.ref_graph();
    for r in rg.referrers(target) {
        // A STYLED_ITEM references `target` only through its `item` field, so any
        // styled item referring to `target` styles it.
        let m::EntityKey::StyledItem(sid) = r else {
            continue;
        };
        let si = cx.model.styled_item_arena.get(sid.0);
        if let Some(rgb) = si.styles.iter().find_map(|psa| psa_colour(cx, psa)) {
            return Some(rgb);
        }
    }
    None
}

fn psa_colour(cx: Ctx<'_>, r: &m::PresentationStyleAssignmentRef) -> Option<Rgb> {
    let styles: &[m::PresentationStyleSelectRef] = match r {
        m::PresentationStyleAssignmentRef::PresentationStyleAssignment(i) => {
            &cx.model.presentation_style_assignment_arena.get(i.0).styles
        }
        m::PresentationStyleAssignmentRef::PresentationStyleByContext(i) => {
            &cx.model.presentation_style_by_context_arena.get(i.0).styles
        }
        m::PresentationStyleAssignmentRef::Complex(_) => {
            cx.warn("styled item uses a complex presentation style assignment".to_owned());
            return None;
        }
    };
    styles.iter().find_map(|s| style_select_colour(cx, s))
}

fn style_select_colour(cx: Ctx<'_>, r: &m::PresentationStyleSelectRef) -> Option<Rgb> {
    match r {
        m::PresentationStyleSelectRef::SurfaceStyleUsage(i) => {
            let ssu = cx.model.surface_style_usage_arena.get(i.0);
            side_style_colour(cx, &ssu.style)
        }
        m::PresentationStyleSelectRef::FillAreaStyle(i) => fill_area_colour(cx, *i),
        m::PresentationStyleSelectRef::Complex(_) => {
            cx.warn("complex presentation style".to_owned());
            None
        }
        // Curve / point / text / pre-defined styles carry no surface colour.
        _ => None,
    }
}

fn side_style_colour(cx: Ctx<'_>, r: &m::SurfaceSideStyleSelectRef) -> Option<Rgb> {
    match r {
        m::SurfaceSideStyleSelectRef::SurfaceSideStyle(i) => cx
            .model
            .surface_side_style_arena
            .get(i.0)
            .styles
            .iter()
            .find_map(|e| element_colour(cx, e)),
        m::SurfaceSideStyleSelectRef::Complex(_) => {
            cx.warn("complex surface side style".to_owned());
            None
        }
        m::SurfaceSideStyleSelectRef::PreDefinedSurfaceSideStyle(_) => None,
    }
}

fn element_colour(cx: Ctx<'_>, r: &m::SurfaceStyleElementSelectRef) -> Option<Rgb> {
    match r {
        m::SurfaceStyleElementSelectRef::SurfaceStyleFillArea(i) => fill_area_ref_colour(
            cx,
            &cx.model.surface_style_fill_area_arena.get(i.0).fill_area,
        ),
        m::SurfaceStyleElementSelectRef::SurfaceStyleRendering(i) => colour_to_rgb(
            cx,
            &cx.model
                .surface_style_rendering_arena
                .get(i.0)
                .surface_colour,
        ),
        m::SurfaceStyleElementSelectRef::SurfaceStyleRenderingWithProperties(i) => colour_to_rgb(
            cx,
            &cx.model
                .surface_style_rendering_with_properties_arena
                .get(i.0)
                .surface_colour,
        ),
        m::SurfaceStyleElementSelectRef::Complex(_) => {
            cx.warn("complex surface style element".to_owned());
            None
        }
        // Boundary / control-grid / parameter-line / segmentation / silhouette.
        _ => None,
    }
}

fn fill_area_ref_colour(cx: Ctx<'_>, r: &m::FillAreaStyleRef) -> Option<Rgb> {
    match r {
        m::FillAreaStyleRef::FillAreaStyle(i) => fill_area_colour(cx, *i),
        m::FillAreaStyleRef::Complex(_) => {
            cx.warn("complex fill area style".to_owned());
            None
        }
    }
}

fn fill_area_colour(cx: Ctx<'_>, id: m::FillAreaStyleId) -> Option<Rgb> {
    cx.model
        .fill_area_style_arena
        .get(id.0)
        .fill_styles
        .iter()
        .find_map(|f| match f {
            m::FillStyleSelectRef::FillAreaStyleColour(i) => colour_to_rgb(
                cx,
                &cx.model.fill_area_style_colour_arena.get(i.0).fill_colour,
            ),
            m::FillStyleSelectRef::Complex(_) => {
                cx.warn("complex fill style".to_owned());
                None
            }
            // Hatching / tiles / texture styles carry no solid colour.
            _ => None,
        })
}

fn colour_to_rgb(cx: Ctx<'_>, r: &m::ColourRef) -> Option<Rgb> {
    match r {
        m::ColourRef::ColourRgb(i) => {
            let c = cx.model.colour_rgb_arena.get(i.0);
            Some(Rgb {
                red: c.red,
                green: c.green,
                blue: c.blue,
            })
        }
        m::ColourRef::DraughtingPreDefinedColour(i) => {
            let name = &cx.model.draughting_pre_defined_colour_arena.get(i.0).name;
            predefined_rgb(name).or_else(|| {
                cx.warn(format!("unknown pre-defined colour: {name}"));
                None
            })
        }
        m::ColourRef::Complex(_) => {
            cx.warn("complex colour".to_owned());
            None
        }
        // Bare COLOUR / colour specification / other pre-defined colour.
        _ => None,
    }
}

/// The eight standard draughting pre-defined colours.
fn predefined_rgb(name: &str) -> Option<Rgb> {
    let (red, green, blue) = match name.to_ascii_lowercase().as_str() {
        "red" => (1.0, 0.0, 0.0),
        "green" => (0.0, 1.0, 0.0),
        "blue" => (0.0, 0.0, 1.0),
        "yellow" => (1.0, 1.0, 0.0),
        "cyan" => (0.0, 1.0, 1.0),
        "magenta" => (1.0, 0.0, 1.0),
        "black" => (0.0, 0.0, 0.0),
        "white" => (1.0, 1.0, 1.0),
        _ => return None,
    };
    Some(Rgb { red, green, blue })
}

// ---------------------------------------------------------------------------
// Transparency
// ---------------------------------------------------------------------------

/// Reverse-walk from `target` (a face or solid) to each surface-style element
/// applied to it — `STYLED_ITEM` → presentation-style assignment →
/// `SURFACE_STYLE_USAGE` → `SURFACE_SIDE_STYLE` → element — running `leaf` on
/// each and returning its first `Some`. Shared by any per-element surface-style
/// query (currently transparency).
fn surface_elements_of<T>(
    cx: Ctx<'_>,
    target: m::EntityKey,
    leaf: impl Fn(Ctx<'_>, &m::SurfaceStyleElementSelectRef) -> Option<T>,
) -> Option<T> {
    let rg = cx.ref_graph();
    for r in rg.referrers(target) {
        let m::EntityKey::StyledItem(sid) = r else {
            continue;
        };
        let si = cx.model.styled_item_arena.get(sid.0);
        for psa in &si.styles {
            let styles: &[m::PresentationStyleSelectRef] = match psa {
                m::PresentationStyleAssignmentRef::PresentationStyleAssignment(i) => {
                    &cx.model.presentation_style_assignment_arena.get(i.0).styles
                }
                m::PresentationStyleAssignmentRef::PresentationStyleByContext(i) => {
                    &cx.model.presentation_style_by_context_arena.get(i.0).styles
                }
                m::PresentationStyleAssignmentRef::Complex(_) => {
                    cx.warn("styled item uses a complex presentation style assignment".to_owned());
                    continue;
                }
            };
            for style in styles {
                let m::PresentationStyleSelectRef::SurfaceStyleUsage(i) = style else {
                    continue;
                };
                let side = &cx.model.surface_style_usage_arena.get(i.0).style;
                let elements: &[m::SurfaceStyleElementSelectRef] = match side {
                    m::SurfaceSideStyleSelectRef::SurfaceSideStyle(i) => {
                        &cx.model.surface_side_style_arena.get(i.0).styles
                    }
                    m::SurfaceSideStyleSelectRef::Complex(_) => {
                        cx.warn("complex surface side style".to_owned());
                        continue;
                    }
                    m::SurfaceSideStyleSelectRef::PreDefinedSurfaceSideStyle(_) => continue,
                };
                if let Some(found) = elements.iter().find_map(|e| leaf(cx, e)) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// The transparency on a surface-style element, from a
/// `SURFACE_STYLE_RENDERING_WITH_PROPERTIES`'s `SURFACE_STYLE_TRANSPARENT`
/// property (the only place transparency lives).
fn element_transparency(cx: Ctx<'_>, r: &m::SurfaceStyleElementSelectRef) -> Option<f64> {
    let m::SurfaceStyleElementSelectRef::SurfaceStyleRenderingWithProperties(i) = r else {
        return None;
    };
    cx.model
        .surface_style_rendering_with_properties_arena
        .get(i.0)
        .properties
        .iter()
        .find_map(|p| match p {
            m::RenderingPropertiesSelectRef::SurfaceStyleTransparent(t) => Some(
                cx.model
                    .surface_style_transparent_arena
                    .get(t.0)
                    .transparency,
            ),
            _ => None,
        })
}

/// The transparency styled onto `target` (a face or solid), if any. Raw STEP
/// value: `0.0` = opaque, `1.0` = fully transparent.
pub(crate) fn transparency_of(cx: Ctx<'_>, target: m::EntityKey) -> Option<f64> {
    surface_elements_of(cx, target, element_transparency)
}

// ---------------------------------------------------------------------------
// Layer and visibility
// ---------------------------------------------------------------------------
//
// Corpus: a shape usually reaches a layer / invisibility through its STYLED_ITEM
// (88% / 97%), not directly. So both queries consider `target`'s "presentation
// delegates" = the target itself plus the styled items that reference it.

/// The styled items that style `target`.
fn styled_items_of(cx: Ctx<'_>, target: m::EntityKey) -> Vec<m::StyledItemId> {
    cx.ref_graph()
        .referrers(target)
        .iter()
        .filter_map(|r| match r {
            m::EntityKey::StyledItem(sid) => Some(*sid),
            _ => None,
        })
        .collect()
}

/// The layers `target` belongs to — assignments that contain it directly or
/// contain one of its styled items.
fn layers_of(cx: Ctx<'_>, target: m::EntityKey) -> Vec<m::PresentationLayerAssignmentId> {
    let rg = cx.ref_graph();
    let mut out: Vec<m::PresentationLayerAssignmentId> = rg
        .referrers(target)
        .iter()
        .filter_map(|r| match r {
            m::EntityKey::PresentationLayerAssignment(pid) => Some(*pid),
            _ => None,
        })
        .collect();
    for sid in styled_items_of(cx, target) {
        for r in rg.referrers(m::EntityKey::StyledItem(sid)) {
            if let m::EntityKey::PresentationLayerAssignment(pid) = r {
                out.push(*pid);
            }
        }
    }
    out
}

/// Whether an `INVISIBILITY` lists `key` among its invisible items.
fn referred_by_invisibility(cx: Ctx<'_>, key: m::EntityKey) -> bool {
    cx.ref_graph()
        .referrers(key)
        .iter()
        .any(|r| matches!(r, m::EntityKey::Invisibility(_)))
}

/// The name of the first layer `target` belongs to, if any.
pub(crate) fn layer_of(cx: Ctx<'_>, target: m::EntityKey) -> Option<&str> {
    let pid = *layers_of(cx, target).first()?;
    Some(&cx.model.presentation_layer_assignment_arena.get(pid.0).name)
}

/// Whether `target` is visible — i.e. no `INVISIBILITY` hides one of its styled
/// items or one of its layers. Defaults to visible when nothing hides it.
pub(crate) fn is_visible(cx: Ctx<'_>, target: m::EntityKey) -> bool {
    let styled_hidden = styled_items_of(cx, target)
        .into_iter()
        .any(|sid| referred_by_invisibility(cx, m::EntityKey::StyledItem(sid)));
    let layer_hidden = layers_of(cx, target)
        .into_iter()
        .any(|pid| referred_by_invisibility(cx, m::EntityKey::PresentationLayerAssignment(pid)));
    !(styled_hidden || layer_hidden)
}

// ---------------------------------------------------------------------------
// Curve style (an edge's / curve's colour, width, and line font)
// ---------------------------------------------------------------------------
//
// Corpus: curve styles apply to standalone curves (LINE / TRIMMED_CURVE / CIRCLE,
// ~60k) — reached via the `Curve` handle — not b-rep EDGE_CURVEs (498, a
// two-file Spatial InterOp anomaly).

/// Reverse-walk from `target` to each presentation-style select styling it
/// (`STYLED_ITEM` → presentation-style assignment → style), running `leaf` on
/// each and returning its first `Some`.
fn each_presentation_style<'m, T>(
    cx: Ctx<'m>,
    target: m::EntityKey,
    leaf: impl Fn(Ctx<'m>, &'m m::PresentationStyleSelectRef) -> Option<T>,
) -> Option<T> {
    let rg = cx.ref_graph();
    for r in rg.referrers(target) {
        let m::EntityKey::StyledItem(sid) = r else {
            continue;
        };
        for psa in &cx.model.styled_item_arena.get(sid.0).styles {
            let styles: &[m::PresentationStyleSelectRef] = match psa {
                m::PresentationStyleAssignmentRef::PresentationStyleAssignment(i) => {
                    &cx.model.presentation_style_assignment_arena.get(i.0).styles
                }
                m::PresentationStyleAssignmentRef::PresentationStyleByContext(i) => {
                    &cx.model.presentation_style_by_context_arena.get(i.0).styles
                }
                m::PresentationStyleAssignmentRef::Complex(_) => {
                    cx.warn("styled item uses a complex presentation style assignment".to_owned());
                    continue;
                }
            };
            if let Some(found) = styles.iter().find_map(|s| leaf(cx, s)) {
                return Some(found);
            }
        }
    }
    None
}

/// The `CURVE_STYLE` styling `target`, if any.
fn curve_style_of(cx: Ctx<'_>, target: m::EntityKey) -> Option<&m::CurveStyle> {
    each_presentation_style(cx, target, |cx, s| match s {
        m::PresentationStyleSelectRef::CurveStyle(i) => Some(cx.model.curve_style_arena.get(i.0)),
        _ => None,
    })
}

/// The scalar value of a `SIZE_SELECT` (a curve width).
fn size_f64(cx: Ctx<'_>, r: &m::SizeSelectRef) -> Option<f64> {
    let value = match r {
        m::SizeSelectRef::PositiveLengthMeasure(v) => return Some(*v),
        m::SizeSelectRef::LengthMeasureWithUnit(i) => {
            &cx.model
                .length_measure_with_unit_arena
                .get(i.0)
                .value_component
        }
        m::SizeSelectRef::MeasureWithUnit(i) => {
            &cx.model.measure_with_unit_arena.get(i.0).value_component
        }
        m::SizeSelectRef::MeasureRepresentationItem(i) => {
            &cx.model
                .measure_representation_item_arena
                .get(i.0)
                .value_component
        }
        m::SizeSelectRef::PlaneAngleMeasureWithUnit(i) => {
            &cx.model
                .plane_angle_measure_with_unit_arena
                .get(i.0)
                .value_component
        }
        m::SizeSelectRef::RatioMeasureWithUnit(i) => {
            &cx.model
                .ratio_measure_with_unit_arena
                .get(i.0)
                .value_component
        }
        m::SizeSelectRef::UncertaintyMeasureWithUnit(i) => {
            &cx.model
                .uncertainty_measure_with_unit_arena
                .get(i.0)
                .value_component
        }
        m::SizeSelectRef::MassMeasureWithUnit(i) => {
            &cx.model
                .mass_measure_with_unit_arena
                .get(i.0)
                .value_component
        }
        m::SizeSelectRef::DescriptiveMeasure(_) | m::SizeSelectRef::Complex(_) => return None,
    };
    Some(value.value.as_f64())
}

/// The line-font name of a curve font select (a line pattern like `continuous`).
fn curve_font_name<'m>(cx: Ctx<'m>, r: &m::CurveFontOrScaledCurveFontSelectRef) -> Option<&'m str> {
    match r {
        m::CurveFontOrScaledCurveFontSelectRef::CurveStyleFont(i) => {
            Some(&cx.model.curve_style_font_arena.get(i.0).name)
        }
        m::CurveFontOrScaledCurveFontSelectRef::CurveStyleFontAndScaling(i) => {
            Some(&cx.model.curve_style_font_and_scaling_arena.get(i.0).name)
        }
        m::CurveFontOrScaledCurveFontSelectRef::DraughtingPreDefinedCurveFont(i) => Some(
            &cx.model
                .draughting_pre_defined_curve_font_arena
                .get(i.0)
                .name,
        ),
        m::CurveFontOrScaledCurveFontSelectRef::PreDefinedCurveFont(i) => {
            Some(&cx.model.pre_defined_curve_font_arena.get(i.0).name)
        }
        // No name (externally defined) or unresolved (complex).
        m::CurveFontOrScaledCurveFontSelectRef::ExternallyDefinedCurveFont(_) => None,
        m::CurveFontOrScaledCurveFontSelectRef::Complex(_) => {
            cx.warn("curve style uses a complex font".to_owned());
            None
        }
    }
}

/// The colour of the curve style on `target`, if any.
pub(crate) fn curve_colour_of(cx: Ctx<'_>, target: m::EntityKey) -> Option<Rgb> {
    let cs = curve_style_of(cx, target)?;
    colour_to_rgb(cx, cs.curve_colour.as_ref()?)
}

/// The line width of the curve style on `target` (raw measure value), if any.
pub(crate) fn curve_width_of(cx: Ctx<'_>, target: m::EntityKey) -> Option<f64> {
    let cs = curve_style_of(cx, target)?;
    size_f64(cx, cs.curve_width.as_ref()?)
}

/// The line-font name of the curve style on `target`, if any.
pub(crate) fn curve_font_of(cx: Ctx<'_>, target: m::EntityKey) -> Option<&str> {
    let cs = curve_style_of(cx, target)?;
    curve_font_name(cx, cs.curve_font.as_ref()?)
}
