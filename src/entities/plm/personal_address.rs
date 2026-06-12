//! `PERSONAL_ADDRESS` handler — plm (2-layer path: generated
//! bind/serialize + hand-written lower/lift). Subtype of `ADDRESS`
//! (12 inherited optional fields + `people` + `description`).

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::Address;
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

pub(crate) struct PersonalAddressHandler;

#[step_entity(name = "PERSONAL_ADDRESS")]
impl SimpleEntityHandler for PersonalAddressHandler {
    type WriteInput = Address;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_personal_address(entity_id, attrs)?;
        lower::lower_personal_address(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, addr: Address) -> Result<u64, WriteError> {
        let pa = match addr {
            Address::PersonalAddress(p) => p,
            Address::Itself(_) => {
                unreachable!("ADDRESS::Itself writes through AddressHandler")
            }
        };
        let people: Vec<u64> = pa.people.iter().map(|pid| buf.step_id(pid)).collect();
        let early = lift::lift_personal_address(pa, people);
        Ok(serialize::serialize_personal_address(buf, &early))
    }
}
