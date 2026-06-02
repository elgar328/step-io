//! `PERSON` handler — Pass 9-5 plm leaf.

use crate::entities::SimpleEntityHandler;
use crate::ir::attr::{
    check_count, read_optional_string, read_optional_string_list, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::plm::{Person, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PersonHandler;

#[step_entity(name = "PERSON")]
impl SimpleEntityHandler for PersonHandler {
    type WriteInput = Person;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 6, entity_id, "PERSON")?;
        let id = read_string_or_unset(attrs, 0, entity_id, "id")?.to_owned();
        let last_name = read_optional_string(attrs, 1, entity_id, "last_name")?;
        let first_name = read_optional_string(attrs, 2, entity_id, "first_name")?;
        let middle_names = read_optional_string_list(attrs, 3, entity_id, "middle_names")?;
        let prefix_titles = read_optional_string_list(attrs, 4, entity_id, "prefix_titles")?;
        let suffix_titles = read_optional_string_list(attrs, 5, entity_id, "suffix_titles")?;
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let p_id = pool.persons.push(Person {
            id,
            last_name,
            first_name,
            middle_names,
            prefix_titles,
            suffix_titles,
        });
        ctx.plm_person_id_map.insert(entity_id, p_id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, p: Person) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "PERSON",
            vec![
                Attribute::String(p.id),
                opt_str(p.last_name),
                opt_str(p.first_name),
                opt_list(p.middle_names),
                opt_list(p.prefix_titles),
                opt_list(p.suffix_titles),
            ],
        ))
    }
}

fn opt_str(opt: Option<String>) -> Attribute {
    match opt {
        Some(s) => Attribute::String(s),
        None => Attribute::Unset,
    }
}

fn opt_list(opt: Option<Vec<String>>) -> Attribute {
    match opt {
        Some(v) => Attribute::List(v.into_iter().map(Attribute::String).collect()),
        None => Attribute::Unset,
    }
}
