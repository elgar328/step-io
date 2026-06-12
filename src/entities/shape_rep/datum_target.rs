//! `DATUM_TARGET` handler (alongside the other
//! `SHAPE_ASPECT` subtypes). 5-attr body: the shared `shape_aspect` four
//! plus `target_id` (an alphabetic label such as `"A1"`). The ir.toml
//! blueprint folds this entity into the `shape_aspect` arena under
//! `ShapeAspectId`; step-io keeps a dedicated `datum_targets` arena like
//! every other shape-aspect subtype and exposes it through the
//! [`ShapeAspectRef::DatumTarget`] enum variant.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::DatumTarget;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct DatumTargetHandler;

#[step_entity(name = "DATUM_TARGET")]
impl SimpleEntityHandler for DatumTargetHandler {
    type WriteInput = DatumTarget;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 5, entity_id, "DATUM_TARGET")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let of_shape_ref = read_entity_ref(attrs, 2, entity_id, "of_shape")?;
        let product_definitional = read_bool(attrs, 3, entity_id, "product_definitional")?;
        let target_id = read_string_or_unset(attrs, 4, entity_id, "target_id")?.to_owned();

        // of_shape → PRODUCT_DEFINITION_SHAPE → ProductId (typed one-probe).
        let Some(target) = ctx.product_of_pds(of_shape_ref) else {
            return Ok(());
        };

        let id = ctx.datum_targets.push(DatumTarget {
            name,
            description,
            target,
            product_definitional,
            target_id,
        });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dt: DatumTarget) -> Result<u64, WriteError> {
        let pds_step_id = buf
            .product_def_shape_ids
            .get(&dt.target)
            .copied()
            .unwrap_or(0);
        let bool_attr = if dt.product_definitional { "T" } else { "F" };
        Ok(buf.push_simple(
            "DATUM_TARGET",
            vec![
                Attribute::String(dt.name),
                Attribute::String(dt.description),
                Attribute::EntityRef(pds_step_id),
                Attribute::Enum(bool_attr.into()),
                Attribute::String(dt.target_id),
            ],
        ))
    }
}
