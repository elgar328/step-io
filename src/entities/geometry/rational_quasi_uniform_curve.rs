//! Complex `RATIONAL_B_SPLINE_CURVE` carrying a `QUASI_UNIFORM_CURVE`
//! marker. Mirrors `rational_bspline_curve` except the knot vector +
//! multiplicities are derived from `(degree, cp_count)` instead of being
//! read from a `B_SPLINE_CURVE_WITH_KNOTS` part. Required parts therefore
//! drop `B_SPLINE_CURVE_WITH_KNOTS` and add `QUASI_UNIFORM_CURVE`.

use crate::entities::ComplexEntityHandler;
use crate::entities::geometry::nurbs_shared::{build_curve_common, quasi_uniform_knots};
use crate::ir::attr::{
    read_bool, read_entity_ref_list, read_enum, read_integer, read_logical, read_real_list,
    read_string_or_unset,
};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::geometry::NurbsKind;
use crate::ir::{Curve, CurveForm, NurbsCurve};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::reader::require_part_attrs;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct RationalQuasiUniformCurveHandler;

#[step_entity_complex(
    name = "RATIONAL_B_SPLINE_CURVE",
    pass = Pass4Rational,
    cases = [[
        "BOUNDED_CURVE", "B_SPLINE_CURVE", "CURVE", "GEOMETRIC_REPRESENTATION_ITEM",
        "QUASI_UNIFORM_CURVE", "RATIONAL_B_SPLINE_CURVE", "REPRESENTATION_ITEM"
    ]]
)]
impl ComplexEntityHandler for RationalQuasiUniformCurveHandler {
    type WriteInput = NurbsCurve;

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
        let self_intersect = read_logical(bsc_attrs, 4, entity_id, "self_intersect")?;

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

        let Some((knot_multiplicities, knots)) = quasi_uniform_knots(degree, control_points.len())
        else {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "control_points_list",
                expected: degree as usize + 1,
                actual: control_points.len(),
            });
        };

        let curve = NurbsCurve {
            degree,
            control_points,
            kind: NurbsKind::Rational { weights },
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
        let common = build_curve_common(buf, &nurbs)?;
        let NurbsKind::Rational { weights } = nurbs.kind else {
            return Err(WriteError::UnsupportedIrVariant {
                detail: "RationalQuasiUniformCurveHandler::write requires weights".into(),
            });
        };
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
                            common.degree,
                            common.cps,
                            common.form,
                            common.closed,
                            common.self_intersect,
                        ],
                    ),
                    ("CURVE".into(), vec![]),
                    ("GEOMETRIC_REPRESENTATION_ITEM".into(), vec![]),
                    ("QUASI_UNIFORM_CURVE".into(), vec![]),
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
