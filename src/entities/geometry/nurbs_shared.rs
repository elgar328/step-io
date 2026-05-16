//! Shared writer helpers for the four NURBS handlers
//! (`B_SPLINE_CURVE_WITH_KNOTS`, `RATIONAL_B_SPLINE_CURVE`,
//! `B_SPLINE_SURFACE_WITH_KNOTS`, `RATIONAL_B_SPLINE_SURFACE`).
//!
//! Each handler still owns the simple-vs-complex body assembly; only the
//! attribute building that is identical across the rational/non-rational
//! pair lives here.

use crate::ir::attr::logical_to_step;
use crate::ir::geometry::{NurbsCurve, NurbsSurface};
use crate::parser::entity::Attribute;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

pub(super) struct CurveCommonAttrs {
    pub degree: Attribute,
    pub cps: Attribute,
    pub form: Attribute,
    pub closed: Attribute,
    pub self_intersect: Attribute,
    pub mults: Attribute,
    pub knots: Attribute,
    pub knot_spec: Attribute,
}

pub(super) fn build_curve_common(
    buf: &mut WriteBuffer,
    nurbs: &NurbsCurve,
) -> Result<CurveCommonAttrs, WriteError> {
    let mut cp_refs = Vec::with_capacity(nurbs.control_points.len());
    for &pid in &nurbs.control_points {
        cp_refs.push(buf.emit_point(pid)?);
    }
    #[allow(clippy::cast_possible_wrap)]
    let degree = Attribute::Integer(i64::from(nurbs.degree));
    Ok(CurveCommonAttrs {
        degree,
        cps: Attribute::List(cp_refs.into_iter().map(Attribute::EntityRef).collect()),
        form: Attribute::Enum(nurbs.form.as_step_enum().into()),
        closed: Attribute::Enum(if nurbs.closed { "T".into() } else { "F".into() }),
        self_intersect: Attribute::Enum(logical_to_step(nurbs.self_intersect).into()),
        mults: Attribute::List(
            nurbs
                .knot_multiplicities
                .iter()
                .copied()
                .map(Attribute::Integer)
                .collect(),
        ),
        knots: Attribute::List(nurbs.knots.iter().copied().map(Attribute::Real).collect()),
        knot_spec: Attribute::Enum("UNSPECIFIED".into()),
    })
}

pub(super) struct SurfaceCommonAttrs {
    pub u_degree: Attribute,
    pub v_degree: Attribute,
    pub cps: Attribute,
    pub form: Attribute,
    pub u_closed: Attribute,
    pub v_closed: Attribute,
    pub self_intersect: Attribute,
    pub u_mults: Attribute,
    pub v_mults: Attribute,
    pub u_knots: Attribute,
    pub v_knots: Attribute,
    pub knot_spec: Attribute,
}

pub(super) fn build_surface_common(
    buf: &mut WriteBuffer,
    nurbs: &NurbsSurface,
) -> Result<SurfaceCommonAttrs, WriteError> {
    let mut cp_rows: Vec<Attribute> = Vec::with_capacity(nurbs.control_points.len());
    for row in &nurbs.control_points {
        let mut refs = Vec::with_capacity(row.len());
        for &pid in row {
            refs.push(Attribute::EntityRef(buf.emit_point(pid)?));
        }
        cp_rows.push(Attribute::List(refs));
    }
    #[allow(clippy::cast_possible_wrap)]
    let u_degree = Attribute::Integer(i64::from(nurbs.u_degree));
    #[allow(clippy::cast_possible_wrap)]
    let v_degree = Attribute::Integer(i64::from(nurbs.v_degree));
    Ok(SurfaceCommonAttrs {
        u_degree,
        v_degree,
        cps: Attribute::List(cp_rows),
        form: Attribute::Enum(nurbs.form.as_step_enum().into()),
        u_closed: Attribute::Enum(if nurbs.u_closed {
            "T".into()
        } else {
            "F".into()
        }),
        v_closed: Attribute::Enum(if nurbs.v_closed {
            "T".into()
        } else {
            "F".into()
        }),
        self_intersect: Attribute::Enum(logical_to_step(nurbs.self_intersect).into()),
        u_mults: Attribute::List(
            nurbs
                .u_knot_multiplicities
                .iter()
                .copied()
                .map(Attribute::Integer)
                .collect(),
        ),
        v_mults: Attribute::List(
            nurbs
                .v_knot_multiplicities
                .iter()
                .copied()
                .map(Attribute::Integer)
                .collect(),
        ),
        u_knots: Attribute::List(nurbs.u_knots.iter().copied().map(Attribute::Real).collect()),
        v_knots: Attribute::List(nurbs.v_knots.iter().copied().map(Attribute::Real).collect()),
        knot_spec: Attribute::Enum("UNSPECIFIED".into()),
    })
}
