//! `PERSONAL_ADDRESS` handler — Pass 9-24 plm. STEP positional shape
//! `(12 inherited ADDRESS fields, people, description)` per `AP214e3`
//! schema. `people` is `SET[1:?] OF person`; `description` is required
//! text. Lands in the same `addresses` arena as `ADDRESS` via the
//! `Address::PersonalAddress` variant.

use crate::entities::SimpleEntityHandler;
use crate::entities::plm::{read_address_data, write_address_data};
use crate::ir::PersonId;
use crate::ir::attr::{check_count, read_entity_ref_list, read_string_or_unset};
use crate::ir::error::ConvertError;
use crate::ir::plm::{Address, PersonalAddress, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PersonalAddressHandler;

#[step_entity(name = "PERSONAL_ADDRESS", pass = Pass9PlmAddress)]
impl SimpleEntityHandler for PersonalAddressHandler {
    type WriteInput = Address;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 14, entity_id, "PERSONAL_ADDRESS")?;
        let inherited = read_address_data(attrs, 0, entity_id, "PERSONAL_ADDRESS")?;
        let people_refs = read_entity_ref_list(attrs, 12, entity_id, "people")?;
        let description = read_string_or_unset(attrs, 13, entity_id, "description")?.to_owned();
        let people: Vec<PersonId> = people_refs
            .iter()
            .filter_map(|r| ctx.plm_person_id_map.get(r).copied())
            .collect();
        // Schema mandates SET[1:?]; an empty resolved set means no Person
        // ref survived (forward-ref drop or unsupported variant). Drop
        // the entry rather than emit a violating empty.
        if people.is_empty() {
            return Ok(());
        }
        let pa = PersonalAddress {
            inherited,
            people,
            description,
        };
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.addresses.push(Address::PersonalAddress(pa));
        ctx.plm_address_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, addr: Address) -> Result<u64, WriteError> {
        let pa = match addr {
            Address::PersonalAddress(p) => p,
            Address::Itself(_) => {
                unreachable!("ADDRESS::Itself writes through AddressHandler")
            }
        };
        let mut attrs: Vec<Attribute> = Vec::with_capacity(14);
        write_address_data(&mut attrs, pa.inherited);
        let people_refs: Vec<Attribute> = pa
            .people
            .iter()
            .map(|pid| Attribute::EntityRef(buf.plm_person_step_ids[pid.0 as usize]))
            .collect();
        attrs.push(Attribute::List(people_refs));
        attrs.push(Attribute::String(pa.description));
        Ok(buf.push_simple("PERSONAL_ADDRESS", attrs))
    }
}
