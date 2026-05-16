//! `SEAM_CURVE` handler — Pass 4-3.
//!
//! Sister handler of `SURFACE_CURVE`. Both share their read/write body
//! in `surface_curve.rs`; only the entity name on emission differs.

use crate::entities::geometry::surface_curve::{
    read_surface_or_seam_curve_body, write_surface_or_seam_curve_body,
};
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::Pcurve;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(crate) struct SeamCurveHandler;

impl SimpleEntityHandler for SeamCurveHandler {
    const NAME: &'static str = "SEAM_CURVE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4_3SurfaceCurve;
    type WriteInput = (u64, Vec<Pcurve>);

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_surface_or_seam_curve_body(ctx, entity_id, attrs, "SEAM_CURVE")
    }

    fn write(
        buf: &mut WriteBuffer,
        (curve_3d_ref, pcurves): (u64, Vec<Pcurve>),
    ) -> Result<u64, WriteError> {
        write_surface_or_seam_curve_body(buf, curve_3d_ref, &pcurves, true)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static SEAM_CURVE_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: SeamCurveHandler::NAME,
    pass_level: SeamCurveHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: SeamCurveHandler::read,
    },
};
