//! `RATIONAL_B_SPLINE_SURFACE` handler — Pass 4-2 complex NURBS surface.
//!
//! Mirrors the legacy `convert_rational_bspline_surface` and the
//! rational branch of the writer's `emit_nurbs_surface`. Shares the
//! `Pass4Rational` `PassLevel` with `RATIONAL_B_SPLINE_CURVE` — the two
//! complex entities key on different `REQUIRED_PARTS` so dispatch never
//! mistakes one for the other.

use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::attr::{
    logical_to_step, read_bool, read_entity_ref_grid, read_enum, read_integer, read_integer_list,
    read_logical, read_real_grid, read_real_list, read_string,
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

pub(crate) struct RationalBsplineSurfaceHandler;

#[step_entity_complex(name = "RATIONAL_B_SPLINE_SURFACE", pass = Pass4Rational, required = ["B_SPLINE_SURFACE",
        "B_SPLINE_SURFACE_WITH_KNOTS",
        "RATIONAL_B_SPLINE_SURFACE",])]
impl ComplexEntityHandler for RationalBsplineSurfaceHandler {
    type WriteInput = NurbsSurface;

    #[allow(clippy::too_many_lines)]
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

        let bswk_attrs = require_part_attrs(parts, "B_SPLINE_SURFACE_WITH_KNOTS", entity_id)?;
        let u_knot_multiplicities =
            read_integer_list(bswk_attrs, 0, entity_id, "u_multiplicities")?;
        let v_knot_multiplicities =
            read_integer_list(bswk_attrs, 1, entity_id, "v_multiplicities")?;
        let u_knots = read_real_list(bswk_attrs, 2, entity_id, "u_knots")?;
        let v_knots = read_real_list(bswk_attrs, 3, entity_id, "v_knots")?;

        let rat_attrs = require_part_attrs(parts, "RATIONAL_B_SPLINE_SURFACE", entity_id)?;
        let weights = read_real_grid(rat_attrs, 0, entity_id, "weights_data")?;

        // Validate weights 2D grid matches control points 2D grid.
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
        let NurbsSurfaceKind::Rational { weights } = nurbs.kind else {
            return Err(WriteError::UnsupportedIrVariant {
                detail: "RationalBsplineSurfaceHandler::write requires weights".into(),
            });
        };

        let mut cp_rows: Vec<Attribute> = Vec::with_capacity(nurbs.control_points.len());
        for row in &nurbs.control_points {
            let mut refs = Vec::with_capacity(row.len());
            for &pid in row {
                refs.push(Attribute::EntityRef(CartesianPointHandler::write(
                    buf, pid,
                )?));
            }
            cp_rows.push(Attribute::List(refs));
        }
        let cps_attr = Attribute::List(cp_rows);
        #[allow(clippy::cast_possible_wrap)]
        let u_deg = Attribute::Integer(i64::from(nurbs.u_degree));
        #[allow(clippy::cast_possible_wrap)]
        let v_deg = Attribute::Integer(i64::from(nurbs.v_degree));
        let u_closed = Attribute::Enum(if nurbs.u_closed {
            "T".into()
        } else {
            "F".into()
        });
        let v_closed = Attribute::Enum(if nurbs.v_closed {
            "T".into()
        } else {
            "F".into()
        });
        let u_mults_attr = Attribute::List(
            nurbs
                .u_knot_multiplicities
                .iter()
                .copied()
                .map(Attribute::Integer)
                .collect(),
        );
        let v_mults_attr = Attribute::List(
            nurbs
                .v_knot_multiplicities
                .iter()
                .copied()
                .map(Attribute::Integer)
                .collect(),
        );
        let u_knots_attr =
            Attribute::List(nurbs.u_knots.iter().copied().map(Attribute::Real).collect());
        let v_knots_attr =
            Attribute::List(nurbs.v_knots.iter().copied().map(Attribute::Real).collect());
        let form = nurbs.form;
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
                            u_deg,
                            v_deg,
                            cps_attr,
                            Attribute::Enum(form.as_step_enum().into()),
                            u_closed,
                            v_closed,
                            Attribute::Enum(logical_to_step(nurbs.self_intersect).into()),
                        ],
                    ),
                    (
                        "B_SPLINE_SURFACE_WITH_KNOTS".into(),
                        vec![
                            u_mults_attr,
                            v_mults_attr,
                            u_knots_attr,
                            v_knots_attr,
                            Attribute::Enum("UNSPECIFIED".into()),
                        ],
                    ),
                    ("GEOMETRIC_REPRESENTATION_ITEM".into(), vec![]),
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
