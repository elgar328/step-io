//! `DATUM_SYSTEM` handler.
//!
//! `DATUM_SYSTEM` is a `SHAPE_ASPECT` subtype: the 4-attr shape-aspect body
//! (name, description, `of_shape`, `product_definitional`) plus a
//! `constituents` LIST of `DATUM_REFERENCE_COMPARTMENT` references. The
//! ir.toml blueprint folds it into the `shape_aspect` arena; step-io keeps a
//! dedicated `datum_systems` arena like every other shape-aspect subtype.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref, read_entity_ref_list, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::DatumSystem;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DatumSystemHandler;

#[step_entity(name = "DATUM_SYSTEM")]
impl SimpleEntityHandler for DatumSystemHandler {
    type WriteInput = DatumSystem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 5, entity_id, "DATUM_SYSTEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let of_shape_ref = read_entity_ref(attrs, 2, entity_id, "of_shape")?;
        let product_definitional = read_bool(attrs, 3, entity_id, "product_definitional")?;
        let constituent_refs = read_entity_ref_list(attrs, 4, entity_id, "constituents")?;

        // of_shape → PRODUCT_DEFINITION_SHAPE → PRODUCT_DEFINITION → ProductId.
        let Some(&pdef_step_id) = ctx.pdef_shape_to_pdef.get(&of_shape_ref) else {
            return Ok(());
        };
        let Some(&product_step_id) = ctx.pdef_to_product.get(&pdef_step_id) else {
            return Ok(());
        };
        let Some(&target) = ctx.product_arena_map.get(&product_step_id) else {
            return Ok(());
        };

        // constituents — `DATUM_REFERENCE_COMPARTMENT` refs. An individual
        // ref that does not resolve is skipped (symmetric on re-read).
        let mut constituents = Vec::with_capacity(constituent_refs.len());
        for r in constituent_refs {
            if let Some(&id) = ctx.general_datum_reference_id_map.get(&r) {
                constituents.push(id);
            }
        }

        let id = ctx.datum_systems.push(DatumSystem {
            name,
            description,
            target,
            product_definitional,
            constituents,
        });
        ctx.datum_system_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ds: DatumSystem) -> Result<u64, WriteError> {
        // `target` → PRODUCT_DEFINITION_SHAPE step id; a miss is the
        // kernel-built IR defensive case, in practice unreachable.
        let pds_step_id = buf
            .product_def_shape_ids
            .get(&ds.target)
            .copied()
            .unwrap_or(0);
        let mut constituent_refs = Vec::with_capacity(ds.constituents.len());
        for gdr_id in &ds.constituents {
            constituent_refs.push(Attribute::EntityRef(
                buf.general_datum_reference_step_ids[gdr_id.0 as usize],
            ));
        }
        let bool_attr = if ds.product_definitional { "T" } else { "F" };
        Ok(buf.push_simple(
            "DATUM_SYSTEM",
            vec![
                Attribute::String(ds.name),
                Attribute::String(ds.description),
                Attribute::EntityRef(pds_step_id),
                Attribute::Enum(bool_attr.into()),
                Attribute::List(constituent_refs),
            ],
        ))
    }
}
