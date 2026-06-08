//! `TOLERANCE_ZONE` handler.
//!
//! `TOLERANCE_ZONE` is a `SHAPE_ASPECT` subtype: the 4-attr shape-aspect body
//! plus a `defining_tolerance` SET of `geometric_tolerance` references and a
//! `form` reference to a `TOLERANCE_ZONE_FORM`. The ir.toml blueprint folds it
//! into the `shape_aspect` arena; step-io keeps a dedicated `tolerance_zones`
//! arena like every other shape-aspect subtype.

use crate::entities::SimpleEntityHandler;
use crate::entities::pmi::resolve_geometric_tolerance_ref;
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref, read_entity_ref_list, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::pmi::GeometricToleranceRef;
use crate::ir::shape_rep::ToleranceZone;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct ToleranceZoneHandler;

#[step_entity(name = "TOLERANCE_ZONE")]
impl SimpleEntityHandler for ToleranceZoneHandler {
    type WriteInput = ToleranceZone;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 6, entity_id, "TOLERANCE_ZONE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let of_shape_ref = read_entity_ref(attrs, 2, entity_id, "of_shape")?;
        let product_definitional = read_bool(attrs, 3, entity_id, "product_definitional")?;
        let tolerance_refs = read_entity_ref_list(attrs, 4, entity_id, "defining_tolerance")?;
        let form_ref = read_entity_ref(attrs, 5, entity_id, "form")?;

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
        // form — a TOLERANCE_ZONE_FORM; an unresolved form drops the zone.
        let Some(&form) = ctx.tolerance_zone_form_id_map.get(&form_ref) else {
            return Ok(());
        };

        // defining_tolerance — geometric tolerances. An individual ref that
        // does not resolve is skipped (symmetric on re-read).
        let mut defining_tolerance = Vec::with_capacity(tolerance_refs.len());
        for r in tolerance_refs {
            if let Some(gtr) = resolve_geometric_tolerance_ref(ctx, r) {
                defining_tolerance.push(gtr);
            }
        }

        let id = ctx.tolerance_zones.push(ToleranceZone {
            name,
            description,
            target,
            product_definitional,
            defining_tolerance,
            form,
        });
        ctx.tolerance_zone_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, tz: ToleranceZone) -> Result<u64, WriteError> {
        // `target` → PRODUCT_DEFINITION_SHAPE step id; a miss is the
        // kernel-built IR defensive case, in practice unreachable.
        let pds_step_id = buf
            .product_def_shape_ids
            .get(&tz.target)
            .copied()
            .unwrap_or(0);
        let mut tolerance_refs = Vec::with_capacity(tz.defining_tolerance.len());
        for gtr in &tz.defining_tolerance {
            let step_id = match gtr {
                GeometricToleranceRef::Plain(id) => buf.geometric_tolerance_step_ids[id.0 as usize],
                GeometricToleranceRef::WithDatumReference(id) => {
                    buf.geometric_tolerance_with_datum_reference_step_ids[id.0 as usize]
                }
            };
            tolerance_refs.push(Attribute::EntityRef(step_id));
        }
        let form = buf.tolerance_zone_form_step_ids[tz.form.0 as usize];
        let bool_attr = if tz.product_definitional { "T" } else { "F" };
        Ok(buf.push_simple(
            "TOLERANCE_ZONE",
            vec![
                Attribute::String(tz.name),
                Attribute::String(tz.description),
                Attribute::EntityRef(pds_step_id),
                Attribute::Enum(bool_attr.into()),
                Attribute::List(tolerance_refs),
                Attribute::EntityRef(form),
            ],
        ))
    }
}
