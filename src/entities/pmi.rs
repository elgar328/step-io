//! `pmi` pool entity handlers — Pass 8.
//!
//! Three dependency-free `single_struct` primitives — `TOLERANCE_ZONE_FORM`,
//! `TYPE_QUALIFIER`, `VALUE_FORMAT_TYPE_QUALIFIER` — each a 1-attr string
//! entity pushed into [`PmiPool`]. They have no entity references; the
//! GD&T entities that consume them arrive in later phases.

use crate::entities::shape_rep::shape_aspect::ShapeAspectWriteInput;
use crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::PmiPool;
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref, read_entity_ref_list, read_enum, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::pmi::{
    AngleSelection, AngularLocationData, AnnotationOccurrence, AnnotationPlane, Datum,
    DatumFeature, DimensionalCharacteristic, DimensionalLocation, DimensionalLocationData,
    DimensionalSize, DimensionalSizeKind, DraughtingPreDefinedTextFont, GeneralDatumBase,
    GeneralDatumReference, GeneralDatumReferenceData, GeometricTolerance, GeometricToleranceData,
    GeometricToleranceWithDatumReference, GeometricToleranceWithDatumReferenceData, LimitsAndFits,
    PlusMinusTolerance, TessellatedAnnotationOccurrence, ToleranceMagnitude,
    ToleranceMethodDefinition, ToleranceValue, ToleranceZoneForm, TypeQualifier,
    ValueFormatTypeQualifier,
};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::{step_entity, step_entity_complex};

pub(crate) struct ToleranceZoneFormHandler;

#[step_entity(name = "TOLERANCE_ZONE_FORM", pass = Pass8ShapeAspect)]
impl SimpleEntityHandler for ToleranceZoneFormHandler {
    type WriteInput = ToleranceZoneForm;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "TOLERANCE_ZONE_FORM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .tolerance_zone_forms
            .push(ToleranceZoneForm { name });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, tzf: ToleranceZoneForm) -> Result<u64, WriteError> {
        Ok(buf.push_simple("TOLERANCE_ZONE_FORM", vec![Attribute::String(tzf.name)]))
    }
}

pub(crate) struct TypeQualifierHandler;

#[step_entity(name = "TYPE_QUALIFIER", pass = Pass8ShapeAspect)]
impl SimpleEntityHandler for TypeQualifierHandler {
    type WriteInput = TypeQualifier;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "TYPE_QUALIFIER")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .type_qualifiers
            .push(TypeQualifier { name });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, tq: TypeQualifier) -> Result<u64, WriteError> {
        Ok(buf.push_simple("TYPE_QUALIFIER", vec![Attribute::String(tq.name)]))
    }
}

pub(crate) struct ValueFormatTypeQualifierHandler;

#[step_entity(name = "VALUE_FORMAT_TYPE_QUALIFIER", pass = Pass8ShapeAspect)]
impl SimpleEntityHandler for ValueFormatTypeQualifierHandler {
    type WriteInput = ValueFormatTypeQualifier;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "VALUE_FORMAT_TYPE_QUALIFIER")?;
        let format_type = read_string_or_unset(attrs, 0, entity_id, "format_type")?.to_owned();
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .value_format_type_qualifiers
            .push(ValueFormatTypeQualifier { format_type });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, vftq: ValueFormatTypeQualifier) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "VALUE_FORMAT_TYPE_QUALIFIER",
            vec![Attribute::String(vftq.format_type)],
        ))
    }
}

pub(crate) struct DraughtingPreDefinedTextFontHandler;

#[step_entity(name = "DRAUGHTING_PRE_DEFINED_TEXT_FONT", pass = Pass8ShapeAspect)]
impl SimpleEntityHandler for DraughtingPreDefinedTextFontHandler {
    type WriteInput = DraughtingPreDefinedTextFont;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 1, entity_id, "DRAUGHTING_PRE_DEFINED_TEXT_FONT")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .draughting_pre_defined_text_fonts
            .push(DraughtingPreDefinedTextFont { name });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, font: DraughtingPreDefinedTextFont) -> Result<u64, WriteError> {
        Ok(buf.push_simple(
            "DRAUGHTING_PRE_DEFINED_TEXT_FONT",
            vec![Attribute::String(font.name)],
        ))
    }
}

pub(crate) struct AnnotationPlaneHandler;

/// `ANNOTATION_PLANE(name, styles, item, elements)` — a `styled_item`
/// subtype. `styles` resolves through `viz_psa_id_map` (like `STYLED_ITEM`)
/// and `item` through the shared `representation_item` resolver; the 4th
/// attribute `elements` (a `DRAUGHTING_CALLOUT` list) is not modelled and
/// is ignored on read. An `ANNOTATION_PLANE` whose `item` does not resolve
/// is silently dropped, symmetric on re-read.
#[step_entity(name = "ANNOTATION_PLANE", pass = Pass7AnnotationPlane)]
impl SimpleEntityHandler for AnnotationPlaneHandler {
    type WriteInput = AnnotationPlane;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "ANNOTATION_PLANE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;
        // attr 3 (`elements`) — DRAUGHTING_CALLOUT list, not modelled.

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(&psa_id) = ctx.viz_psa_id_map.get(&r) {
                styles.push(psa_id);
            }
        }
        let Some(item) = resolve_representation_item_ref(ctx, item_ref) else {
            return Ok(());
        };

        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .annotation_occurrences
            .push(AnnotationOccurrence::AnnotationPlane(AnnotationPlane {
                name,
                styles,
                item,
            }));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ap: AnnotationPlane) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(ap.item)?;
        let mut style_refs = Vec::with_capacity(ap.styles.len());
        for psa_id in ap.styles {
            style_refs.push(Attribute::EntityRef(buf.psa_step_ids[psa_id.0 as usize]));
        }
        Ok(buf.push_simple(
            "ANNOTATION_PLANE",
            vec![
                Attribute::String(ap.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(item_id),
                Attribute::Unset,
            ],
        ))
    }
}

pub(crate) struct TessellatedAnnotationOccurrenceHandler;

