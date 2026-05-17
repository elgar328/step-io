//! Complex `RATIONAL_B_SPLINE_SURFACE` carrying a `QUASI_UNIFORM_SURFACE`
//! marker. Mirrors `rational_bspline_surface` except the u / v knot
//! vectors + multiplicities are derived from `(degree, cp_count)` per
//! the STEP `QuasiUniformKnots` / `QuasiUniformKnotsMultiplicities`
//! rules; required parts therefore drop `B_SPLINE_SURFACE_WITH_KNOTS`
//! and add `QUASI_UNIFORM_SURFACE`.

use crate::entities::ComplexEntityHandler;
use crate::entities::geometry::nurbs_shared::{build_surface_common, quasi_uniform_knots};
use crate::ir::attr::{
    read_bool, read_entity_ref_grid, read_enum, read_integer, read_logical, read_real_grid,
    read_string,
};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::geometry::{NurbsSurface, NurbsSurfaceKind, Surface, SurfaceForm};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::reader::require_part_attrs;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct RationalQuasiUniformSurfaceHandler;

#[step_entity_complex(
    name = "RATIONAL_B_SPLINE_SURFACE",
    pass = Pass4Rational,
    required = ["B_SPLINE_SURFACE", "QUASI_UNIFORM_SURFACE", "RATIONAL_B_SPLINE_SURFACE"]
)]
impl ComplexEntityHandler for RationalQuasiUniformSurfaceHandler {
    type WriteInput = NurbsSurface;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let repr_attrs = require_part_attrs(parts, "REPRESENTATION_ITEM", entity_id)?;
        let _name = read_string(repr_attrs, 0, entity_id, "name")?;

        let bss_attrs = require_part_attrs(parts, "B_SPLINE_SURFACE", entity_id)?;
        let u_degree_i = read_integer(bss_attrs, 0, entity_id, "u_degree")?;
        let v_degree_i = read_integer(bss_attrs, 1, entity_id, "v_degree")?;
        let cp_grid = read_entity_ref_grid(bss_attrs, 2, entity_id, "control_points_list")?;
        let form = SurfaceForm::from_step_enum(read_enum(bss_attrs, 3, entity_id, "surface_form")?);
        let u_closed = read_bool(bss_attrs, 4, entity_id, "u_closed")?;
        let v_closed = read_bool(bss_attrs, 5, entity_id, "v_closed")?;
        let self_intersect = read_logical(bss_attrs, 6, entity_id, "self_intersect")?;

        let rat_attrs = require_part_attrs(parts, "RATIONAL_B_SPLINE_SURFACE", entity_id)?;
        let weights = read_real_grid(rat_attrs, 0, entity_id, "weights_data")?;

        if weights.len() != cp_grid.len() {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "weights_data",
                expected: cp_grid.len(),
                actual: weights.len(),
            });
        }
        for (w_row, cp_row) in weights.iter().zip(cp_grid.iter()) {
            if w_row.len() != cp_row.len() {
                return Err(ConvertError::DimensionMismatch {
                    entity_id,
                    field_name: "weights_data",
                    expected: cp_row.len(),
                    actual: w_row.len(),
                });
            }
        }

        let u_degree = u32::try_from(u_degree_i).map_err(|_| ConvertError::AttributeType {
            entity_id,
            field_name: "u_degree",
            expected: "non-negative Integer",
            actual: AttributeKindTag::Integer,
        })?;
        let v_degree = u32::try_from(v_degree_i).map_err(|_| ConvertError::AttributeType {
            entity_id,
            field_name: "v_degree",
            expected: "non-negative Integer",
            actual: AttributeKindTag::Integer,
        })?;

        let mut control_points = Vec::with_capacity(cp_grid.len());
        for row in &cp_grid {
            let mut pt_row = Vec::with_capacity(row.len());
            for &r in row {
                let pt = ctx.resolve_point(entity_id, r, "control_points_list")?;
                pt_row.push(pt);
            }
            control_points.push(pt_row);
        }

        let u_cp_count = control_points.len();
        let v_cp_count = control_points.first().map_or(0, Vec::len);
        let Some((u_knot_multiplicities, u_knots)) = quasi_uniform_knots(u_degree, u_cp_count)
        else {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "control_points_list",
                expected: u_degree as usize + 1,
                actual: u_cp_count,
            });
        };
        let Some((v_knot_multiplicities, v_knots)) = quasi_uniform_knots(v_degree, v_cp_count)
        else {
            return Err(ConvertError::DimensionMismatch {
                entity_id,
                field_name: "control_points_list",
                expected: v_degree as usize + 1,
                actual: v_cp_count,
            });
        };

        let surface = NurbsSurface {
            u_degree,
            v_degree,
            control_points,
            kind: NurbsSurfaceKind::Rational { weights },
            u_knot_multiplicities,
            v_knot_multiplicities,
            u_knots,
            v_knots,
            u_closed,
            v_closed,
            form,
            self_intersect,
        };
        let id = ctx.geometry.surfaces.push(Surface::Nurbs(surface));
        ctx.surface_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, nurbs: NurbsSurface) -> Result<u64, WriteError> {
        let common = build_surface_common(buf, &nurbs)?;
        let NurbsSurfaceKind::Rational { weights } = nurbs.kind else {
            return Err(WriteError::UnsupportedIrVariant {
                detail: "RationalQuasiUniformSurfaceHandler::write requires weights".into(),
            });
        };
        let mut w_rows: Vec<Attribute> = Vec::with_capacity(weights.len());
        for row in weights {
            w_rows.push(Attribute::List(
                row.into_iter().map(Attribute::Real).collect(),
            ));
        }
        let weights_attr = Attribute::List(w_rows);

        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex {
                parts: vec![
                    ("BOUNDED_SURFACE".into(), vec![]),
                    (
                        "B_SPLINE_SURFACE".into(),
                        vec![
                            common.u_degree,
                            common.v_degree,
                            common.cps,
                            common.form,
                            common.u_closed,
                            common.v_closed,
                            common.self_intersect,
                        ],
                    ),
                    ("GEOMETRIC_REPRESENTATION_ITEM".into(), vec![]),
                    ("QUASI_UNIFORM_SURFACE".into(), vec![]),
                    ("RATIONAL_B_SPLINE_SURFACE".into(), vec![weights_attr]),
                    (
                        "REPRESENTATION_ITEM".into(),
                        vec![Attribute::String(String::new())],
                    ),
                    ("SURFACE".into(), vec![]),
                ],
            },
        });
        Ok(n)
    }
}
