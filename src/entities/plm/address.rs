//! `ADDRESS` handler plm. STEP positional shape
//! `(internal_location, street_number, street, postal_box, town,
//! region, postal_code, country, facsimile_number, telephone_number,
//! electronic_mail_address, telex_number)` per `AP214e3` schema. The
//! concrete supertype variant; subtypes (e.g. `PERSONAL_ADDRESS`) have
//! their own handlers and land in the same `addresses` arena.

use crate::entities::SimpleEntityHandler;
use crate::entities::plm::{read_address_data, write_address_data};
use crate::ir::attr::check_count;
use crate::ir::error::ConvertError;
use crate::ir::plm::{Address, PlmPool};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct AddressHandler;

#[step_entity(name = "ADDRESS")]
impl SimpleEntityHandler for AddressHandler {
    type WriteInput = Address;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 12, entity_id, "ADDRESS")?;
        let data = read_address_data(attrs, 0, entity_id, "ADDRESS")?;
        let pool = ctx.plm.get_or_insert_with(PlmPool::default);
        let id = pool.addresses.push(Address::Itself(data));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, addr: Address) -> Result<u64, WriteError> {
        let data = match addr {
            Address::Itself(d) => d,
            Address::PersonalAddress(_) => {
                unreachable!("PersonalAddress writes through PersonalAddressHandler")
            }
        };
        let mut attrs: Vec<Attribute> = Vec::with_capacity(12);
        write_address_data(&mut attrs, data);
        Ok(buf.push_simple("ADDRESS", attrs))
    }
}
