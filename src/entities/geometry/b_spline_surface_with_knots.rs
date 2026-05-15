//! `B_SPLINE_SURFACE_WITH_KNOTS` handler — Pass 4-1 leaf NURBS surface
//! (non-rational; rational form lives in `rational_bspline_surface.rs`).

use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::attr::{
    check_count, logical_to_step, read_bool, read_entity_ref_grid, read_enum, read_integer,
    read_integer_list, read_logical, read_real_list, read_string,
};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::geometry::{NurbsSurface, Surface, SurfaceForm};
use crate::parser::entity::Attribute;
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct BSplineSurfaceWithKnotsHandler;

impl SimpleEntityHandler for BSplineSurfaceWithKnotsHandler {
    const NAME: &'static str = "B_SPLINE_SURFACE_WITH_KNOTS";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4Leaf;
    type WriteInput = NurbsSurface;

    #[allow(clippy::too_many_lines)]
    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
    ) -> Result<(), ConvertError> {
        check_count(attrs, 13, entity_id, "B_SPLINE_SURFACE_WITH_KNOTS")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let u_degree_i = read_integer(attrs, 1, entity_id, "u_degree")?;
        let v_degree_i = read_integer(attrs, 2, entity_id, "v_degree")?;
        let cp_grid = read_entity_ref_grid(attrs, 3, entity_id, "control_points_list")?;
        let form = SurfaceForm::from_step_enum(read_enum(attrs, 4, entity_id, "surface_form")?);
        let u_closed = read_bool(attrs, 5, entity_id, "u_closed")?;
        let v_closed = read_bool(attrs, 6, entity_id, "v_closed")?;
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
            weights: None,
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
            nurbs.weights.is_none(),
            "BSplineSurfaceWithKnotsHandler::write expects a non-rational surface"
        );
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
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "B_SPLINE_SURFACE_WITH_KNOTS".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    u_deg,
                    v_deg,
                    cps_attr,
                    Attribute::Enum(form.as_step_enum().into()),
                    u_closed,
                    v_closed,
                    Attribute::Enum(logical_to_step(nurbs.self_intersect).into()),
                    u_mults_attr,
                    v_mults_attr,
                    u_knots_attr,
                    v_knots_attr,
                    Attribute::Enum("UNSPECIFIED".into()),
                ],
            },
        });
        Ok(n)
    }
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static B_SPLINE_SURFACE_WITH_KNOTS_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: BSplineSurfaceWithKnotsHandler::NAME,
    pass_level: BSplineSurfaceWithKnotsHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: BSplineSurfaceWithKnotsHandler::read,
    },
};
