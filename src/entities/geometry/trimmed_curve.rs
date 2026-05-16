//! `TRIMMED_CURVE` handler — Pass 4-3c.
//!
//! Mirrors `ReaderContext::convert_trimmed_curve` and
//! `WriteBuffer::emit_trimmed_curve`.

use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::entities::{
    ENTITY_HANDLERS, EntityHandlerEntry, PassLevel, ReadKind, SimpleEntityHandler,
};
use crate::ir::PointId;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_enum, read_string};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::geometry::{Curve, TrimMaster, TrimmedCurve};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};

pub(crate) struct TrimmedCurveHandler;

impl SimpleEntityHandler for TrimmedCurveHandler {
    const NAME: &'static str = "TRIMMED_CURVE";
    const PASS_LEVEL: PassLevel = PassLevel::Pass4_3cTrimSeg;
    type WriteInput = TrimmedCurve;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 6, entity_id, "TRIMMED_CURVE")?;
        let _name = read_string(attrs, 0, entity_id, "name")?;
        let basis_ref = read_entity_ref(attrs, 1, entity_id, "basis_curve")?;
        let basis = ctx.resolve_curve(entity_id, basis_ref, "basis_curve")?;
        let (trim_1_param, trim_1_point) = read_trim_select(ctx, attrs, 2, entity_id, "trim_1")?;
        let (trim_2_param, trim_2_point) = read_trim_select(ctx, attrs, 3, entity_id, "trim_2")?;
        let sense_agreement = read_bool(attrs, 4, entity_id, "sense_agreement")?;
        let master = read_enum(attrs, 5, entity_id, "master_representation")?;
        let master = match master {
            "CARTESIAN" => TrimMaster::Cartesian,
            "PARAMETER" => TrimMaster::Parameter,
            _ => TrimMaster::Unspecified,
        };

        let trimmed = TrimmedCurve {
            basis,
            trim_1_param,
            trim_1_point,
            trim_2_param,
            trim_2_point,
            sense_agreement,
            master,
        };
        let id = ctx.geometry.curves.push(Curve::Trimmed(trimmed));
        ctx.curve_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, trimmed: TrimmedCurve) -> Result<u64, WriteError> {
        let basis = buf.emit_curve(trimmed.basis)?;
        let trim_1 = build_trim_select(buf, trimmed.trim_1_point, trimmed.trim_1_param)?;
        let trim_2 = build_trim_select(buf, trimmed.trim_2_point, trimmed.trim_2_param)?;
        let master = match trimmed.master {
            TrimMaster::Cartesian => "CARTESIAN",
            TrimMaster::Parameter => "PARAMETER",
            TrimMaster::Unspecified => "UNSPECIFIED",
        };
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Simple {
                name: "TRIMMED_CURVE".into(),
                attrs: vec![
                    Attribute::String(String::new()),
                    Attribute::EntityRef(basis),
                    Attribute::List(trim_1),
                    Attribute::List(trim_2),
                    Attribute::Enum(if trimmed.sense_agreement { "T" } else { "F" }.into()),
                    Attribute::Enum(master.into()),
                ],
            },
        });
        Ok(n)
    }
}

/// Decode a `TRIMMED_CURVE` trim slot — a SET (`Attribute::List`) of
/// `CARTESIAN_POINT` refs and `PARAMETER_VALUE(real)` typed parameters.
/// Either, both, or neither may appear; missing values come back as
/// `None`.
fn read_trim_select(
    ctx: &ReaderContext,
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<(Option<f64>, Option<PointId>), ConvertError> {
    let Attribute::List(elements) = attrs.get(index).ok_or(ConvertError::AttributeCount {
        entity_id,
        entity_name: "TRIMMED_CURVE".into(),
        expected: index + 1,
        actual: attrs.len(),
    })?
    else {
        return Err(ConvertError::AttributeType {
            entity_id,
            field_name,
            expected: "List",
            actual: AttributeKindTag::from_attribute(attrs.get(index).unwrap()),
        });
    };
    let mut param = None;
    let mut point = None;
    for el in elements {
        match el {
            Attribute::EntityRef(r) => {
                if let Some(&pid) = ctx.point_map.get(r) {
                    point = Some(pid);
                }
            }
            Attribute::Typed { type_name, value } if type_name == "PARAMETER_VALUE" => {
                if let Attribute::Real(v) = **value {
                    param = Some(v);
                }
            }
            _ => {}
        }
    }
    Ok((param, point))
}

/// Build the SET-of-trim_select attribute list for a `TRIMMED_CURVE` slot.
/// Either, both, or neither of the cartesian point and parameter value may
/// be present; the writer emits whatever the IR carries, faithfully.
fn build_trim_select(
    buf: &mut WriteBuffer,
    point: Option<PointId>,
    param: Option<f64>,
) -> Result<Vec<Attribute>, WriteError> {
    let mut elements = Vec::new();
    if let Some(p) = point {
        elements.push(Attribute::EntityRef(CartesianPointHandler::write(buf, p)?));
    }
    if let Some(v) = param {
        elements.push(Attribute::Typed {
            type_name: "PARAMETER_VALUE".into(),
            value: Box::new(Attribute::Real(v)),
        });
    }
    Ok(elements)
}

#[allow(unsafe_code)] // linkme uses link_section internally
#[linkme::distributed_slice(ENTITY_HANDLERS)]
static TRIMMED_CURVE_HANDLER_ENTRY: EntityHandlerEntry = EntityHandlerEntry {
    name: TrimmedCurveHandler::NAME,
    pass_level: TrimmedCurveHandler::PASS_LEVEL,
    kind: ReadKind::Simple {
        read: TrimmedCurveHandler::read,
    },
};
