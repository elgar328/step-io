//! `(GRC PRC REP_CONTEXT)` unitless context handler — phase unitless-context.
//!
//! Matches complex MI instances composed of `GEOMETRIC_REPRESENTATION_CONTEXT`,
//! `PARAMETRIC_REPRESENTATION_CONTEXT`, and `REPRESENTATION_CONTEXT` parts —
//! and **without** a `GLOBAL_UNIT_ASSIGNED_CONTEXT` part. A 4-part complex
//! (`GRC+PRC+GUAC+REP_CONTEXT`) is intentionally **not** modelled here: the
//! GUAC handler claims that case, so we skip on read to avoid a double push.

use crate::entities::ComplexEntityHandler;
use crate::ir::attr::{check_count, read_integer, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::UnitlessContext;
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::step_entity_complex;

pub(crate) struct ParametricRepresentationContextHandler;

#[step_entity_complex(
    name = "PARAMETRIC_REPRESENTATION_CONTEXT",
    cases = [["GEOMETRIC_REPRESENTATION_CONTEXT", "PARAMETRIC_REPRESENTATION_CONTEXT", "REPRESENTATION_CONTEXT"]]
)]
impl ComplexEntityHandler for ParametricRepresentationContextHandler {
    type WriteInput = UnitlessContext;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        // Negative-part guard: if GUAC is present this is a unitful context
        // and the GUAC handler owns it.
        if parts
            .iter()
            .any(|p| p.name == "GLOBAL_UNIT_ASSIGNED_CONTEXT")
        {
            return Ok(());
        }
        let grc_attrs = require_part_attrs(parts, "GEOMETRIC_REPRESENTATION_CONTEXT", entity_id)?;
        check_count(grc_attrs, 1, entity_id, "GEOMETRIC_REPRESENTATION_CONTEXT")?;
        let coordinate_space_dimension =
            read_integer(grc_attrs, 0, entity_id, "coordinate_space_dimension")?;
        let rc_attrs = require_part_attrs(parts, "REPRESENTATION_CONTEXT", entity_id)?;
        check_count(rc_attrs, 2, entity_id, "REPRESENTATION_CONTEXT")?;
        let identifier =
            read_string_or_unset(rc_attrs, 0, entity_id, "context_identifier")?.to_owned();
        let context_type = read_string_or_unset(rc_attrs, 1, entity_id, "context_type")?.to_owned();
        let id = ctx.unitless_contexts.push(UnitlessContext {
            identifier,
            context_type,
            coordinate_space_dimension: Some(coordinate_space_dimension),
        });
        ctx.unitless_context_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, uc: UnitlessContext) -> Result<u64, WriteError> {
        // Complex MI parts emit in alphabetical order per ISO 10303-21:
        //   GEOMETRIC_REPRESENTATION_CONTEXT
        //   PARAMETRIC_REPRESENTATION_CONTEXT
        //   REPRESENTATION_CONTEXT
        // Only invoked for the complex form (dimension present); the plain
        // simple form is emitted by `RepresentationContextHandler`.
        let dimension = uc
            .coordinate_space_dimension
            .expect("parametric context carries a coordinate_space_dimension");
        let parts = vec![
            (
                "GEOMETRIC_REPRESENTATION_CONTEXT".into(),
                vec![Attribute::Integer(dimension)],
            ),
            ("PARAMETRIC_REPRESENTATION_CONTEXT".into(), vec![]),
            (
                "REPRESENTATION_CONTEXT".into(),
                vec![
                    Attribute::String(uc.identifier),
                    Attribute::String(uc.context_type),
                ],
            ),
        ];
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex { parts },
        });
        Ok(n)
    }
}
