//! `NEXT_ASSEMBLY_USAGE_OCCURRENCE` handler — Pass 6-8.
//!
//! Reader resolves the parent / child PDEFs through `pdef_to_product` and
//! pushes a fully-formed `Instance` into the parent product's `Group`
//! content. Transform comes from `nauo_transform_map` populated by the
//! CDSR + RRWT path; missing transforms surface as warnings (rare in
//! commercial fixtures). Writer emits the bare NAUO line; the larger
//! `emit_instance_bundle` orchestrator handles the surrounding
//! `PRODUCT_DEFINITION_SHAPE` + `RR_complex` + CDSR group.

use crate::entities::SimpleEntityHandler;
use crate::ir::assembly::{Instance, ProductContent};
use crate::ir::attr::{check_count, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct NextAssemblyUsageOccurrenceWriteInput {
    pub(crate) inst: Instance,
    pub(crate) parent_pdef: u64,
    pub(crate) child_pdef: u64,
}

pub(crate) struct NextAssemblyUsageOccurrenceHandler;

#[step_entity(name = "NEXT_ASSEMBLY_USAGE_OCCURRENCE", pass = Pass6Nauo)]
impl SimpleEntityHandler for NextAssemblyUsageOccurrenceHandler {
    type WriteInput = NextAssemblyUsageOccurrenceWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 6, entity_id, "NEXT_ASSEMBLY_USAGE_OCCURRENCE")?;
        let occurrence_id = read_string_or_unset(attrs, 0, entity_id, "id")?.to_owned();
        let occurrence_name = read_string_or_unset(attrs, 1, entity_id, "name")?.to_owned();
        // attrs[2] = description, attrs[5] = reference_designator — ignored.
        let relating_pdef = read_entity_ref(attrs, 3, entity_id, "relating_pdef")?;
        let related_pdef = read_entity_ref(attrs, 4, entity_id, "related_pdef")?;

        let parent_pid = ctx.resolve_product_by_pdef(entity_id, relating_pdef, "relating_pdef")?;
        let child_pid = ctx.resolve_product_by_pdef(entity_id, related_pdef, "related_pdef")?;

        let Some(&transform) = ctx.nauo_transform_map.get(&entity_id) else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: String::from("NEXT_ASSEMBLY_USAGE_OCCURRENCE with no transform found"),
            });
            return Ok(());
        };

        match &mut ctx.assembly_products[parent_pid].content {
            ProductContent::Group(instances) => {
                instances.push(Instance {
                    child: child_pid,
                    transform,
                    occurrence_id,
                    occurrence_name,
                });
            }
            ProductContent::Solid(_)
            | ProductContent::SurfaceBody(_)
            | ProductContent::Wireframe(_) => {
                ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                    entity_id,
                    detail: String::from(
                        "NEXT_ASSEMBLY_USAGE_OCCURRENCE parent is a geometry leaf, not a Group",
                    ),
                });
            }
        }
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        NextAssemblyUsageOccurrenceWriteInput {
            inst,
            parent_pdef,
            child_pdef,
        }: NextAssemblyUsageOccurrenceWriteInput,
    ) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "NEXT_ASSEMBLY_USAGE_OCCURRENCE",
            vec![
                Attribute::String(inst.occurrence_id),
                Attribute::String(inst.occurrence_name),
                Attribute::String(String::new()),
                Attribute::EntityRef(parent_pdef),
                Attribute::EntityRef(child_pdef),
                Attribute::Unset,
            ],
        ))
    }
}
