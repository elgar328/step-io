//! `SYMBOL_TARGET` + `DEFINED_SYMBOL` handlers — phase ds-st.
//!
//! Both are `geometric_representation_item` subtypes that step-io models
//! through the unified `GeometricRepresentationItem` enum arena. Pass
//! split: `SYMBOL_TARGET` (Pass6) is read first so that `DEFINED_SYMBOL`
//! (Pass8) can resolve its `target` ref through `symbol_target_id_map`.
//! `DEFINED_SYMBOL.definition` resolves through
//! `viz_pre_defined_symbol_id_map`; unresolved members drop the carrier
//! (symmetric on re-read).

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_real, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::visualization::{
    DefinedSymbol, DefinedSymbolDefinition, GeometricRepresentationItem, SymbolPlacement,
    SymbolTarget,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct SymbolTargetHandler;

#[step_entity(name = "SYMBOL_TARGET", pass = Pass6SymbolTarget)]
impl SimpleEntityHandler for SymbolTargetHandler {
    type WriteInput = SymbolTarget;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "SYMBOL_TARGET")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let placement_ref = read_entity_ref(attrs, 1, entity_id, "placement")?;
        let x_scale = read_real(attrs, 2, entity_id, "x_scale")?;
        let y_scale = read_real(attrs, 3, entity_id, "y_scale")?;
        // axis2_placement SELECT — step-io only models the 3D variant
        // (no 2D placement_map). 2D inputs drop the carrier.
        let Some(&placement_id) = ctx.placement_map.get(&placement_ref) else {
            return Ok(());
        };
        let id =
            ctx.geometric_representation_items
                .push(GeometricRepresentationItem::SymbolTarget(SymbolTarget {
                    name,
                    placement: SymbolPlacement::Placement3d(placement_id),
                    x_scale,
                    y_scale,
                }));
        ctx.symbol_target_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, t: SymbolTarget) -> Result<u64, WriteError> {
        let SymbolPlacement::Placement3d(placement_id) = t.placement;
        let placement_step = buf.emit_axis2_placement_3d(placement_id)?;
        Ok(buf.push_simple(
            "SYMBOL_TARGET",
            vec![
                Attribute::String(t.name),
                Attribute::EntityRef(placement_step),
                Attribute::Real(t.x_scale),
                Attribute::Real(t.y_scale),
            ],
        ))
    }
}

pub(crate) struct DefinedSymbolHandler;

#[step_entity(name = "DEFINED_SYMBOL", pass = Pass8DefinedSymbol)]
impl SimpleEntityHandler for DefinedSymbolHandler {
    type WriteInput = DefinedSymbol;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "DEFINED_SYMBOL")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let def_ref = read_entity_ref(attrs, 1, entity_id, "definition")?;
        let target_ref = read_entity_ref(attrs, 2, entity_id, "target")?;
        let Some(&pds_id) = ctx.viz_pre_defined_symbol_id_map.get(&def_ref) else {
            return Ok(());
        };
        let Some(&target) = ctx.symbol_target_id_map.get(&target_ref) else {
            return Ok(());
        };
        ctx.geometric_representation_items
            .push(GeometricRepresentationItem::DefinedSymbol(DefinedSymbol {
                name,
                definition: DefinedSymbolDefinition::PreDefinedSymbol(pds_id),
                target,
            }));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, d: DefinedSymbol) -> Result<u64, WriteError> {
        let definition_step = match d.definition {
            DefinedSymbolDefinition::PreDefinedSymbol(id) => {
                buf.pre_defined_symbol_step_ids[id.0 as usize]
            }
        };
        let target_step = buf.geometric_representation_item_step_ids[d.target.0 as usize];
        Ok(buf.push_simple(
            "DEFINED_SYMBOL",
            vec![
                Attribute::String(d.name),
                Attribute::EntityRef(definition_step),
                Attribute::EntityRef(target_step),
            ],
        ))
    }
}