/// `TESSELLATED_ANNOTATION_OCCURRENCE(name, styles, item)` — an
/// `annotation_occurrence` subtype. `styles` resolves through
/// `viz_psa_id_map` (like `ANNOTATION_PLANE`); `item` is a
/// `TESSELLATED_GEOMETRIC_SET` resolved through `tessellated_item_id_map`.
/// An occurrence whose `item` does not resolve is silently dropped.
#[step_entity(name = "TESSELLATED_ANNOTATION_OCCURRENCE", pass = Pass7AnnotationPlane)]
impl SimpleEntityHandler for TessellatedAnnotationOccurrenceHandler {
    type WriteInput = TessellatedAnnotationOccurrence;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "TESSELLATED_ANNOTATION_OCCURRENCE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(&psa_id) = ctx.viz_psa_id_map.get(&r) {
                styles.push(psa_id);
            }
        }
        let Some(&item) = ctx.tessellated_item_id_map.get(&item_ref) else {
            return Ok(()); // item unresolved — drop the occurrence
        };

        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .annotation_occurrences
            .push(AnnotationOccurrence::TessellatedAnnotationOccurrence(
                TessellatedAnnotationOccurrence { name, styles, item },
            ));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        tao: TessellatedAnnotationOccurrence,
    ) -> Result<u64, WriteError> {
        let item = buf.tessellated_item_step_ids[tao.item.0 as usize];
        let mut style_refs = Vec::with_capacity(tao.styles.len());
        for psa_id in tao.styles {
            style_refs.push(Attribute::EntityRef(buf.psa_step_ids[psa_id.0 as usize]));
        }
        Ok(buf.push_simple(
            "TESSELLATED_ANNOTATION_OCCURRENCE",
            vec![
                Attribute::String(tao.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(item),
            ],
        ))
    }
}

/// Resolved write input for [`DatumHandler`] — the caller resolves
/// `of_shape` (a `ProductId`) to a `PRODUCT_DEFINITION_SHAPE` step id.
pub(crate) struct DatumWriteInput {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) pds_step_id: u64,
    pub(crate) product_definitional: bool,
    pub(crate) identification: String,
}

pub(crate) struct DatumHandler;

/// `DATUM(name, description, of_shape, product_definitional, identification)`
/// — a `shape_aspect` subtype. `of_shape` resolves to a `ProductId` through
/// the same `PRODUCT_DEFINITION_SHAPE` → `PRODUCT_DEFINITION` chain as
/// `SHAPE_ASPECT`; an unresolved `of_shape` drops the datum, symmetric on
/// re-read. `product_definitional` is the inherited `shape_aspect` BOOLEAN
/// (read as `bool`, like every other shape-aspect-family entity).
#[step_entity(name = "DATUM", pass = Pass8ShapeAspect)]
impl SimpleEntityHandler for DatumHandler {
    type WriteInput = DatumWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 5, entity_id, "DATUM")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let of_shape_ref = read_entity_ref(attrs, 2, entity_id, "of_shape")?;
        let product_definitional = read_bool(attrs, 3, entity_id, "product_definitional")?;
        let identification =
            read_string_or_unset(attrs, 4, entity_id, "identification")?.to_owned();

        // of_shape → PRODUCT_DEFINITION_SHAPE → PRODUCT_DEFINITION → ProductId.
        let Some(&pdef_step_id) = ctx.pdef_shape_to_pdef.get(&of_shape_ref) else {
            return Ok(());
        };
        let Some(&product_step_id) = ctx.pdef_to_product.get(&pdef_step_id) else {
            return Ok(());
        };
        let Some(&target) = ctx.product_arena_map.get(&product_step_id) else {
            return Ok(());
        };

        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .datums
            .push(Datum {
                name,
                description,
                target,
                product_definitional,
                identification,
            });
        ctx.datum_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, input: DatumWriteInput) -> Result<u64, WriteError> {
        let bool_attr = if input.product_definitional { "T" } else { "F" };
        Ok(buf.push_simple(
            "DATUM",
            vec![
                Attribute::String(input.name),
                Attribute::String(input.description),
                Attribute::EntityRef(input.pds_step_id),
                Attribute::Enum(bool_attr.into()),
                Attribute::String(input.identification),
            ],
        ))
    }
}

pub(crate) struct DatumFeatureHandler;

/// `DATUM_FEATURE(name, description, of_shape, product_definitional)` — a
/// `shape_aspect` subtype naming the physical feature realising a datum.
/// Same 4-attr `shape_aspect` body and `of_shape → ProductId` resolution as
/// `SHAPE_ASPECT`; an unresolved `of_shape` drops the datum feature,
/// symmetric on re-read. Registered into `datum_feature_id_map` so a
/// `shape_aspect` ref (e.g. `geometric_tolerance.toleranced_shape_aspect`)
/// resolves onto it through `resolve_shape_aspect_ref`.
#[step_entity(name = "DATUM_FEATURE", pass = Pass8ShapeAspect)]
impl SimpleEntityHandler for DatumFeatureHandler {
    type WriteInput = ShapeAspectWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "DATUM_FEATURE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let of_shape_ref = read_entity_ref(attrs, 2, entity_id, "of_shape")?;
        let product_definitional = read_bool(attrs, 3, entity_id, "product_definitional")?;

        // of_shape → PRODUCT_DEFINITION_SHAPE → PRODUCT_DEFINITION → ProductId.
        let Some(&pdef_step_id) = ctx.pdef_shape_to_pdef.get(&of_shape_ref) else {
            return Ok(());
        };
        let Some(&product_step_id) = ctx.pdef_to_product.get(&pdef_step_id) else {
            return Ok(());
        };
        let Some(&target) = ctx.product_arena_map.get(&product_step_id) else {
            return Ok(());
        };

        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .datum_features
            .push(DatumFeature {
                name,
                description,
                target,
                product_definitional,
            });
        ctx.datum_feature_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, input: ShapeAspectWriteInput) -> Result<u64, WriteError> {
        let bool_attr = if input.product_definitional { "T" } else { "F" };
        Ok(buf.push_simple(
            "DATUM_FEATURE",
            vec![
                Attribute::String(input.name),
                Attribute::String(input.description),
                Attribute::EntityRef(input.pds_step_id),
                Attribute::Enum(bool_attr.into()),
            ],
        ))
    }
}

/// Map a STEP `angle_relator` enum value to [`AngleSelection`].
fn read_angle_selection(
    attrs: &[Attribute],
    index: usize,
    entity_id: u64,
    field_name: &'static str,
) -> Result<AngleSelection, ConvertError> {
    match read_enum(attrs, index, entity_id, field_name)? {
        "EQUAL" => Ok(AngleSelection::Equal),
        "LARGE" => Ok(AngleSelection::Large),
        "SMALL" => Ok(AngleSelection::Small),
        other => Err(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!("{field_name}: unknown angle_relator '.{other}.'"),
        }),
    }
}

