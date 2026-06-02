//! `QUASI_UNIFORM_SURFACE` handler — simple, non-rational B-spline
//! surface with derived knot vectors + multiplicities (STEP
//! `QuasiUniformKnots` / `QuasiUniformKnotsMultiplicities` rules applied
//! to both u and v directions). Mirrors `B_SPLINE_SURFACE_WITH_KNOTS`
//! except the four knot/multiplicity attributes plus `knot_spec` are
//! absent on the wire and derived on read.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::nurbs_shared::quasi_uniform_knots;
use crate::ir::attr::{
    check_count, logical_to_step, read_bool, read_entity_ref_grid, read_enum, read_integer,
    read_logical, read_string_or_unset,
};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::geometry::{NurbsSurface, NurbsSurfaceKind, Surface, SurfaceForm};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct QuasiUniformSurfaceHandler;

#[step_entity(name = "QUASI_UNIFORM_SURFACE")]
impl SimpleEntityHandler for QuasiUniformSurfaceHandler {
    type WriteInput = NurbsSurface;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 8, entity_id, "QUASI_UNIFORM_SURFACE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let u_degree_i = read_integer(attrs, 1, entity_id, "u_degree")?;
        let v_degree_i = read_integer(attrs, 2, entity_id, "v_degree")?;
        let cp_grid = read_entity_ref_grid(attrs, 3, entity_id, "control_points_list")?;
        let form = SurfaceForm::from_step_enum(read_enum(attrs, 4, entity_id, "surface_form")?);
        let u_closed = read_bool(attrs, 5, entity_id, "u_closed")?;
        let v_closed = read_bool(attrs, 6, entity_id, "v_closed")?;
        let self_intersect = read_logical(attrs, 7, entity_id, "self_intersect")?;

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
            kind: NurbsSurfaceKind::NonRational,
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
        debug_assert!(
            nurbs.weights().is_none(),
            "QuasiUniformSurfaceHandler::write expects a non-rational surface"
        );
        let mut cp_rows: Vec<Attribute> = Vec::with_capacity(nurbs.control_points.len());
        for row in &nurbs.control_points {
            let mut refs = Vec::with_capacity(row.len());
            for &pid in row {
                refs.push(Attribute::EntityRef(buf.emit_point(pid)?));
            }
            cp_rows.push(Attribute::List(refs));
        }
        let attrs = vec![
            Attribute::String(String::new()),
            Attribute::Integer(i64::from(nurbs.u_degree)),
            Attribute::Integer(i64::from(nurbs.v_degree)),
            Attribute::List(cp_rows),
            Attribute::Enum(nurbs.form.as_step_enum().into()),
            Attribute::Enum(if nurbs.u_closed {
                "T".into()
            } else {
                "F".into()
            }),
            Attribute::Enum(if nurbs.v_closed {
                "T".into()
            } else {
                "F".into()
            }),
            Attribute::Enum(logical_to_step(nurbs.self_intersect).into()),
        ];
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "QUASI_UNIFORM_SURFACE".into(),
                attrs,
            },
        });
        Ok(n)
    }
}
