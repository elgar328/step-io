//! Visualization emission — `STYLED_ITEM` chain + `COLOUR_RGB` + style metadata.
//!
//! Recursive emit — each parent emits its children first, then references
//! their fresh ids. The IR is tree-inline so this never deduplicates colors;
//! 15 styled items sharing a color in the source file emit 15 separate
//! `COLOUR_RGB` entities. See `crate::ir::visualization` for the design
//! rationale.

use super::WriteBuffer;
use crate::ir::visualization::{
    ColorRgb, Mdgpr, PresentationStyleAssignment, RenderingProperty, ShadingMethod, StyledItem,
    StyledItemTarget, SurfaceSide, SurfaceSideStyle, SurfaceSideStyleEntry, SurfaceStyleFillArea,
    SurfaceStyleRendering, SurfaceStyleUsage,
};
use crate::parser::entity::Attribute;
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_visualization_if_set(
        &mut self,
    ) -> Result<(), WriteError> {
        let Some(viz) = self.model.visualization.clone() else {
            return Ok(());
        };
        for mdgpr in &viz.mdgprs {
            self.emit_mdgpr(mdgpr)?;
        }
        Ok(())
    }

    pub(crate) fn emit_mdgpr(&mut self, mdgpr: &Mdgpr) -> Result<u64, WriteError> {
        let mut item_refs = Vec::with_capacity(mdgpr.items.len());
        for si in &mdgpr.items {
            let id = self.emit_styled_item(si)?;
            item_refs.push(Attribute::EntityRef(id));
        }
        // MDGPR's `context_of_items` is required by the spec but the IR
        // accepts `None` for kernel-built fragments. Some(id) → resolve via
        // cached `unit_context_ids`; None → emit `Unset` (current behaviour
        // preserved for synthetic IR with no context info).
        let context = match mdgpr.context {
            Some(id) => Attribute::EntityRef(self.unit_context_ids[id.0 as usize]),
            None => Attribute::Unset,
        };
        Ok(self.push_simple(
            "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION",
            vec![
                Attribute::String(mdgpr.name.clone()),
                Attribute::List(item_refs),
                context,
            ],
        ))
    }

    pub(crate) fn emit_styled_item(&mut self, si: &StyledItem) -> Result<u64, WriteError> {
        let item_id = match si.item {
            StyledItemTarget::Solid(sid) => self.emit_solid(sid)?,
            StyledItemTarget::Face(fid) => self.emit_face(fid)?,
            StyledItemTarget::Curve(cid) => self.emit_curve(cid)?,
            StyledItemTarget::Point(pid) => self.emit_point(pid)?,
        };
        let mut style_refs = Vec::with_capacity(si.styles.len());
        for psa in &si.styles {
            style_refs.push(Attribute::EntityRef(self.emit_psa(psa)));
        }
        Ok(self.push_simple(
            "STYLED_ITEM",
            vec![
                Attribute::String(si.name.clone()),
                Attribute::List(style_refs),
                Attribute::EntityRef(item_id),
            ],
        ))
    }

    pub(crate) fn emit_psa(&mut self, psa: &PresentationStyleAssignment) -> u64 {
        let mut style_refs = Vec::with_capacity(psa.styles.len());
        for ssu in &psa.styles {
            style_refs.push(Attribute::EntityRef(self.emit_ssu(ssu)));
        }
        self.push_simple(
            "PRESENTATION_STYLE_ASSIGNMENT",
            vec![Attribute::List(style_refs)],
        )
    }

    pub(crate) fn emit_ssu(&mut self, ssu: &SurfaceStyleUsage) -> u64 {
        let style_ref = self.emit_sss(&ssu.style);
        let side = match ssu.side {
            SurfaceSide::Front => "POSITIVE",
            SurfaceSide::Back => "NEGATIVE",
            SurfaceSide::Both => "BOTH",
        };
        self.push_simple(
            "SURFACE_STYLE_USAGE",
            vec![
                Attribute::Enum(side.into()),
                Attribute::EntityRef(style_ref),
            ],
        )
    }

    pub(crate) fn emit_sss(&mut self, sss: &SurfaceSideStyle) -> u64 {
        let mut style_refs = Vec::with_capacity(sss.styles.len());
        for entry in &sss.styles {
            let entry_id = match entry {
                SurfaceSideStyleEntry::FillArea(ssfa) => self.emit_ssfa(ssfa),
                SurfaceSideStyleEntry::Rendering(ssr) => self.emit_ssr(ssr),
            };
            style_refs.push(Attribute::EntityRef(entry_id));
        }
        self.push_simple(
            "SURFACE_SIDE_STYLE",
            vec![
                Attribute::String(sss.name.clone()),
                Attribute::List(style_refs),
            ],
        )
    }

    pub(crate) fn emit_ssfa(&mut self, ssfa: &SurfaceStyleFillArea) -> u64 {
        use crate::entities::SimpleEntityHandler;
        crate::entities::visualization::surface_style_fill_area::SurfaceStyleFillAreaHandler::write(
            self,
            ssfa.clone(),
        )
        .expect("SSFA write only pushes simple entities")
    }
}

// The `emit_fas` / `emit_fasc` helpers that used to live here have moved
// into `entities/visualization/{fill_area_style,fill_area_style_colour}.rs`
// — Plan 7 stage C2 deleted the buffer-level wrappers since `emit_ssfa`
// dispatches into the entity handler chain directly.

impl WriteBuffer<'_> {
    pub(crate) fn emit_ssr(&mut self, ssr: &SurfaceStyleRendering) -> u64 {
        let colour_id = self.emit_colour_rgb(&ssr.surface_colour);
        let mut prop_refs = Vec::with_capacity(ssr.properties.len());
        for prop in &ssr.properties {
            let prop_id = match prop {
                RenderingProperty::Transparent(t) => self.emit_surface_style_transparent(*t),
            };
            prop_refs.push(Attribute::EntityRef(prop_id));
        }
        let method_attr = match ssr.rendering_method {
            None => Attribute::Unset,
            Some(ShadingMethod::Constant) => Attribute::Enum("CONSTANT_SHADING".into()),
            Some(ShadingMethod::Colour) => Attribute::Enum("COLOUR_SHADING".into()),
            Some(ShadingMethod::Dot) => Attribute::Enum("DOT_SHADING".into()),
            Some(ShadingMethod::Normal) => Attribute::Enum("NORMAL_SHADING".into()),
        };
        self.push_simple(
            "SURFACE_STYLE_RENDERING_WITH_PROPERTIES",
            vec![
                method_attr,
                Attribute::EntityRef(colour_id),
                Attribute::List(prop_refs),
            ],
        )
    }

    pub(crate) fn emit_surface_style_transparent(&mut self, transparency: f64) -> u64 {
        self.push_simple(
            "SURFACE_STYLE_TRANSPARENT",
            vec![Attribute::Real(transparency)],
        )
    }

    pub(crate) fn emit_colour_rgb(&mut self, c: &ColorRgb) -> u64 {
        use crate::entities::SimpleEntityHandler;
        crate::entities::visualization::colour_rgb::ColourRgbHandler::write(self, c.clone())
            .expect("COLOUR_RGB write only pushes simple entities")
    }
}
