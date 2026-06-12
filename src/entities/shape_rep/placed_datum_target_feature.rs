//! `PLACED_DATUM_TARGET_FEATURE` handler. Same 5-attr
//! shape as [`DatumTargetHandler`] (the `target_id` field is inherited
//! from `datum_target`); the entity is its own ir.toml `shape_aspect`
//! variant. step-io keeps a dedicated `placed_datum_target_features`
//! arena and exposes it through the
//! [`ShapeAspectRef::PlacedDatumTargetFeature`] enum variant.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_bool, read_entity_ref, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::shape_rep::PlacedDatumTargetFeature;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PlacedDatumTargetFeatureHandler;

#[step_entity(name = "PLACED_DATUM_TARGET_FEATURE")]
impl SimpleEntityHandler for PlacedDatumTargetFeatureHandler {
    type WriteInput = PlacedDatumTargetFeature;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 5, entity_id, "PLACED_DATUM_TARGET_FEATURE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let of_shape_ref = read_entity_ref(attrs, 2, entity_id, "of_shape")?;
        let product_definitional = read_bool(attrs, 3, entity_id, "product_definitional")?;
        let target_id = read_string_or_unset(attrs, 4, entity_id, "target_id")?.to_owned();

        let Some(&pdef_step_id) = ctx.pdef_shape_to_pdef.get(&of_shape_ref) else {
            return Ok(());
        };
        let Some(target) = ctx.product_of_pdef(pdef_step_id) else {
            return Ok(());
        };

        let id = ctx
            .placed_datum_target_features
            .push(PlacedDatumTargetFeature {
                name,
                description,
                target,
                product_definitional,
                target_id,
            });
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, p: PlacedDatumTargetFeature) -> Result<u64, WriteError> {
        let pds_step_id = buf
            .product_def_shape_ids
            .get(&p.target)
            .copied()
            .unwrap_or(0);
        let bool_attr = if p.product_definitional { "T" } else { "F" };
        Ok(buf.push_simple(
            "PLACED_DATUM_TARGET_FEATURE",
            vec![
                Attribute::String(p.name),
                Attribute::String(p.description),
                Attribute::EntityRef(pds_step_id),
                Attribute::Enum(bool_attr.into()),
                Attribute::String(p.target_id),
            ],
        ))
    }
}
