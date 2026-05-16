//! `RATIONAL_B_SPLINE_CURVE` handler — Plan 3 stage 3 pilot for the
//! `ComplexEntityHandler` registry path.
//!
//! Mirrors the legacy `ReaderContext::convert_rational_bspline_curve`
//! (`src/reader/geometry.rs`) and the rational branch of
//! `WriteBuffer::emit_nurbs_curve` (`src/writer/buffer/geometry.rs`).
//! `emit_nurbs_curve` still emits the non-rational simple
//! `B_SPLINE_CURVE_WITH_KNOTS` form inline; only the rational complex
//! emission moves here.

use crate::entities::{
    ComplexEntityHandler, ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind,
};
use crate::ir::attr::{
    logical_to_step, read_bool, read_entity_ref_list, read_enum, read_integer, read_integer_list,
    read_logical, read_real_list, read_string,
};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::{Curve, CurveForm, NurbsCurve};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::reader::require_part_attrs;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct RationalBsplineCurveHandler;

impl ComplexEntityHandler for RationalBsplineCurveHandler {
    const NAME: &'static str = "RATIONAL_B_SPLINE_CURVE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4Rational;
    const REQUIRED_PARTS: &'static [&'static str] = &[
        "B_SPLINE_CURVE",
        "B_SPLINE_CURVE_WITH_KNOTS",
        "RATIONAL_B_SPLINE_CURVE",
    ];
    type WriteInput = NurbsCurve;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let repr_attrs = require_part_attrs(parts, "REPRESENTATION_ITEM", entity_id)?;
        let _name = read_string(repr_attrs, 0, entity_id, "name")?;

        let bsc_attrs = require_part_attrs(parts, "B_SPLINE_CURVE", entity_id)?;
        let degree_i = read_integer(bsc_attrs, 0, entity_id, "degree")?;
        let cp_refs = read_entity_ref_list(bsc_attrs, 1, entity_id, "control_points_list")?;
        let form = CurveForm::from_step_enum(read_enum(bsc_attrs, 2, entity_id, "curve_form")?);
        let closed = read_bool(bsc_attrs, 3, entity_id, "closed_curve")?;
        let self_intersect = read_logical(bsc_attrs, 4, entity_id, "self_intersect")?;

        let bswk_attrs = require_part_attrs(parts, "B_SPLINE_CURVE_WITH_KNOTS", entity_id)?;
        let knot_multiplicities =
            read_integer_list(bswk_attrs, 0, entity_id, "knot_multiplicities")?;
        let knots = read_real_list(bswk_attrs, 1, entity_id, "knots")?;

        let rat_attrs = require_part_attrs(parts, "RATIONAL_B_SPLINE_CURVE", entity_id)?;
        let weights = read_real_list(rat_attrs, 0, entity_id, "weights_data")?;

        if weights.len() != cp_refs.len() {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "weights_data",
                expected: cp_refs.len(),
                actual: weights.len(),
            });
        }

        let degree = u32::try_from(degree_i).map_err(|_| ConvertError::AttributeType {
            entity_id,
            field_name: "degree",
            expected: "non-negative Integer",
            actual: AttributeKindTag::Integer,
        })?;

        let mut control_points = Vec::with_capacity(cp_refs.len());
        for &r in &cp_refs {
            let pt = ctx.resolve_point(entity_id, r, "control_points_list")?;
            control_points.push(pt);
        }

        let curve = NurbsCurve {
            degree,
            control_points,
            weights: Some(weights),
            knot_multiplicities,
            knots,
            closed,
            form,
            self_intersect,
        };
        let id = ctx.geometry.curves.push(Curve::Nurbs(curve));
        ctx.curve_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, nurbs: NurbsCurve) -> Result<u64, WriteError> {
        let weights = nurbs
            .weights
            .ok_or_else(|| WriteError::UnsupportedIrVariant {
                detail: "RationalBsplineCurveHandler::write requires weights".into(),
            })?;

        let mut cp_refs = Vec::with_capacity(nurbs.control_points.len());
        for &pid in &nurbs.control_points {
            cp_refs.push(buf.emit_point(pid)?);
        }
        #[allow(clippy::cast_possible_wrap)]
        let degree_attr = Attribute::Integer(i64::from(nurbs.degree));
        let cps_attr = Attribute::List(cp_refs.into_iter().map(Attribute::EntityRef).collect());
        let mults_attr = Attribute::List(
            nurbs
                .knot_multiplicities
                .iter()
                .copied()
                .map(Attribute::Integer)
                .collect(),
        );
        let knots_attr =
            Attribute::List(nurbs.knots.iter().copied().map(Attribute::Real).collect());
        let closed_attr = Attribute::Enum(if nurbs.closed { "T".into() } else { "F".into() });
        let form = nurbs.form;
        let weights_attr = Attribute::List(weights.into_iter().map(Attribute::Real).collect());

        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex {
                parts: vec![
                    ("BOUNDED_CURVE".into(), vec![]),
                    (
                        "B_SPLINE_CURVE".into(),
                        vec![
                            degree_attr,
                            cps_attr,
                            Attribute::Enum(form.as_step_enum().into()),
                            closed_attr,
                            Attribute::Enum(logical_to_step(nurbs.self_intersect).into()),
                        ],
                    ),
                    (
                        "B_SPLINE_CURVE_WITH_KNOTS".into(),
                        vec![
                            mults_attr,
                            knots_attr,
                            Attribute::Enum("UNSPECIFIED".into()),
                        ],
                    ),
                    ("CURVE".into(), vec![]),
                    ("GEOMETRIC_REPRESENTATION_ITEM".into(), vec![]),
                    ("RATIONAL_B_SPLINE_CURVE".into(), vec![weights_attr]),
                    (
                        "REPRESENTATION_ITEM".into(),
                        vec![Attribute::String(String::new())],
                    ),
                ],
            },
        });
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static RATIONAL_BSPLINE_CURVE_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: RationalBsplineCurveHandler::NAME,
    pass_level: RationalBsplineCurveHandler::PASS_LEVEL,
    kind: ReadKind::Complex {
        required_parts: RationalBsplineCurveHandler::REQUIRED_PARTS,
        read: RationalBsplineCurveHandler::read_complex,
    },
};
