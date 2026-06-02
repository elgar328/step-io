//! `B_SPLINE_CURVE_WITH_KNOTS` handler — Pass 4a-3 (2D, pcurve subtree).
//!
//! Sister of [`crate::entities::geometry::b_spline_curve_with_knots`].
//! Read accepts the non-rational form only; write returns
//! `UnsupportedIrVariant` if `weights` is set since 2D rational NURBS
//! has no fixture coverage yet.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::cartesian_point_2d::CartesianPoint2dHandler;
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref_list, read_enum, read_integer, read_integer_list,
    read_real_list, read_string_or_unset,
};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::geometry::{Curve2d, CurveForm, NurbsCurve2d, NurbsKind2d};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct BSplineCurve2dWithKnotsHandler;

#[step_entity(name = "B_SPLINE_CURVE_WITH_KNOTS", pass = Pass4aCurve, is_2d)]
impl SimpleEntityHandler for BSplineCurve2dWithKnotsHandler {
    type WriteInput = NurbsCurve2d;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 9, entity_id, "B_SPLINE_CURVE_WITH_KNOTS")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let degree_i = read_integer(attrs, 1, entity_id, "degree")?;
        let cp_refs = read_entity_ref_list(attrs, 2, entity_id, "control_points_list")?;
        let form = CurveForm::from_step_enum(read_enum(attrs, 3, entity_id, "curve_form")?);
        let closed = read_bool(attrs, 4, entity_id, "closed_curve")?;
        let knot_multiplicities = read_integer_list(attrs, 6, entity_id, "knot_multiplicities")?;
        let knots = read_real_list(attrs, 7, entity_id, "knots")?;

        let degree = u32::try_from(degree_i).map_err(|_| ConvertError::AttributeType {
            entity_id,
            field_name: "degree",
            expected: "non-negative Integer",
            actual: AttributeKindTag::Integer,
        })?;

        let mut control_points = Vec::with_capacity(cp_refs.len());
        // First control point discriminates 2D vs 3D: if it's absent
        // from the 2D arena, this is the 3D B_SPLINE_CURVE_WITH_KNOTS.
        // Once the first point is confirmed 2D, missing successors are
        // legitimate errors.
        if let Some(&first_ref) = cp_refs.first() {
            if !ctx.point_2d_map.contains_key(&first_ref) {
                return Ok(());
            }
        }
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
            kind: NurbsKind2d::NonRational,
            knot_multiplicities,
            knots,
            closed,
            form,
        }));
        ctx.curve_2d_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, nurbs: NurbsCurve2d) -> Result<u64, WriteError> {
        debug_assert!(
            nurbs.weights().is_none(),
            "BSplineCurve2dWithKnotsHandler::write expects a non-rational curve; \
             dispatch in emit_nurbs_curve_2d routes rational forms to the 2D rational handler",
        );
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
                    Attribute::Enum("F".into()),
                    mults_attr,
                    knots_attr,
                    Attribute::Enum("UNSPECIFIED".into()),
                ],
            },
        });
        Ok(n)
    }
}
