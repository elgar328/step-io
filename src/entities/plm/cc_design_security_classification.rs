//! `CC_DESIGN_SECURITY_CLASSIFICATION` handler — Pass 9-14 plm. Variant
//! of the `security_classification_assignment` arena enum. STEP entity
//! name lacks the `_ASSIGNMENT` suffix carried by the Applied sibling.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref, read_entity_ref_list};
use crate::ir::error::ConvertError;
use crate::ir::plm::{
    CcDesignSecurityClassification, PlmPool, SecurityClassificationAssignment,
    SecurityClassificationItem,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;

use super::resolve_date_time_item;
use step_io_macros::step_entity;

pub(crate) struct CcDesignSecurityClassificationHandler;

#[step_entity(name = "CC_DESIGN_SECURITY_CLASSIFICATION")]
impl SimpleEntityHandler for CcDesignSecurityClassificationHandler {
    type WriteInput = CcDesignSecurityClassification;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "CC_DESIGN_SECURITY_CLASSIFICATION")?;
        let sc_ref = read_entity_ref(attrs, 0, entity_id, "assigned_security_classification")?;
        let item_refs = read_entity_ref_list(attrs, 1, entity_id, "items")?;
        let Some(&assigned_security_classification) =
            ctx.plm_security_classification_id_map.get(&sc_ref)
        else {
            return Ok(());
        };
        let mut items = Vec::with_capacity(item_refs.len());
        for r in item_refs {
            if let Some(pid) = resolve_date_time_item(ctx, r) {
                items.push(SecurityClassificationItem::Product(pid));
            }
        }
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.security_classification_assignments.push(
            SecurityClassificationAssignment::CcDesign(CcDesignSecurityClassification {
                assigned_security_classification,
                items,
            }),
        );
        ctx.plm_sca_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, c: CcDesignSecurityClassification) -> Result<u64, WriteError> {
        let sc_step =
            buf.plm_security_classification_step_ids[c.assigned_security_classification.0 as usize];
        let mut item_refs = Vec::with_capacity(c.items.len());
        for item in c.items {
            let step_id = match item {
                SecurityClassificationItem::Product(pid) => buf.product_def_ids[&pid],
            };
            item_refs.push(Attribute::EntityRef(step_id));
        }
        Ok(buf.push_simple(
            "CC_DESIGN_SECURITY_CLASSIFICATION",
            vec![Attribute::EntityRef(sc_step), Attribute::List(item_refs)],
        ))
    }
}
