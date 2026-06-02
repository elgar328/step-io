//! `RATIONAL_B_SPLINE_CURVE` handler for the PCURVE 2D variant
//!. Mirrors the 3D `rational_bspline_curve.rs` but reads
//! 2D control points and pushes into `curves_2d`.
//!
//! The `is_2d` handler flag routes 2D handlers only to
//! entities inside a PCURVE `DEFINITIONAL_REPRESENTATION` subtree, so this
//! handler never sees a 3D rational entity. The 3D sister handler in
//! `rational_bspline_curve.rs` likewise never sees a 2D entity (the 3D
//! dispatch path skips the pcurve subtree).

use crate::entities::geometry::cartesian_point_2d::CartesianPoint2dHandler;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::attr::{
    read_bool, read_entity_ref_list, read_enum, read_integer, read_integer_list, read_logical,
    read_real_list, read_string_or_unset,
};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::geometry::{Curve2d, CurveForm, NurbsCurve2d, NurbsKind2d};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::reader::require_part_attrs;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct RationalBsplineCurve2dHandler;

#[step_entity_complex(
    name = "RATIONAL_B_SPLINE_CURVE",
    is_2d,
    cases = [[
        "BOUNDED_CURVE", "B_SPLINE_CURVE", "B_SPLINE_CURVE_WITH_KNOTS", "CURVE",
        "GEOMETRIC_REPRESENTATION_ITEM", "RATIONAL_B_SPLINE_CURVE", "REPRESENTATION_ITEM"
    ]]
)]
impl ComplexEntityHandler for RationalBsplineCurve2dHandler {
    type WriteInput = NurbsCurve2d;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let repr_attrs = require_part_attrs(parts, "REPRESENTATION_ITEM", entity_id)?;
        let _name = read_string_or_unset(repr_attrs, 0, entity_id, "name")?;

        let bsc_attrs = require_part_attrs(parts, "B_SPLINE_CURVE", entity_id)?;
        let degree_i = read_integer(bsc_attrs, 0, entity_id, "degree")?;
        let cp_refs = read_entity_ref_list(bsc_attrs, 1, entity_id, "control_points_list")?;
        let form = CurveForm::from_step_enum(read_enum(bsc_attrs, 2, entity_id, "curve_form")?);
        let closed = read_bool(bsc_attrs, 3, entity_id, "closed_curve")?;
        let _self_intersect = read_logical(bsc_attrs, 4, entity_id, "self_intersect")?;

        let bswk_attrs = require_part_attrs(parts, "B_SPLINE_CURVE_WITH_KNOTS", entity_id)?;
        let knot_multiplicities =
            read_integer_list(bswk_attrs, 0, entity_id, "knot_multiplicities")?;
        let knots = read_real_list(bswk_attrs, 1, entity_id, "knots")?;

        // 2D self-discrimination: if the first control point is not in the
        // 2D map, the 3D sister handler in `rational_bspline_curve.rs` owns
        // this entity. Silent skip — mirrors the same pattern in
        // `b_spline_curve_with_knots.rs:51-55`.
        if let Some(&first_ref) = cp_refs.first()
            && !ctx.point_2d_map.contains_key(&first_ref)
        {
            return Ok(());
        }

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
            let pt = *ctx
                .point_2d_map
                .get(&r)
                .ok_or(ConvertError::MissingReference {
                    from: entity_id,
                    to: r,
                    field_name: "control_points_list",
                })?;
            control_points.push(pt);
        }

        let id = ctx.geometry.curves_2d.push(Curve2d::Nurbs(NurbsCurve2d {
            degree,
            control_points,
            kind: NurbsKind2d::Rational { weights },
            knot_multiplicities,
            knots,
            closed,
            form,
        }));
        ctx.curve_2d_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, nurbs: NurbsCurve2d) -> Result<u64, WriteError> {
        let NurbsKind2d::Rational { weights } = nurbs.kind else {
            return Err(WriteError::UnsupportedIrVariant {
                detail: "RationalBsplineCurve2dHandler::write requires weights".into(),
            });
        };

        let mut cp_refs = Vec::with_capacity(nurbs.control_points.len());
        for &pid in &nurbs.control_points {
            cp_refs.push(CartesianPoint2dHandler::write(buf, pid)?);
        }
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
                            Attribute::Enum("F".into()),
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
