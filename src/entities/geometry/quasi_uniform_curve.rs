//! `QUASI_UNIFORM_CURVE` handler — simple, non-rational B-spline with
//! derived knot vector + multiplicities (STEP `QuasiUniformKnots` and
//! `QuasiUniformKnotsMultiplicities` rules). Mirrors the
//! `B_SPLINE_CURVE_WITH_KNOTS` handler in shape; the only differences are
//! the source entity name, the absence of explicit knot attributes on
//! the wire, and the on-read derivation of mults/knots from
//! `(degree, cp_count)`.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::nurbs_shared::quasi_uniform_knots;
use crate::ir::attr::{
    check_count, logical_to_step, read_bool, read_entity_ref_list, read_enum, read_integer,
    read_logical, read_string_or_unset,
};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::geometry::{Curve, CurveForm, NurbsCurve, NurbsKind};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct QuasiUniformCurveHandler;

#[step_entity(name = "QUASI_UNIFORM_CURVE", pass = Pass4Leaf)]
impl SimpleEntityHandler for QuasiUniformCurveHandler {
    type WriteInput = NurbsCurve;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 6, entity_id, "QUASI_UNIFORM_CURVE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let degree_i = read_integer(attrs, 1, entity_id, "degree")?;
        let cp_refs = read_entity_ref_list(attrs, 2, entity_id, "control_points_list")?;
        let form = CurveForm::from_step_enum(read_enum(attrs, 3, entity_id, "curve_form")?);
        let closed = read_bool(attrs, 4, entity_id, "closed_curve")?;
        let self_intersect = read_logical(attrs, 5, entity_id, "self_intersect")?;

        let degree = u32::try_from(degree_i).map_err(|_| ConvertError::AttributeType {
            entity_id,
            field_name: "degree",
            expected: "non-negative Integer",
            actual: AttributeKindTag::Integer,
        })?;

        // 2D sister curve discrimination: if the first control point lives
        // in `point_2d_map`, this is a 2D PCURVE-side QUC instance and the
        // 3D handler should silently skip. (No 2D QUC handler exists yet;
        // such entries currently fall through to the warning path below.)
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
            kind: NurbsKind::NonRational,
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
            nurbs.weights().is_none(),
            "QuasiUniformCurveHandler::write expects a non-rational curve"
        );
        // Emit control-point references; reuse the existing point pool so
        // shared CARTESIAN_POINTs aren't duplicated.
        let mut cp_refs = Vec::with_capacity(nurbs.control_points.len());
        for &pid in &nurbs.control_points {
            cp_refs.push(buf.emit_point(pid)?);
        }
        let attrs = vec![
            Attribute::String(String::new()),
            Attribute::Integer(i64::from(nurbs.degree)),
            Attribute::List(cp_refs.into_iter().map(Attribute::EntityRef).collect()),
            Attribute::Enum(nurbs.form.as_step_enum().into()),
            Attribute::Enum(if nurbs.closed { "T".into() } else { "F".into() }),
            Attribute::Enum(logical_to_step(nurbs.self_intersect).into()),
        ];
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "QUASI_UNIFORM_CURVE".into(),
                attrs,
            },
        });
        Ok(n)
    }
}
