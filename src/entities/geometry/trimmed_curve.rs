//! `TRIMMED_CURVE` handler — Pass 4-3c.
//!
//! Mirrors `ReaderContext::convert_trimmed_curve` and
//! `WriteBuffer::emit_trimmed_curve`.

use crate::entities::SimpleEntityHandler;
use crate::entities::geometry::cartesian_point::CartesianPointHandler;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_enum, read_string_or_unset};
use crate::ir::error::{AttributeKindTag, ConvertError};
use crate::ir::geometry::{Curve, TrimMaster, TrimSelect, TrimmedCurve};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity;

pub(crate) struct TrimmedCurveHandler;

#[step_entity(name = "TRIMMED_CURVE")]
impl SimpleEntityHandler for TrimmedCurveHandler {
    type WriteInput = TrimmedCurve;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 6, entity_id, "TRIMMED_CURVE")?;
        let _name = read_string_or_unset(attrs, 0, entity_id, "name")?;
        let basis_ref = read_entity_ref(attrs, 1, entity_id, "basis_curve")?;
        let basis = ctx.resolve_curve(entity_id, basis_ref, "basis_curve")?;
        let trim_1 = read_trim_select(ctx, attrs, 2, entity_id, "trim_1")?;
        let trim_2 = read_trim_select(ctx, attrs, 3, entity_id, "trim_2")?;
        let sense_agreement = read_bool(attrs, 4, entity_id, "sense_agreement")?;
        let master = read_enum(attrs, 5, entity_id, "master_representation")?;
        let master = match master {
            "CARTESIAN" => TrimMaster::Cartesian,
            "PARAMETER" => TrimMaster::Parameter,
            _ => TrimMaster::Unspecified,
        };

        let trimmed = TrimmedCurve {
            basis,
            trim_1,
            trim_2,
            sense_agreement,
            master,
        };
        let id = ctx.geometry.curves.push(Curve::Trimmed(trimmed));
        ctx.curve_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, trimmed: TrimmedCurve) -> Result<u64, WriteError> {
        let basis = buf.emit_curve(trimmed.basis)?;
        let trim_1 = build_trim_select(buf, &trimmed.trim_1)?;
        let trim_2 = build_trim_select(buf, &trimmed.trim_2)?;
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
/// The SET may be empty (reader accepts; round-trip preserved).
fn read_trim_select(
    ctx: &ReaderContext,
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<Vec<TrimSelect>, ConvertError> {
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
    let mut out = Vec::new();
    for el in elements {
        match el {
            Attribute::EntityRef(r) => {
                if let Some(&pid) = ctx.point_map.get(r) {
                    out.push(TrimSelect::Point(pid));
                }
            }
            Attribute::Typed { type_name, value } if type_name == "PARAMETER_VALUE" => {
                if let Attribute::Real(v) = **value {
                    out.push(TrimSelect::Param(v));
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Build the SET-of-trim_select attribute list for a `TRIMMED_CURVE` slot.
/// The writer emits whatever the IR carries, faithfully (including empty).
fn build_trim_select(
    buf: &mut WriteBuffer,
    items: &[TrimSelect],
) -> Result<Vec<Attribute>, WriteError> {
    let mut elements = Vec::with_capacity(items.len());
    for sel in items {
        match *sel {
            TrimSelect::Point(p) => {
                elements.push(Attribute::EntityRef(CartesianPointHandler::write(buf, p)?));
            }
            TrimSelect::Param(v) => {
                elements.push(Attribute::Typed {
                    type_name: "PARAMETER_VALUE".into(),
                    value: Box::new(Attribute::Real(v)),
                });
            }
        }
    }
    Ok(elements)
}
