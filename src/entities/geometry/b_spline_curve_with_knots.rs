//! `B_SPLINE_CURVE_WITH_KNOTS` handler — Pass 4-1 leaf NURBS curve
//! (non-rational; rational form lives in `rational_bspline_curve.rs`).

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::ir::attr::{
    check_count, logical_to_step, read_bool, read_entity_ref_list, read_enum, read_integer,
    read_integer_list, read_logical, read_real_list, read_string,
};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::geometry::{Curve, CurveForm, NurbsCurve};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct BSplineCurveWithKnotsHandler;

#[step_entity(name = "B_SPLINE_CURVE_WITH_KNOTS", pass = Pass4Leaf)]
impl SimpleEntityHandler for BSplineCurveWithKnotsHandler {
    type WriteInput = NurbsCurve;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 9, entity_id, "B_SPLINE_CURVE_WITH_KNOTS")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let degree_i = read_integer(attrs, 1, entity_id, "degree")?;
        let cp_refs = read_entity_ref_list(attrs, 2, entity_id, "control_points_list")?;
        let form = CurveForm::from_step_enum(read_enum(attrs, 3, entity_id, "curve_form")?);
        let closed = read_bool(attrs, 4, entity_id, "closed_curve")?;
        let self_intersect = read_logical(attrs, 5, entity_id, "self_intersect")?;
        let knot_multiplicities = read_integer_list(attrs, 6, entity_id, "knot_multiplicities")?;
        let knots = read_real_list(attrs, 7, entity_id, "knots")?;
        // [8] knot_spec — informational, skipped

        let degree = u32::try_from(degree_i).map_err(|_| ConvertError::AttributeType {
            entity_id,
            field_name: "degree",
            expected: "non-negative Integer",
            actual: AttributeKindTag::Integer,
        })?;

        // If the first control point is a known 2D point, this is the
        // 2D sister B_SPLINE_CURVE_WITH_KNOTS — silently skip.
        if let Some(&first_ref) = cp_refs.first()
            && ctx.point_2d_map.contains_key(&first_ref)
        {
            return Ok(());
        }
        let mut control_points = Vec::with_capacity(cp_refs.len());
        for &r in &cp_refs {
            let pt = ctx.resolve_point(entity_id, r, "control_points_list")?;
            control_points.push(pt);
        }

        let curve = NurbsCurve {
            degree,
            control_points,
            weights: None,
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
        debug_assert!(
            nurbs.weights.is_none(),
            "BSplineCurveWithKnotsHandler::write expects a non-rational curve"
        );
        let mut cp_refs = Vec::with_capacity(nurbs.control_points.len());
        for &pid in &nurbs.control_points {
            cp_refs.push(CartesianPointHandler::write(buf, pid)?);
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
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "B_SPLINE_CURVE_WITH_KNOTS".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    degree_attr,
                    cps_attr,
                    Attribute::Enum(form.as_step_enum().into()),
                    closed_attr,
                    Attribute::Enum(logical_to_step(nurbs.self_intersect).into()),
                    mults_attr,
                    knots_attr,
                    Attribute::Enum("UNSPECIFIED".into()),
                ],
            },
        });
        Ok(n)
    }
}
