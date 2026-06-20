//! `ADDRESS` handler — plm (2-layer path: generated bind/serialize +
//! hand-written lower/lift). The `PERSONAL_ADDRESS` subtype has its own
//! handler; both share the `Address` arena.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::SimpleEntityHandler;
use crate::ir::error::ConvertError;
use crate::ir::plm::Address;
use crate::parser::entity::Attribute;
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
        _: crate::early::EarlyGraph<'_>,
    ) -> Result<(), ConvertError> {
        let early = bind::bind_address(entity_id, attrs)?;
        lower::lower_address(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, addr: Address) -> Result<u64, WriteError> {
        let data = match addr {
            Address::Itself(d) => d,
            Address::PersonalAddress(_) => {
                unreachable!("PersonalAddress writes through PersonalAddressHandler")
            }
        };
        let early = lift::lift_address(data);
        Ok(serialize::serialize_address(buf, &early))
    }
}
