//! `pmi` pool entity handlers — Pass 8.
//!
//! Three dependency-free `single_struct` primitives — `TOLERANCE_ZONE_FORM`,
//! `TYPE_QUALIFIER`, `VALUE_FORMAT_TYPE_QUALIFIER` — each a 1-attr string
//! entity pushed into [`PmiPool`]. They have no entity references; the
//! GD&T entities that consume them arrive in later phases.

use crate::entities::SimpleEntityHandler;
use crate::ir::PmiPool;
use crate::ir::attr::{check_count, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::pmi::{ToleranceZoneForm, TypeQualifier, ValueFormatTypeQualifier};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ToleranceZoneFormHandler;

#[step_entity(name = "TOLERANCE_ZONE_FORM", pass = Pass8ShapeAspect)]
impl SimpleEntityHandler for ToleranceZoneFormHandler {
    type WriteInput = ToleranceZoneForm;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "TOLERANCE_ZONE_FORM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .tolerance_zone_forms
            .push(ToleranceZoneForm { name });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, tzf: ToleranceZoneForm) -> Result<u64, WriteError> {
        Ok(buf.push_simple("TOLERANCE_ZONE_FORM", vec![Attribute::String(tzf.name)]))
    }
}

pub(crate) struct TypeQualifierHandler;

#[step_entity(name = "TYPE_QUALIFIER", pass = Pass8ShapeAspect)]
impl SimpleEntityHandler for TypeQualifierHandler {
    type WriteInput = TypeQualifier;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "TYPE_QUALIFIER")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .type_qualifiers
            .push(TypeQualifier { name });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, tq: TypeQualifier) -> Result<u64, WriteError> {
        Ok(buf.push_simple("TYPE_QUALIFIER", vec![Attribute::String(tq.name)]))
    }
}

pub(crate) struct ValueFormatTypeQualifierHandler;

#[step_entity(name = "VALUE_FORMAT_TYPE_QUALIFIER", pass = Pass8ShapeAspect)]
impl SimpleEntityHandler for ValueFormatTypeQualifierHandler {
    type WriteInput = ValueFormatTypeQualifier;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "VALUE_FORMAT_TYPE_QUALIFIER")?;
        let format_type = read_string_or_unset(attrs, 0, entity_id, "format_type")?.to_owned();
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .value_format_type_qualifiers
            .push(ValueFormatTypeQualifier { format_type });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, vftq: ValueFormatTypeQualifier) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "VALUE_FORMAT_TYPE_QUALIFIER",
            vec![Attribute::String(vftq.format_type)],
        ))
    }
}
