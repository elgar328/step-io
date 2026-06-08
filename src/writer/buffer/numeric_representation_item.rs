//! `INTEGER_REPRESENTATION_ITEM` / `REAL_REPRESENTATION_ITEM` emission
//! (phase numeric-representation-item). Standalone orphan emit in arena
//! order — the value-items carry no references, so the call site in
//! `emit_all` is order-independent.

use super::WriteBuffer;
use crate::entities::SimpleEntityHandler;
use crate::entities::shape_rep::numeric_representation_item::{
    IntegerRepresentationItemHandler, RealRepresentationItemHandler,
};
use crate::ir::shape_rep::NumericRepresentationItem;
use crate::writer::WriteError;

impl WriteBuffer<'_> {
    pub(in crate::writer::buffer) fn emit_numeric_representation_items(
        &mut self,
    ) -> Result<(), WriteError> {
        // Snapshot to release the &model borrow before per-item emission.
        let items: Vec<_> = self
            .model
            .numeric_representation_items
            .iter()
            .cloned()
            .collect();
        for item in items {
            match item {
                NumericRepresentationItem::Integer(i) => {
                    IntegerRepresentationItemHandler::write(self, i)?;
                }
                NumericRepresentationItem::Real(r) => {
                    RealRepresentationItemHandler::write(self, r)?;
                }
            }
        }
        Ok(())
    }
}