/// [`AngleSelection`] → a STEP enum `Attribute`.
fn angle_selection_attr(sel: AngleSelection) -> Attribute {
    Attribute::Enum(
        match sel {
            AngleSelection::Equal => "EQUAL",
            AngleSelection::Large => "LARGE",
            AngleSelection::Small => "SMALL",
        }
        .into(),
    )
}

/// Emit a `DimensionalSize` under the STEP entity name its `kind` selects.
fn write_dimensional_size(buf: &mut WriteBuffer, ds: DimensionalSize) -> u64 {
    let applies_to = buf.emit_shape_aspect_ref(ds.applies_to);
    let mut fields = vec![Attribute::EntityRef(applies_to), Attribute::String(ds.name)];
    let name = match ds.kind {
        DimensionalSizeKind::Plain => "DIMENSIONAL_SIZE",
        DimensionalSizeKind::Angular(sel) => {
            fields.push(angle_selection_attr(sel));
            "ANGULAR_SIZE"
        }
    };
    buf.push_simple(name, fields)
}

pub(crate) struct DimensionalSizeHandler;

#[step_entity(name = "DIMENSIONAL_SIZE", pass = Pass8Dimensional)]
impl SimpleEntityHandler for DimensionalSizeHandler {
    type WriteInput = DimensionalSize;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "DIMENSIONAL_SIZE")?;
        let applies_to_ref = read_entity_ref(attrs, 0, entity_id, "applies_to")?;
        let name = read_string_or_unset(attrs, 1, entity_id, "name")?.to_owned();

        let Some(applies_to) = resolve_shape_aspect_ref(ctx, applies_to_ref) else {
            return Ok(()); // applies_to unresolved — drop the dimension
        };

        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .dimensional_sizes
            .push(DimensionalSize {
                applies_to,
                name,
                kind: DimensionalSizeKind::Plain,
            });
        ctx.dimensional_size_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ds: DimensionalSize) -> Result<u64, WriteError> {
        Ok(write_dimensional_size(buf, ds))
    }
}

pub(crate) struct AngularSizeHandler;

#[step_entity(name = "ANGULAR_SIZE", pass = Pass8Dimensional)]
impl SimpleEntityHandler for AngularSizeHandler {
    type WriteInput = DimensionalSize;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "ANGULAR_SIZE")?;
        let applies_to_ref = read_entity_ref(attrs, 0, entity_id, "applies_to")?;
        let name = read_string_or_unset(attrs, 1, entity_id, "name")?.to_owned();
        let angle_selection = read_angle_selection(attrs, 2, entity_id, "angle_selection")?;

        let Some(applies_to) = resolve_shape_aspect_ref(ctx, applies_to_ref) else {
            return Ok(());
        };

        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .dimensional_sizes
            .push(DimensionalSize {
                applies_to,
                name,
                kind: DimensionalSizeKind::Angular(angle_selection),
            });
        ctx.dimensional_size_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ds: DimensionalSize) -> Result<u64, WriteError> {
        Ok(write_dimensional_size(buf, ds))
    }
}

/// Read the shared `DIMENSIONAL_LOCATION` 4-attr body. `Ok(None)` when
/// either endpoint does not resolve — the location is dropped, symmetric
/// on re-read.
fn read_dimensional_location_data(
    ctx: &ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    entity_name: &'static str,
) -> Result<Option<DimensionalLocationData>, ConvertError> {
    check_count(attrs, 4, entity_id, entity_name)?;
    let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
    let relating_ref = read_entity_ref(attrs, 2, entity_id, "relating_shape_aspect")?;
    let related_ref = read_entity_ref(attrs, 3, entity_id, "related_shape_aspect")?;

    let Some(relating_shape_aspect) = resolve_shape_aspect_ref(ctx, relating_ref) else {
        return Ok(None);
    };
    let Some(related_shape_aspect) = resolve_shape_aspect_ref(ctx, related_ref) else {
        return Ok(None);
    };
    Ok(Some(DimensionalLocationData {
        name,
        description,
        relating_shape_aspect,
        related_shape_aspect,
    }))
}

/// Emit the shared 4-attr `DIMENSIONAL_LOCATION`-shaped body under `name`.
fn write_dimensional_location_4attr(
    buf: &mut WriteBuffer,
    name: &str,
    data: DimensionalLocationData,
) -> u64 {
    let relating = buf.emit_shape_aspect_ref(data.relating_shape_aspect);
    let related = buf.emit_shape_aspect_ref(data.related_shape_aspect);
    buf.push_simple(
        name,
        vec![
            Attribute::String(data.name),
            Attribute::String(data.description),
            Attribute::EntityRef(relating),
            Attribute::EntityRef(related),
        ],
    )
}

/// Emit a `DimensionalLocation` under the STEP entity name its variant
/// selects, returning the STEP id. Shared by all three family handlers.
fn write_dimensional_location(buf: &mut WriteBuffer, dl: DimensionalLocation) -> u64 {
    match dl {
        DimensionalLocation::Plain(d) => {
            write_dimensional_location_4attr(buf, "DIMENSIONAL_LOCATION", d)
        }
        DimensionalLocation::Directed(d) => {
            write_dimensional_location_4attr(buf, "DIRECTED_DIMENSIONAL_LOCATION", d)
        }
        DimensionalLocation::Angular(d) => {
            let relating = buf.emit_shape_aspect_ref(d.relating_shape_aspect);
            let related = buf.emit_shape_aspect_ref(d.related_shape_aspect);
            buf.push_simple(
                "ANGULAR_LOCATION",
                vec![
                    Attribute::String(d.name),
                    Attribute::String(d.description),
                    Attribute::EntityRef(relating),
                    Attribute::EntityRef(related),
                    angle_selection_attr(d.angle_selection),
                ],
            )
        }
    }
}

pub(crate) struct DimensionalLocationHandler;

#[step_entity(name = "DIMENSIONAL_LOCATION", pass = Pass8Dimensional)]
impl SimpleEntityHandler for DimensionalLocationHandler {
    type WriteInput = DimensionalLocation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) =
            read_dimensional_location_data(ctx, entity_id, attrs, "DIMENSIONAL_LOCATION")?
        else {
            return Ok(());
        };
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .dimensional_locations
            .push(DimensionalLocation::Plain(data));
        ctx.dimensional_location_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dl: DimensionalLocation) -> Result<u64, WriteError> {
        Ok(write_dimensional_location(buf, dl))
    }
}

pub(crate) struct DirectedDimensionalLocationHandler;

