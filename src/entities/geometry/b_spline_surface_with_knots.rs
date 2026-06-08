//! `B_SPLINE_SURFACE_WITH_KNOTS` handler leaf NURBS surface
//! (non-rational; rational form lives in `rational_bspline_surface.rs`).

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::nurbs_shared::build_surface_common;
use crate::ir::attr::{
    check_count, read_entity_ref_grid, read_enum, read_integer, read_integer_list, read_logical,
    read_real_list, read_string_or_unset,
};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::geometry::{NurbsSurface, NurbsSurfaceKind, Surface, SurfaceForm};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct BSplineSurfaceWithKnotsHandler;

#[step_entity(name = "B_SPLINE_SURFACE_WITH_KNOTS")]
impl SimpleEntityHandler for BSplineSurfaceWithKnotsHandler {
    type WriteInput = NurbsSurface;

    #[allow(clippy::too_many_lines)]
    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 13, entity_id, "B_SPLINE_SURFACE_WITH_KNOTS")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let u_degree_i = read_integer(attrs, 1, entity_id, "u_degree")?;
        let v_degree_i = read_integer(attrs, 2, entity_id, "v_degree")?;
        let cp_grid = read_entity_ref_grid(attrs, 3, entity_id, "control_points_list")?;
        let form = SurfaceForm::from_step_enum(read_enum(attrs, 4, entity_id, "surface_form")?);
        let u_closed = read_logical(attrs, 5, entity_id, "u_closed")?;
        let v_closed = read_logical(attrs, 6, entity_id, "v_closed")?;
        let self_intersect = read_logical(attrs, 7, entity_id, "self_intersect")?;
        let u_knot_multiplicities = read_integer_list(attrs, 8, entity_id, "u_multiplicities")?;
        let v_knot_multiplicities = read_integer_list(attrs, 9, entity_id, "v_multiplicities")?;
        let u_knots = read_real_list(attrs, 10, entity_id, "u_knots")?;
        let v_knots = read_real_list(attrs, 11, entity_id, "v_knots")?;
        // [12] knot_spec — informational, skipped

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
            "BSplineSurfaceWithKnotsHandler::write expects a non-rational surface"
        );
        let common = build_surface_common(buf, &nurbs)?;
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "B_SPLINE_SURFACE_WITH_KNOTS".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    common.u_degree,
                    common.v_degree,
                    common.cps,
                    common.form,
                    common.u_closed,
                    common.v_closed,
                    common.self_intersect,
                    common.u_mults,
                    common.v_mults,
                    common.u_knots,
                    common.v_knots,
                    common.knot_spec,
                ],
            },
        });
        Ok(n)
    }
}
