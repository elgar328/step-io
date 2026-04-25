//! Visualization emission — `STYLED_ITEM` chain + `COLOUR_RGB` + style metadata.
//!
//! Recursive emit — each parent emits its children first, then references
//! their fresh ids. The IR is tree-inline so this never deduplicates colors;
//! 15 styled items sharing a color in the source file emit 15 separate
//! `COLOUR_RGB` entities. See `crate::ir::visualization` for the design
//! rationale.

use super::WriteBuffer;
use crate::ir::visualization::{
    ColorRgb, FillAreaStyle, FillAreaStyleColour, Mdgpr, PresentationStyleAssignment, StyledItem,
    StyledItemTarget, SurfaceSide, SurfaceSideStyle, SurfaceStyleFillArea, SurfaceStyleUsage,
};
use crate::parser::entity::Attribute;
use crate::writer::WriteError;
use crate::writer::entity::{WriterBody, WriterEntity};

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_visualization_if_set(
        &mut self,
    ) -> Result<(), WriteError> {
        let Some(viz) = self.model.visualization.clone() else {
            return Ok(());
        };
        let unit_ctx = self.global_unit_context_id;
        for mdgpr in &viz.mdgprs {
            self.emit_mdgpr(mdgpr, unit_ctx)?;
        }
        Ok(())
    }

    fn emit_mdgpr(&mut self, mdgpr: &Mdgpr, unit_ctx: Option<u64>) -> Result<u64, WriteError> {
        let mut item_refs = Vec::with_capacity(mdgpr.items.len());
        for si in &mdgpr.items {
            let id = self.emit_styled_item(si)?;
            item_refs.push(Attribute::EntityRef(id));
        }
        let context = match unit_ctx {
            Some(ctx) => Attribute::EntityRef(ctx),
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

    fn emit_styled_item(&mut self, si: &StyledItem) -> Result<u64, WriteError> {
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

    fn emit_psa(&mut self, psa: &PresentationStyleAssignment) -> u64 {
        let mut style_refs = Vec::with_capacity(psa.styles.len());
        for ssu in &psa.styles {
            style_refs.push(Attribute::EntityRef(self.emit_ssu(ssu)));
        }
        self.push_simple(
            "PRESENTATION_STYLE_ASSIGNMENT",
            vec![Attribute::List(style_refs)],
        )
    }

    fn emit_ssu(&mut self, ssu: &SurfaceStyleUsage) -> u64 {
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

    fn emit_sss(&mut self, sss: &SurfaceSideStyle) -> u64 {
        let mut style_refs = Vec::with_capacity(sss.styles.len());
        for ssfa in &sss.styles {
            style_refs.push(Attribute::EntityRef(self.emit_ssfa(ssfa)));
        }
        self.push_simple(
            "SURFACE_SIDE_STYLE",
            vec![
                Attribute::String(sss.name.clone()),
                Attribute::List(style_refs),
            ],
        )
    }

    fn emit_ssfa(&mut self, ssfa: &SurfaceStyleFillArea) -> u64 {
        let fas_id = self.emit_fas(&ssfa.fill_area);
        self.push_simple(
            "SURFACE_STYLE_FILL_AREA",
            vec![Attribute::EntityRef(fas_id)],
        )
    }

    fn emit_fas(&mut self, fas: &FillAreaStyle) -> u64 {
        let mut style_refs = Vec::with_capacity(fas.fill_styles.len());
        for fasc in &fas.fill_styles {
            style_refs.push(Attribute::EntityRef(self.emit_fasc(fasc)));
        }
        self.push_simple(
            "FILL_AREA_STYLE",
            vec![
                Attribute::String(fas.name.clone()),
                Attribute::List(style_refs),
            ],
        )
    }

    fn emit_fasc(&mut self, fasc: &FillAreaStyleColour) -> u64 {
        let colour_id = self.emit_colour_rgb(&fasc.colour);
        self.push_simple(
            "FILL_AREA_STYLE_COLOUR",
            vec![
                Attribute::String(fasc.name.clone()),
                Attribute::EntityRef(colour_id),
            ],
        )
    }

    fn emit_colour_rgb(&mut self, c: &ColorRgb) -> u64 {
        let n = self.fresh();
        self.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "COLOUR_RGB".into(),
                attrs: vec![
                    Attribute::String(c.name.clone()),
                    Attribute::Real(c.red),
                    Attribute::Real(c.green),
                    Attribute::Real(c.blue),
                ],
            },
        });
        n
    }
}