#[step_entity(name = "DIRECTED_DIMENSIONAL_LOCATION", pass = Pass8Dimensional)]
impl SimpleEntityHandler for DirectedDimensionalLocationHandler {
    type WriteInput = DimensionalLocation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) =
            read_dimensional_location_data(ctx, entity_id, attrs, "DIRECTED_DIMENSIONAL_LOCATION")?
        else {
            return Ok(());
        };
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .dimensional_locations
            .push(DimensionalLocation::Directed(data));
        ctx.dimensional_location_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dl: DimensionalLocation) -> Result<u64, WriteError> {
        Ok(write_dimensional_location(buf, dl))
    }
}

pub(crate) struct AngularLocationHandler;

#[step_entity(name = "ANGULAR_LOCATION", pass = Pass8Dimensional)]
impl SimpleEntityHandler for AngularLocationHandler {
    type WriteInput = DimensionalLocation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 5, entity_id, "ANGULAR_LOCATION")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let relating_ref = read_entity_ref(attrs, 2, entity_id, "relating_shape_aspect")?;
        let related_ref = read_entity_ref(attrs, 3, entity_id, "related_shape_aspect")?;
        let angle_selection = read_angle_selection(attrs, 4, entity_id, "angle_selection")?;

        let Some(relating_shape_aspect) = resolve_shape_aspect_ref(ctx, relating_ref) else {
            return Ok(());
        };
        let Some(related_shape_aspect) = resolve_shape_aspect_ref(ctx, related_ref) else {
            return Ok(());
        };

        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .dimensional_locations
            .push(DimensionalLocation::Angular(AngularLocationData {
                name,
                description,
                relating_shape_aspect,
                related_shape_aspect,
                angle_selection,
            }));
        ctx.dimensional_location_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dl: DimensionalLocation) -> Result<u64, WriteError> {
        Ok(write_dimensional_location(buf, dl))
    }
}

/// Resolve a `geometric_tolerance.magnitude` ref (`ref_measure_with_unit`).
/// A plain `*_MEASURE_WITH_UNIT` resolves through `mwu_id_map` (units pool);
/// a `MEASURE_REPRESENTATION_ITEM` (simple or complex) through
/// `measure_item_map`. `None` when the ref resolves to neither.
fn resolve_tolerance_magnitude(ctx: &ReaderContext, item_ref: u64) -> Option<ToleranceMagnitude> {
    if let Some(&id) = ctx.mwu_id_map.get(&item_ref) {
        return Some(ToleranceMagnitude::MeasureWithUnit(id));
    }
    ctx.measure_item_map
        .get(&item_ref)
        .cloned()
        .map(ToleranceMagnitude::Measure)
}

/// Read the shared `geometric_tolerance` 4-attr form-tolerance body.
/// `Ok(None)` when `magnitude` or `toleranced_shape_aspect` does not
/// resolve — the tolerance is dropped, symmetric on re-read.
fn read_geometric_tolerance_data(
    ctx: &ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    entity_name: &'static str,
) -> Result<Option<GeometricToleranceData>, ConvertError> {
    check_count(attrs, 4, entity_id, entity_name)?;
    let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
    let magnitude_ref = read_entity_ref(attrs, 2, entity_id, "magnitude")?;
    let shape_aspect_ref = read_entity_ref(attrs, 3, entity_id, "toleranced_shape_aspect")?;

    let Some(magnitude) = resolve_tolerance_magnitude(ctx, magnitude_ref) else {
        return Ok(None);
    };
    let Some(toleranced_shape_aspect) = resolve_shape_aspect_ref(ctx, shape_aspect_ref) else {
        return Ok(None);
    };
    Ok(Some(GeometricToleranceData {
        name,
        description,
        magnitude,
        toleranced_shape_aspect,
    }))
}

/// Emit a `GeometricTolerance` under the STEP entity name its variant
/// selects, returning the STEP id. Shared by all four form-tolerance
/// handlers and by `emit_geometric_tolerances`.
pub(crate) fn write_geometric_tolerance(buf: &mut WriteBuffer, gt: GeometricTolerance) -> u64 {
    let (entity_name, data) = match gt {
        GeometricTolerance::Flatness(d) => ("FLATNESS_TOLERANCE", d),
        GeometricTolerance::Straightness(d) => ("STRAIGHTNESS_TOLERANCE", d),
        GeometricTolerance::Roundness(d) => ("ROUNDNESS_TOLERANCE", d),
        GeometricTolerance::Cylindricity(d) => ("CYLINDRICITY_TOLERANCE", d),
    };
    // A `MeasureWithUnit` magnitude is already emitted by the units pass —
    // reference its cached step id. A `Measure` magnitude has no arena entry;
    // emit the (simple) MRI inline here.
    let magnitude = match data.magnitude {
        ToleranceMagnitude::MeasureWithUnit(id) => buf.mwu_step_ids[id.0 as usize],
        ToleranceMagnitude::Measure(m) => buf.emit_property_measure(&m, None),
    };
    let shape_aspect = buf.emit_shape_aspect_ref(data.toleranced_shape_aspect);
    buf.push_simple(
        entity_name,
        vec![
            Attribute::String(data.name),
            Attribute::String(data.description),
            Attribute::EntityRef(magnitude),
            Attribute::EntityRef(shape_aspect),
        ],
    )
}

pub(crate) struct FlatnessToleranceHandler;

#[step_entity(name = "FLATNESS_TOLERANCE", pass = Pass8GeometricTolerance)]
impl SimpleEntityHandler for FlatnessToleranceHandler {
    type WriteInput = GeometricTolerance;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) =
            read_geometric_tolerance_data(ctx, entity_id, attrs, "FLATNESS_TOLERANCE")?
        else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerances
            .push(GeometricTolerance::Flatness(data));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gt: GeometricTolerance) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance(buf, gt))
    }
}

pub(crate) struct StraightnessToleranceHandler;

#[step_entity(name = "STRAIGHTNESS_TOLERANCE", pass = Pass8GeometricTolerance)]
impl SimpleEntityHandler for StraightnessToleranceHandler {
    type WriteInput = GeometricTolerance;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) =
            read_geometric_tolerance_data(ctx, entity_id, attrs, "STRAIGHTNESS_TOLERANCE")?
        else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerances
            .push(GeometricTolerance::Straightness(data));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gt: GeometricTolerance) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance(buf, gt))
    }
}

pub(crate) struct RoundnessToleranceHandler;

