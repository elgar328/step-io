//! `pmi` pool entity handlers — Pass 8.
//!
//! Three dependency-free `single_struct` primitives — `TOLERANCE_ZONE_FORM`,
//! `TYPE_QUALIFIER`, `VALUE_FORMAT_TYPE_QUALIFIER` — each a 1-attr string
//! entity pushed into [`PmiPool`]. They have no entity references; the
//! GD&T entities that consume them arrive in later phases.

use crate::entities::SimpleEntityHandler;
use crate::entities::shape_rep::shape_aspect::ShapeAspectWriteInput;
use crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::ir::PmiPool;
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref, read_entity_ref_list, read_enum, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::pmi::{
    AngleSelection, AngularLocationData, AnnotationOccurrence, AnnotationPlane, Datum,
    DatumFeature, DimensionalLocation, DimensionalLocationData, DimensionalSize,
    DimensionalSizeKind, DraughtingPreDefinedTextFont, TessellatedAnnotationOccurrence,
    ToleranceZoneForm, TypeQualifier, ValueFormatTypeQualifier,
};
use crate::parser::entity::{Attribute, EntityGraph};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use step_io_macros::step_entity;

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

        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .dimensional_sizes
            .push(DimensionalSize {
                applies_to,
                name,
                kind: DimensionalSizeKind::Plain,
            });
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

        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .dimensional_sizes
            .push(DimensionalSize {
                applies_to,
                name,
                kind: DimensionalSizeKind::Angular(angle_selection),
            });
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
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .dimensional_locations
            .push(DimensionalLocation::Plain(data));
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
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .dimensional_locations
            .push(DimensionalLocation::Directed(data));
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

        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .dimensional_locations
            .push(DimensionalLocation::Angular(AngularLocationData {
                name,
                description,
                relating_shape_aspect,
                related_shape_aspect,
                angle_selection,
            }));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dl: DimensionalLocation) -> Result<u64, WriteError> {
        Ok(write_dimensional_location(buf, dl))
    }
}
