//! `NEXT_ASSEMBLY_USAGE_OCCURRENCE` handler.
//!
//! Reader binds the L1 shape and defers the instance to the
//! `resolve_nauo_instances` post-pass (see `lower`). Transform comes from
//! the CDSR + RRWT path; missing transforms surface as warnings (rare in
//! commercial fixtures). Writer emits the bare NAUO line from pre-resolved
//! fields; the larger `emit_instance_bundle` orchestrator handles the
//! surrounding `PRODUCT_DEFINITION_SHAPE` + `RR_complex` + CDSR group.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

/// Pre-resolved NAUO attrs. `emit_nauo` builds this either from the canonical
/// `assembly_component_usage` arena entry (reader-built, faithful round-trip)
/// or from the `Instance` fields (kernel-built: empty description, no
/// reference designator) — both paths emit through this one handler.
pub(crate) struct NextAssemblyUsageOccurrenceWriteInput {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) reference_designator: Option<String>,
    pub(crate) relating: u64,
    pub(crate) related: u64,
}

pub(crate) struct NextAssemblyUsageOccurrenceHandler;

#[step_entity(name = "NEXT_ASSEMBLY_USAGE_OCCURRENCE")]
impl SimpleEntityHandler for NextAssemblyUsageOccurrenceHandler {
    type WriteInput = NextAssemblyUsageOccurrenceWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_next_assembly_usage_occurrence(entity_id, attrs)?;
        lower::lower_next_assembly_usage_occurrence(ctx, entity_id, early)
    }

    fn write(
        buf: &mut WriteBuffer,
        input: NextAssemblyUsageOccurrenceWriteInput,
    ) -> Result<u64, WriteError> {
        let early = lift::lift_next_assembly_usage_occurrence(input);
        Ok(serialize::serialize_next_assembly_usage_occurrence(
            buf, &early,
        ))
    }
}
