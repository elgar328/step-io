//! `QUALIFIED_REPRESENTATION_ITEM` handler — phase repr-item-arena-1.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::representation_item::{
    QualifiedRepresentationItem, QualifierRef, RepresentationItem,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct QualifiedRepresentationItemHandler;

#[step_entity(name = "QUALIFIED_REPRESENTATION_ITEM")]
impl SimpleEntityHandler for QualifiedRepresentationItemHandler {
    type WriteInput = QualifiedRepresentationItem;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "QUALIFIED_REPRESENTATION_ITEM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let q_refs = read_entity_ref_list(attrs, 1, entity_id, "qualifiers")?;
        let mut qualifiers = Vec::with_capacity(q_refs.len());
        for r in q_refs {
            if let Some(id) = ctx.id_cache.get::<crate::ir::id::TypeQualifierId>(r) {
                qualifiers.push(QualifierRef::TypeQualifier(id));
            } else if let Some(id) = ctx
                .id_cache
                .get::<crate::ir::id::ValueFormatTypeQualifierId>(r)
            {
                qualifiers.push(QualifierRef::ValueFormatTypeQualifier(id));
            }
            // else: precision_qualifier / uncertainty_qualifier (corpus 0,
            // not modelled) — silently drop the SELECT member.
        }
        let id = ctx
            .representation_items
            .push(RepresentationItem::QualifiedRepresentationItem(
                QualifiedRepresentationItem { name, qualifiers },
            ));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, qri: QualifiedRepresentationItem) -> Result<u64, WriteError> {
        let mut q_refs = Vec::with_capacity(qri.qualifiers.len());
        for q in qri.qualifiers {
            let step = match q {
                QualifierRef::TypeQualifier(id) => buf.step_id(id),
                QualifierRef::ValueFormatTypeQualifier(id) => buf.step_id(id),
            };
            q_refs.push(Attribute::EntityRef(step));
        }
        Ok(buf.push_simple(
            "QUALIFIED_REPRESENTATION_ITEM",
            vec![Attribute::String(qri.name), Attribute::List(q_refs)],
        ))
    }
}