#[step_entity(name = "ROUNDNESS_TOLERANCE", pass = Pass8GeometricTolerance)]
impl SimpleEntityHandler for RoundnessToleranceHandler {
    type WriteInput = GeometricTolerance;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) =
            read_geometric_tolerance_data(ctx, entity_id, attrs, "ROUNDNESS_TOLERANCE")?
        else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerances
            .push(GeometricTolerance::Roundness(data));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gt: GeometricTolerance) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance(buf, gt))
    }
}

pub(crate) struct CylindricityToleranceHandler;

#[step_entity(name = "CYLINDRICITY_TOLERANCE", pass = Pass8GeometricTolerance)]
impl SimpleEntityHandler for CylindricityToleranceHandler {
    type WriteInput = GeometricTolerance;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) =
            read_geometric_tolerance_data(ctx, entity_id, attrs, "CYLINDRICITY_TOLERANCE")?
        else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerances
            .push(GeometricTolerance::Cylindricity(data));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gt: GeometricTolerance) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance(buf, gt))
    }
}

/// Read the shared `general_datum_reference` 6-attr body. `Ok(None)` when
/// `of_shape` or `base` does not resolve — the entry is dropped, symmetric
/// on re-read. The 6th attr `modifiers` is not modelled and is ignored.
fn read_general_datum_reference_data(
    ctx: &ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    entity_name: &'static str,
) -> Result<Option<GeneralDatumReferenceData>, ConvertError> {
    check_count(attrs, 6, entity_id, entity_name)?;
    let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
    let of_shape_ref = read_entity_ref(attrs, 2, entity_id, "of_shape")?;
    let product_definitional = read_bool(attrs, 3, entity_id, "product_definitional")?;
    let base_ref = read_entity_ref(attrs, 4, entity_id, "base")?;
    // attr 5 (`modifiers`) — datum_reference_modifier set, not modelled.

    // of_shape → PRODUCT_DEFINITION_SHAPE → PRODUCT_DEFINITION → ProductId.
    let Some(&pdef_step_id) = ctx.pdef_shape_to_pdef.get(&of_shape_ref) else {
        return Ok(None);
    };
    let Some(&product_step_id) = ctx.pdef_to_product.get(&pdef_step_id) else {
        return Ok(None);
    };
    let Some(&target) = ctx.product_arena_map.get(&product_step_id) else {
        return Ok(None);
    };
    // base — `datum_or_common_datum`; only the `datum` member is modelled.
    let Some(&datum_id) = ctx.datum_id_map.get(&base_ref) else {
        return Ok(None);
    };

    Ok(Some(GeneralDatumReferenceData {
        name,
        description,
        target,
        product_definitional,
        base: GeneralDatumBase::Datum(datum_id),
    }))
}

/// Emit a `GeneralDatumReference` under the STEP entity name its variant
/// selects, returning the STEP id. Shared by both handlers and by
/// `emit_general_datum_references`.
pub(crate) fn write_general_datum_reference(
    buf: &mut WriteBuffer,
    gdr: GeneralDatumReference,
) -> u64 {
    let (entity_name, data) = match gdr {
        GeneralDatumReference::Compartment(d) => ("DATUM_REFERENCE_COMPARTMENT", d),
        GeneralDatumReference::Element(d) => ("DATUM_REFERENCE_ELEMENT", d),
    };
    // `target` → PRODUCT_DEFINITION_SHAPE step id. A miss is the kernel-built
    // IR defensive case (no product chain) — in practice unreachable, since a
    // general_datum_reference only enters the arena once `of_shape` resolved.
    let pds_step_id = buf
        .product_def_shape_ids
        .get(&data.target)
        .copied()
        .unwrap_or(0);
    let base_step_id = match data.base {
        GeneralDatumBase::Datum(id) => buf.datum_step_ids[id.0 as usize],
    };
    let bool_attr = if data.product_definitional { "T" } else { "F" };
    buf.push_simple(
        entity_name,
        vec![
            Attribute::String(data.name),
            Attribute::String(data.description),
            Attribute::EntityRef(pds_step_id),
            Attribute::Enum(bool_attr.into()),
            Attribute::EntityRef(base_step_id),
            // modifiers — not modelled, always emitted as `$`.
            Attribute::Unset,
        ],
    )
}

pub(crate) struct DatumReferenceCompartmentHandler;

#[step_entity(name = "DATUM_REFERENCE_COMPARTMENT", pass = Pass8GeneralDatumReference)]
impl SimpleEntityHandler for DatumReferenceCompartmentHandler {
    type WriteInput = GeneralDatumReference;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_general_datum_reference_data(
            ctx,
            entity_id,
            attrs,
            "DATUM_REFERENCE_COMPARTMENT",
        )?
        else {
            return Ok(());
        };
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .general_datum_references
            .push(GeneralDatumReference::Compartment(data));
        ctx.general_datum_reference_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gdr: GeneralDatumReference) -> Result<u64, WriteError> {
        Ok(write_general_datum_reference(buf, gdr))
    }
}

pub(crate) struct DatumReferenceElementHandler;

#[step_entity(name = "DATUM_REFERENCE_ELEMENT", pass = Pass8GeneralDatumReference)]
impl SimpleEntityHandler for DatumReferenceElementHandler {
    type WriteInput = GeneralDatumReference;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) =
            read_general_datum_reference_data(ctx, entity_id, attrs, "DATUM_REFERENCE_ELEMENT")?
        else {
            return Ok(());
        };
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .general_datum_references
            .push(GeneralDatumReference::Element(data));
        ctx.general_datum_reference_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gdr: GeneralDatumReference) -> Result<u64, WriteError> {
        Ok(write_general_datum_reference(buf, gdr))
    }
}

/// Read the shared `geometric_tolerance_with_datum_reference` 5-attr body.
/// `Ok(None)` when `magnitude` or `toleranced_shape_aspect` does not
/// resolve — the tolerance is dropped, symmetric on re-read. Individual
/// `datum_system` refs that do not resolve are skipped.
/// Resolve the shared `geometric_tolerance_with_datum_reference` body from
/// already-read raw refs. `None` when `magnitude` or `toleranced_shape_aspect`
/// does not resolve; individual `datum_system` refs that do not resolve are
/// skipped. Shared by the simple-form and complex-form readers.
fn build_gt_with_datum_reference_data(
    ctx: &ReaderContext,
    name: String,
    description: String,
    magnitude_ref: u64,
    shape_aspect_ref: u64,
    datum_system_refs: &[u64],
) -> Option<GeometricToleranceWithDatumReferenceData> {
    let magnitude = resolve_tolerance_magnitude(ctx, magnitude_ref)?;
    let toleranced_shape_aspect = resolve_shape_aspect_ref(ctx, shape_aspect_ref)?;
    let mut datum_system = Vec::with_capacity(datum_system_refs.len());
    for &r in datum_system_refs {
        if let Some(&id) = ctx.datum_system_id_map.get(&r) {
            datum_system.push(id);
        }
    }
    Some(GeometricToleranceWithDatumReferenceData {
        name,
        description,
        magnitude,
        toleranced_shape_aspect,
        datum_system,
    })
}

