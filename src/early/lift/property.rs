//! Property-domain `lift` fns (attribute leaf batch). See the
//! [module docs](super) for the lifting contract.

use crate::early::model::{
    EarlyDescriptionAttribute, EarlyDimensionalCharacteristicRepresentation, EarlyGeneralProperty,
    EarlyGeneralPropertyAssociation, EarlyIdAttribute, EarlyNameAttribute,
    EarlyShapeDefinitionRepresentation,
};
use crate::ir::property::GeneralProperty;

/// Lift one `GENERAL_PROPERTY` (faithful optional description — the legacy
/// writer emitted `None` as `$`).
pub(crate) fn lift_general_property(gp: GeneralProperty) -> EarlyGeneralProperty {
    EarlyGeneralProperty {
        id: gp.id,
        name: gp.name,
        description: gp.description,
    }
}

/// Lift one `NAME_ATTRIBUTE` (item pre-resolved).
pub(crate) fn lift_name_attribute(attribute_value: String, named_item: u64) -> EarlyNameAttribute {
    EarlyNameAttribute {
        attribute_value,
        named_item,
    }
}

/// Lift one `DESCRIPTION_ATTRIBUTE` (item pre-resolved).
pub(crate) fn lift_description_attribute(
    attribute_value: String,
    described_item: u64,
) -> EarlyDescriptionAttribute {
    EarlyDescriptionAttribute {
        attribute_value,
        described_item,
    }
}

/// Lift one `ID_ATTRIBUTE` (item pre-resolved).
pub(crate) fn lift_id_attribute(attribute_value: String, identified_item: u64) -> EarlyIdAttribute {
    EarlyIdAttribute {
        attribute_value,
        identified_item,
    }
}

/// Lift one `DIMENSIONAL_CHARACTERISTIC_REPRESENTATION` (refs pre-resolved).
pub(crate) fn lift_dimensional_characteristic_representation(
    dimension: u64,
    representation: u64,
) -> EarlyDimensionalCharacteristicRepresentation {
    EarlyDimensionalCharacteristicRepresentation {
        dimension,
        representation,
    }
}

/// Lift one `GENERAL_PROPERTY_ASSOCIATION` (faithful optional description —
/// the legacy writer emitted `None` as `$`).
pub(crate) fn lift_general_property_association(
    name: String,
    description: Option<String>,
    base_definition: u64,
    derived_definition: u64,
) -> EarlyGeneralPropertyAssociation {
    EarlyGeneralPropertyAssociation {
        name,
        description,
        base_definition,
        derived_definition,
    }
}

/// Lift one `SHAPE_DEFINITION_REPRESENTATION` (both refs pre-resolved).
pub(crate) fn lift_shape_definition_representation(
    definition: u64,
    used_representation: u64,
) -> EarlyShapeDefinitionRepresentation {
    EarlyShapeDefinitionRepresentation {
        definition,
        used_representation,
    }
}