/// Read the simple 5-attr `geometric_tolerance_with_datum_reference` body
/// (the form the seven direct subtypes take). `Ok(None)` when a ref does
/// not resolve — the tolerance is dropped, symmetric on re-read.
fn read_geometric_tolerance_with_datum_reference_data(
    ctx: &ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    entity_name: &'static str,
) -> Result<Option<GeometricToleranceWithDatumReferenceData>, ConvertError> {
    check_count(attrs, 5, entity_id, entity_name)?;
    let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
    let magnitude_ref = read_entity_ref(attrs, 2, entity_id, "magnitude")?;
    let shape_aspect_ref = read_entity_ref(attrs, 3, entity_id, "toleranced_shape_aspect")?;
    let datum_system_refs = read_entity_ref_list(attrs, 4, entity_id, "datum_system")?;
    Ok(build_gt_with_datum_reference_data(
        ctx,
        name,
        description,
        magnitude_ref,
        shape_aspect_ref,
        &datum_system_refs,
    ))
}

/// Read the multiple-inheritance complex form `(GEOMETRIC_TOLERANCE
/// GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE <leaf>)` — the encoding
/// `POSITION` / `SURFACE_PROFILE` / `LINE_PROFILE` tolerances take. `Ok(None)`
/// when a ref does not resolve. A `GEOMETRIC_TOLERANCE_WITH_MODIFIERS` part
/// (present in some instances) is not modelled and is ignored.
fn read_gt_with_datum_reference_complex(
    ctx: &ReaderContext,
    entity_id: u64,
    parts: &[RawEntityPart],
) -> Result<Option<GeometricToleranceWithDatumReferenceData>, ConvertError> {
    let gt_attrs = require_part_attrs(parts, "GEOMETRIC_TOLERANCE", entity_id)?;
    check_count(gt_attrs, 4, entity_id, "GEOMETRIC_TOLERANCE")?;
    let name = read_string_or_unset(gt_attrs, 0, entity_id, "name")?.to_owned();
    let description = read_string_or_unset(gt_attrs, 1, entity_id, "description")?.to_owned();
    let magnitude_ref = read_entity_ref(gt_attrs, 2, entity_id, "magnitude")?;
    let shape_aspect_ref = read_entity_ref(gt_attrs, 3, entity_id, "toleranced_shape_aspect")?;
    let gtwdr_attrs =
        require_part_attrs(parts, "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE", entity_id)?;
    let datum_system_refs = read_entity_ref_list(gtwdr_attrs, 0, entity_id, "datum_system")?;
    Ok(build_gt_with_datum_reference_data(
        ctx,
        name,
        description,
        magnitude_ref,
        shape_aspect_ref,
        &datum_system_refs,
    ))
}

/// Emit a `GeometricToleranceWithDatumReference`, returning the STEP id.
/// The seven direct subtypes emit as a simple 5-attr entity; `POSITION` /
/// `SURFACE_PROFILE` / `LINE_PROFILE` emit as the multiple-inheritance
/// complex `(GEOMETRIC_TOLERANCE GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE
/// <leaf>)` (parts in ISO 10303-21 alphabetical order).
pub(crate) fn write_geometric_tolerance_with_datum_reference(
    buf: &mut WriteBuffer,
    gt: GeometricToleranceWithDatumReference,
) -> u64 {
    // `type_name` is the STEP entity name for the simple variants and the
    // leaf part name for the complex variants.
    let (type_name, is_complex, data) = match gt {
        GeometricToleranceWithDatumReference::Angularity(d) => ("ANGULARITY_TOLERANCE", false, d),
        GeometricToleranceWithDatumReference::CircularRunout(d) => {
            ("CIRCULAR_RUNOUT_TOLERANCE", false, d)
        }
        GeometricToleranceWithDatumReference::Concentricity(d) => {
            ("CONCENTRICITY_TOLERANCE", false, d)
        }
        GeometricToleranceWithDatumReference::Parallelism(d) => ("PARALLELISM_TOLERANCE", false, d),
        GeometricToleranceWithDatumReference::Perpendicularity(d) => {
            ("PERPENDICULARITY_TOLERANCE", false, d)
        }
        GeometricToleranceWithDatumReference::Symmetry(d) => ("SYMMETRY_TOLERANCE", false, d),
        GeometricToleranceWithDatumReference::TotalRunout(d) => {
            ("TOTAL_RUNOUT_TOLERANCE", false, d)
        }
        GeometricToleranceWithDatumReference::Position(d) => ("POSITION_TOLERANCE", true, d),
        GeometricToleranceWithDatumReference::SurfaceProfile(d) => {
            ("SURFACE_PROFILE_TOLERANCE", true, d)
        }
        GeometricToleranceWithDatumReference::LineProfile(d) => ("LINE_PROFILE_TOLERANCE", true, d),
    };
    let magnitude = match data.magnitude {
        ToleranceMagnitude::MeasureWithUnit(id) => buf.mwu_step_ids[id.0 as usize],
        ToleranceMagnitude::Measure(m) => buf.emit_property_measure(&m, None),
    };
    let shape_aspect = buf.emit_shape_aspect_ref(data.toleranced_shape_aspect);
    let mut datum_system_refs = Vec::with_capacity(data.datum_system.len());
    for ds_id in &data.datum_system {
        datum_system_refs.push(Attribute::EntityRef(
            buf.datum_system_step_ids[ds_id.0 as usize],
        ));
    }
    if is_complex {
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex {
                parts: vec![
                    (
                        "GEOMETRIC_TOLERANCE".into(),
                        vec![
                            Attribute::String(data.name),
                            Attribute::String(data.description),
                            Attribute::EntityRef(magnitude),
                            Attribute::EntityRef(shape_aspect),
                        ],
                    ),
                    (
                        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE".into(),
                        vec![Attribute::List(datum_system_refs)],
                    ),
                    (type_name.into(), vec![]),
                ],
            },
        });
        n
    } else {
        buf.push_simple(
            type_name,
            vec![
                Attribute::String(data.name),
                Attribute::String(data.description),
                Attribute::EntityRef(magnitude),
                Attribute::EntityRef(shape_aspect),
                Attribute::List(datum_system_refs),
            ],
        )
    }
}

pub(crate) struct AngularityToleranceHandler;

#[step_entity(name = "ANGULARITY_TOLERANCE", pass = Pass8GtWithDatumReference)]
impl SimpleEntityHandler for AngularityToleranceHandler {
    type WriteInput = GeometricToleranceWithDatumReference;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_geometric_tolerance_with_datum_reference_data(
            ctx,
            entity_id,
            attrs,
            "ANGULARITY_TOLERANCE",
        )?
        else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerance_with_datum_references
            .push(GeometricToleranceWithDatumReference::Angularity(data));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
}

pub(crate) struct CircularRunoutToleranceHandler;

#[step_entity(name = "CIRCULAR_RUNOUT_TOLERANCE", pass = Pass8GtWithDatumReference)]
impl SimpleEntityHandler for CircularRunoutToleranceHandler {
    type WriteInput = GeometricToleranceWithDatumReference;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_geometric_tolerance_with_datum_reference_data(
            ctx,
            entity_id,
            attrs,
            "CIRCULAR_RUNOUT_TOLERANCE",
        )?
        else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerance_with_datum_references
            .push(GeometricToleranceWithDatumReference::CircularRunout(data));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
}

pub(crate) struct ConcentricityToleranceHandler;

#[step_entity(name = "CONCENTRICITY_TOLERANCE", pass = Pass8GtWithDatumReference)]
impl SimpleEntityHandler for ConcentricityToleranceHandler {
    type WriteInput = GeometricToleranceWithDatumReference;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_geometric_tolerance_with_datum_reference_data(
            ctx,
            entity_id,
            attrs,
            "CONCENTRICITY_TOLERANCE",
        )?
        else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerance_with_datum_references
            .push(GeometricToleranceWithDatumReference::Concentricity(data));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
}

pub(crate) struct ParallelismToleranceHandler;

#[step_entity(name = "PARALLELISM_TOLERANCE", pass = Pass8GtWithDatumReference)]
impl SimpleEntityHandler for ParallelismToleranceHandler {
    type WriteInput = GeometricToleranceWithDatumReference;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_geometric_tolerance_with_datum_reference_data(
            ctx,
            entity_id,
            attrs,
            "PARALLELISM_TOLERANCE",
        )?
        else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerance_with_datum_references
            .push(GeometricToleranceWithDatumReference::Parallelism(data));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
}

pub(crate) struct PerpendicularityToleranceHandler;

#[step_entity(name = "PERPENDICULARITY_TOLERANCE", pass = Pass8GtWithDatumReference)]
impl SimpleEntityHandler for PerpendicularityToleranceHandler {
    type WriteInput = GeometricToleranceWithDatumReference;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_geometric_tolerance_with_datum_reference_data(
            ctx,
            entity_id,
            attrs,
            "PERPENDICULARITY_TOLERANCE",
        )?
        else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerance_with_datum_references
            .push(GeometricToleranceWithDatumReference::Perpendicularity(data));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
}

pub(crate) struct SymmetryToleranceHandler;

#[step_entity(name = "SYMMETRY_TOLERANCE", pass = Pass8GtWithDatumReference)]
impl SimpleEntityHandler for SymmetryToleranceHandler {
    type WriteInput = GeometricToleranceWithDatumReference;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_geometric_tolerance_with_datum_reference_data(
            ctx,
            entity_id,
            attrs,
            "SYMMETRY_TOLERANCE",
        )?
        else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerance_with_datum_references
            .push(GeometricToleranceWithDatumReference::Symmetry(data));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
}

pub(crate) struct TotalRunoutToleranceHandler;

#[step_entity(name = "TOTAL_RUNOUT_TOLERANCE", pass = Pass8GtWithDatumReference)]
impl SimpleEntityHandler for TotalRunoutToleranceHandler {
    type WriteInput = GeometricToleranceWithDatumReference;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_geometric_tolerance_with_datum_reference_data(
            ctx,
            entity_id,
            attrs,
            "TOTAL_RUNOUT_TOLERANCE",
        )?
        else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerance_with_datum_references
            .push(GeometricToleranceWithDatumReference::TotalRunout(data));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
}

pub(crate) struct PositionToleranceHandler;

#[step_entity_complex(
    name = "POSITION_TOLERANCE",
    pass = Pass8GtWithDatumReference,
    required = [
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE",
        "POSITION_TOLERANCE"
    ]
)]
impl ComplexEntityHandler for PositionToleranceHandler {
    type WriteInput = GeometricToleranceWithDatumReference;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_gt_with_datum_reference_complex(ctx, entity_id, parts)? else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerance_with_datum_references
            .push(GeometricToleranceWithDatumReference::Position(data));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
}

pub(crate) struct SurfaceProfileToleranceHandler;

#[step_entity_complex(
    name = "SURFACE_PROFILE_TOLERANCE",
    pass = Pass8GtWithDatumReference,
    required = [
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE",
        "SURFACE_PROFILE_TOLERANCE"
    ]
)]
impl ComplexEntityHandler for SurfaceProfileToleranceHandler {
    type WriteInput = GeometricToleranceWithDatumReference;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_gt_with_datum_reference_complex(ctx, entity_id, parts)? else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerance_with_datum_references
            .push(GeometricToleranceWithDatumReference::SurfaceProfile(data));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
}

pub(crate) struct LineProfileToleranceHandler;

#[step_entity_complex(
    name = "LINE_PROFILE_TOLERANCE",
    pass = Pass8GtWithDatumReference,
    required = [
        "GEOMETRIC_TOLERANCE",
        "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE",
        "LINE_PROFILE_TOLERANCE"
    ]
)]
impl ComplexEntityHandler for LineProfileToleranceHandler {
    type WriteInput = GeometricToleranceWithDatumReference;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_gt_with_datum_reference_complex(ctx, entity_id, parts)? else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerance_with_datum_references
            .push(GeometricToleranceWithDatumReference::LineProfile(data));
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
}

/// Emit a [`ToleranceMagnitude`] (`ref_measure_with_unit`) and return its
/// STEP id — a `MeasureWithUnit` references the units-pool step id, a
/// `Measure` is emitted inline as a `MEASURE_REPRESENTATION_ITEM`.
fn emit_tolerance_magnitude(buf: &mut WriteBuffer, m: &ToleranceMagnitude) -> u64 {
    match m {
        ToleranceMagnitude::MeasureWithUnit(id) => buf.mwu_step_ids[id.0 as usize],
        ToleranceMagnitude::Measure(pm) => buf.emit_property_measure(pm, None),
    }
}

pub(crate) struct ToleranceValueHandler;

#[step_entity(name = "TOLERANCE_VALUE", pass = Pass8ToleranceValue)]
impl SimpleEntityHandler for ToleranceValueHandler {
    type WriteInput = ToleranceValue;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "TOLERANCE_VALUE")?;
        let lower_ref = read_entity_ref(attrs, 0, entity_id, "lower_bound")?;
        let upper_ref = read_entity_ref(attrs, 1, entity_id, "upper_bound")?;
        let Some(lower_bound) = resolve_tolerance_magnitude(ctx, lower_ref) else {
            return Ok(());
        };
        let Some(upper_bound) = resolve_tolerance_magnitude(ctx, upper_ref) else {
            return Ok(());
        };
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .tolerance_values
            .push(ToleranceValue {
                lower_bound,
                upper_bound,
            });
        ctx.tolerance_value_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, tv: ToleranceValue) -> Result<u64, WriteError> {
        Ok(write_tolerance_value(buf, &tv))
    }
}

/// Emit a `TOLERANCE_VALUE`, returning the STEP id.
pub(crate) fn write_tolerance_value(buf: &mut WriteBuffer, tv: &ToleranceValue) -> u64 {
    let lower = emit_tolerance_magnitude(buf, &tv.lower_bound);
    let upper = emit_tolerance_magnitude(buf, &tv.upper_bound);
    buf.push_simple(
        "TOLERANCE_VALUE",
        vec![Attribute::EntityRef(lower), Attribute::EntityRef(upper)],
    )
}

pub(crate) struct LimitsAndFitsHandler;

#[step_entity(name = "LIMITS_AND_FITS", pass = Pass8ToleranceValue)]
impl SimpleEntityHandler for LimitsAndFitsHandler {
    type WriteInput = LimitsAndFits;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "LIMITS_AND_FITS")?;
        let form_variance = read_string_or_unset(attrs, 0, entity_id, "form_variance")?.to_owned();
        let zone_variance = read_string_or_unset(attrs, 1, entity_id, "zone_variance")?.to_owned();
        let grade = read_string_or_unset(attrs, 2, entity_id, "grade")?.to_owned();
        let source = read_string_or_unset(attrs, 3, entity_id, "source")?.to_owned();
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .limits_and_fits
            .push(LimitsAndFits {
                form_variance,
                zone_variance,
                grade,
                source,
            });
        ctx.limits_and_fits_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, lf: LimitsAndFits) -> Result<u64, WriteError> {
        Ok(write_limits_and_fits(buf, lf))
    }
}

/// Emit a `LIMITS_AND_FITS`, returning the STEP id.
pub(crate) fn write_limits_and_fits(buf: &mut WriteBuffer, lf: LimitsAndFits) -> u64 {
    buf.push_simple(
        "LIMITS_AND_FITS",
        vec![
            Attribute::String(lf.form_variance),
            Attribute::String(lf.zone_variance),
            Attribute::String(lf.grade),
            Attribute::String(lf.source),
        ],
    )
}

/// Resolve a `tolerance_method_definition` SELECT ref (`PLUS_MINUS_TOLERANCE`'s
/// `range`) — a `TOLERANCE_VALUE` or a `LIMITS_AND_FITS`.
fn resolve_tolerance_method_definition(
    ctx: &ReaderContext,
    item_ref: u64,
) -> Option<ToleranceMethodDefinition> {
    if let Some(&id) = ctx.tolerance_value_id_map.get(&item_ref) {
        return Some(ToleranceMethodDefinition::Value(id));
    }
    ctx.limits_and_fits_id_map
        .get(&item_ref)
        .copied()
        .map(ToleranceMethodDefinition::LimitsAndFits)
}

/// Resolve a `dimensional_characteristic` SELECT ref (`PLUS_MINUS_TOLERANCE`'s
/// `toleranced_dimension`) — a `dimensional_location` or `dimensional_size`.
fn resolve_dimensional_characteristic(
    ctx: &ReaderContext,
    item_ref: u64,
) -> Option<DimensionalCharacteristic> {
    if let Some(&id) = ctx.dimensional_location_id_map.get(&item_ref) {
        return Some(DimensionalCharacteristic::Location(id));
    }
    ctx.dimensional_size_id_map
        .get(&item_ref)
        .copied()
        .map(DimensionalCharacteristic::Size)
}

pub(crate) struct PlusMinusToleranceHandler;

#[step_entity(name = "PLUS_MINUS_TOLERANCE", pass = Pass8PlusMinusTolerance)]
impl SimpleEntityHandler for PlusMinusToleranceHandler {
    type WriteInput = PlusMinusTolerance;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "PLUS_MINUS_TOLERANCE")?;
        let range_ref = read_entity_ref(attrs, 0, entity_id, "range")?;
        let dimension_ref = read_entity_ref(attrs, 1, entity_id, "toleranced_dimension")?;
        let Some(range) = resolve_tolerance_method_definition(ctx, range_ref) else {
            return Ok(());
        };
        let Some(toleranced_dimension) = resolve_dimensional_characteristic(ctx, dimension_ref)
        else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .plus_minus_tolerances
            .push(PlusMinusTolerance {
                range,
                toleranced_dimension,
            });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, pmt: PlusMinusTolerance) -> Result<u64, WriteError> {
        Ok(write_plus_minus_tolerance(buf, &pmt))
    }
}

/// Emit a `PLUS_MINUS_TOLERANCE`, returning the STEP id.
pub(crate) fn write_plus_minus_tolerance(buf: &mut WriteBuffer, pmt: &PlusMinusTolerance) -> u64 {
    let range = match pmt.range {
        ToleranceMethodDefinition::Value(id) => buf.tolerance_value_step_ids[id.0 as usize],
        ToleranceMethodDefinition::LimitsAndFits(id) => buf.limits_and_fits_step_ids[id.0 as usize],
    };
    let dimension = match pmt.toleranced_dimension {
        DimensionalCharacteristic::Location(id) => buf.dimensional_location_step_ids[id.0 as usize],
        DimensionalCharacteristic::Size(id) => buf.dimensional_size_step_ids[id.0 as usize],
    };
    buf.push_simple(
        "PLUS_MINUS_TOLERANCE",
        vec![Attribute::EntityRef(range), Attribute::EntityRef(dimension)],
    )
}
