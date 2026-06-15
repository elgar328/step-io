//! `pmi` pool entity handlers.
//!
//! Three dependency-free `single_struct` primitives — `TOLERANCE_ZONE_FORM`,
//! `TYPE_QUALIFIER`, `VALUE_FORMAT_TYPE_QUALIFIER` — each a 1-attr string
//! entity pushed into [`PmiPool`]. They have no entity references; the
//! GD&T entities that consume them arrive in later phases.

use crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::GeometricToleranceTarget;
use crate::ir::PmiPool;
use crate::ir::ShapeAspectRef;
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref, read_entity_ref_list, read_enum, read_real,
    read_real_list, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::geometry::Point3;
use crate::ir::pmi::{
    AnnotationOccurrence, AnnotationOccurrenceAssociativity, AnnotationOccurrenceRef,
    AnnotationPlaceholderLeaderLine, AnnotationPlaceholderOccurrence,
    AnnotationPlaceholderOccurrenceWithLeaderLine, AnnotationPlane, AnnotationSymbolOccurrence,
    AnnotationTextOccurrence, AnnotationToModelLeaderLine, ApllPointData, ApllPointElement,
    ApllPointWithSurfaceData, AuxiliaryLeaderLineData, DatumFeature, DimensionalCharacteristic,
    DimensionalLocation, DimensionalSize, DimensionalSizeKind, DimensionalSizeWithDatumFeatureData,
    DraughtingAnnotationOccurrence, DraughtingCalloutData, DraughtingCalloutElement,
    DraughtingCalloutRelationship, DraughtingModelIdentifiedItem, DraughtingModelItemAssociation,
    DraughtingModelItemDefinition, DraughtingPreDefinedTextFont, GeneralDatumBase,
    GeneralDatumReference, GeneralDatumReferenceData, GeometricTolerance, GeometricToleranceData,
    GeometricToleranceRef, GeometricToleranceRelationship, GeometricToleranceWithDatumReference,
    GeometricToleranceWithDatumReferenceData, LeaderCurve, LeaderTerminator, LimitsAndFits,
    MeasureQualification, PlainAnnotationCurveOccurrence, PlainAnnotationOccurrence,
    PlusMinusTolerance, ProjectedZoneDefinition, TerminatorSymbol, TessellatedAnnotationOccurrence,
    ToleranceMagnitude, ToleranceMethodDefinition, ToleranceValue, ToleranceZoneForm,
    TypeQualifier, ValueFormatTypeQualifier,
};
use crate::parser::entity::{Attribute, EntityGraph, RawEntity, RawEntityPart};
use crate::reader::{ReaderContext, find_part_attrs, require_part_attrs};
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
use crate::writer::entity::{WriterBody, WriterEntity};
use step_io_macros::{step_entity, step_entity_complex};

pub(crate) struct ToleranceZoneFormHandler;

#[step_entity(name = "TOLERANCE_ZONE_FORM")]
impl SimpleEntityHandler for ToleranceZoneFormHandler {
    type WriteInput = ToleranceZoneForm;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_tolerance_zone_form(entity_id, attrs)?;
        crate::early::lower::lower_tolerance_zone_form(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, tzf: ToleranceZoneForm) -> Result<u64, WriteError> {
        let early = crate::early::lift::lift_tolerance_zone_form(tzf.name);
        Ok(crate::early::serialize::serialize_tolerance_zone_form(
            buf, &early,
        ))
    }
}

pub(crate) struct TypeQualifierHandler;

#[step_entity(name = "TYPE_QUALIFIER")]
impl SimpleEntityHandler for TypeQualifierHandler {
    type WriteInput = TypeQualifier;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_type_qualifier(entity_id, attrs)?;
        crate::early::lower::lower_type_qualifier(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, tq: TypeQualifier) -> Result<u64, WriteError> {
        let early = crate::early::lift::lift_type_qualifier(tq.name);
        Ok(crate::early::serialize::serialize_type_qualifier(
            buf, &early,
        ))
    }
}

pub(crate) struct ValueFormatTypeQualifierHandler;

#[step_entity(name = "VALUE_FORMAT_TYPE_QUALIFIER")]
impl SimpleEntityHandler for ValueFormatTypeQualifierHandler {
    type WriteInput = ValueFormatTypeQualifier;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_value_format_type_qualifier(entity_id, attrs)?;
        crate::early::lower::lower_value_format_type_qualifier(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, vftq: ValueFormatTypeQualifier) -> Result<u64, WriteError> {
        let early = crate::early::lift::lift_value_format_type_qualifier(vftq.format_type);
        Ok(crate::early::serialize::serialize_value_format_type_qualifier(buf, &early))
    }
}

pub(crate) struct DraughtingPreDefinedTextFontHandler;

#[step_entity(name = "DRAUGHTING_PRE_DEFINED_TEXT_FONT")]
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
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .draughting_pre_defined_text_fonts
            .push(DraughtingPreDefinedTextFont { name });
        ctx.id_cache.insert(entity_id, id);
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
/// subtype, on the 2-layer path (`bind`/`lower` read, `lift`/`serialize`
/// write). `styles` keeps only resolved `PresentationStyleAssignment` refs and
/// `item` goes through the shared `representation_item` resolver; the 4th
/// attribute `elements` (an `annotation_plane_element` list) is not modelled —
/// `lower` ignores it and `lift` emits `None`, re-serialized as `$` (matching
/// the legacy writer's unconditional unset). An `ANNOTATION_PLANE` whose `item`
/// does not resolve is silently dropped, symmetric on re-read.
#[step_entity(name = "ANNOTATION_PLANE")]
impl SimpleEntityHandler for AnnotationPlaneHandler {
    type WriteInput = AnnotationPlane;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_annotation_plane(entity_id, attrs)?;
        crate::early::lower::lower_annotation_plane(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ap: AnnotationPlane) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(ap.item)?;
        let mut style_refs = Vec::with_capacity(ap.styles.len());
        for psa_id in ap.styles {
            style_refs.push(buf.step_id(psa_id));
        }
        let early = crate::early::lift::lift_annotation_plane(ap.name, style_refs, item_id);
        Ok(crate::early::serialize::serialize_annotation_plane(
            buf, &early,
        ))
    }
}

pub(crate) struct TessellatedAnnotationOccurrenceHandler;

/// `TESSELLATED_ANNOTATION_OCCURRENCE(name, styles, item)` — an
/// `annotation_occurrence` subtype. `styles` resolves through
/// `viz_psa_id_map` (like `ANNOTATION_PLANE`); `item` is a
/// `TESSELLATED_GEOMETRIC_SET` resolved through `tessellated_item_id_map`.
/// An occurrence whose `item` does not resolve is silently dropped.
#[step_entity(name = "TESSELLATED_ANNOTATION_OCCURRENCE")]
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
            if let Some(psa_id) = ctx
                .id_cache
                .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
            {
                styles.push(psa_id);
            }
        }
        let Some(item) = ctx
            .id_cache
            .get::<crate::ir::id::TessellatedItemId>(item_ref)
        else {
            return Ok(()); // item unresolved — drop the occurrence
        };

        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .annotation_occurrences
            .push(AnnotationOccurrence::TessellatedAnnotationOccurrence(
                TessellatedAnnotationOccurrence { name, styles, item },
            ));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        tao: TessellatedAnnotationOccurrence,
    ) -> Result<u64, WriteError> {
        let item = buf.step_id(tao.item);
        let mut style_refs = Vec::with_capacity(tao.styles.len());
        for psa_id in tao.styles {
            style_refs.push(Attribute::EntityRef(buf.step_id(psa_id)));
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

pub(crate) struct AnnotationSymbolOccurrenceHandler;

/// `ANNOTATION_SYMBOL_OCCURRENCE(name, styles, item)` — an
/// `annotation_occurrence` subtype whose `item` is the
/// `annotation_symbol_occurrence_item` SELECT. step-io resolves `item`
/// through the generic `representation_item` resolver
/// (`resolve_representation_item_ref`); occurrences whose `item` does not
/// resolve are silently dropped, symmetric on re-read.
#[step_entity(name = "ANNOTATION_SYMBOL_OCCURRENCE")]
impl SimpleEntityHandler for AnnotationSymbolOccurrenceHandler {
    type WriteInput = AnnotationSymbolOccurrence;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_annotation_symbol_occurrence(entity_id, attrs)?;
        crate::early::lower::lower_annotation_symbol_occurrence(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, aso: AnnotationSymbolOccurrence) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(aso.item)?;
        let mut style_refs = Vec::with_capacity(aso.styles.len());
        for psa_id in aso.styles {
            style_refs.push(buf.step_id(psa_id));
        }
        let early =
            crate::early::lift::lift_annotation_symbol_occurrence(aso.name, style_refs, item_id);
        Ok(crate::early::serialize::serialize_annotation_symbol_occurrence(buf, &early))
    }
}

pub(crate) struct AnnotationTextOccurrenceHandler;

/// Styled `ANNOTATION_TEXT_OCCURRENCE` — read only as the AND-combined complex
/// `(ANNOTATION_OCCURRENCE ANNOTATION_TEXT_OCCURRENCE DRAUGHTING_ANNOTATION_OCCURRENCE
/// GEOMETRIC_REPRESENTATION_ITEM REPRESENTATION_ITEM STYLED_ITEM)`, the only form
/// in the corpus (the simple single-name entity has count 0). `name` is on the
/// `REPRESENTATION_ITEM` part, `styles` + `item` on `STYLED_ITEM`; `item` is the
/// `annotation_text_occurrence_item` SELECT (`TEXT_LITERAL` / `COMPOSITE_TEXT`),
/// resolved through `resolve_representation_item_ref` (unresolved drops the
/// occurrence, symmetric on re-read).
#[step_entity_complex(
    name = "ANNOTATION_TEXT_OCCURRENCE",
    cases = [[
        "ANNOTATION_OCCURRENCE",
        "ANNOTATION_TEXT_OCCURRENCE",
        "DRAUGHTING_ANNOTATION_OCCURRENCE",
        "GEOMETRIC_REPRESENTATION_ITEM",
        "REPRESENTATION_ITEM",
        "STYLED_ITEM",
    ]]
)]
impl ComplexEntityHandler for AnnotationTextOccurrenceHandler {
    type WriteInput = AnnotationTextOccurrence;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_annotation_text_occurrence(entity_id, parts)?;
        crate::early::lower::lower_annotation_text_occurrence(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ato: AnnotationTextOccurrence) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(ato.item)?;
        let mut style_refs = Vec::with_capacity(ato.styles.len());
        for psa_id in ato.styles {
            style_refs.push(buf.step_id(psa_id));
        }
        let early =
            crate::early::lift::lift_annotation_text_occurrence(ato.name, style_refs, item_id);
        Ok(crate::early::serialize::serialize_annotation_text_occurrence(buf, &early))
    }
}

pub(crate) struct DraughtingAnnotationOccurrenceHandler;

/// `DRAUGHTING_ANNOTATION_OCCURRENCE(name, styles, item)` — an
/// `annotation_occurrence` subtype whose `item` is narrowed (via WHERE
/// constraints) to `ref_representation_item`. step-io resolves `item`
/// through `resolve_representation_item_ref`; unresolved items are
/// silently dropped.
#[step_entity(name = "DRAUGHTING_ANNOTATION_OCCURRENCE")]
impl SimpleEntityHandler for DraughtingAnnotationOccurrenceHandler {
    type WriteInput = DraughtingAnnotationOccurrence;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_draughting_annotation_occurrence(entity_id, attrs)?;
        crate::early::lower::lower_draughting_annotation_occurrence(ctx, entity_id, &early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        dao: DraughtingAnnotationOccurrence,
    ) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(dao.item)?;
        let mut style_refs = Vec::with_capacity(dao.styles.len());
        for psa_id in dao.styles {
            style_refs.push(buf.step_id(psa_id));
        }
        let early = crate::early::lift::lift_draughting_annotation_occurrence(
            dao.name, style_refs, item_id,
        );
        Ok(crate::early::serialize::serialize_draughting_annotation_occurrence(buf, &early))
    }
}

pub(crate) struct ApllPointHandler;

/// `APLL_POINT(name, coordinates, symbol_applied)` — a PMI leader-line waypoint
/// (`cartesian_point` subtype). Kept in the dedicated `apll_points` arena
/// (blueprint `apll_point_standalone` recast), not the hot `point` arena.
#[step_entity(name = "APLL_POINT")]
impl SimpleEntityHandler for ApllPointHandler {
    type WriteInput = ApllPointElement;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "APLL_POINT")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let coords = read_real_list(attrs, 1, entity_id, "coordinates")?;
        if coords.len() != 3 {
            return Err(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!("APLL_POINT must have 3 coordinates, got {}", coords.len()),
            });
        }
        let symbol_applied = read_enum(attrs, 2, entity_id, "symbol_applied")?.to_owned();
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .apll_points
            .push(ApllPointElement::ApllPoint(ApllPointData {
                name,
                coordinates: Point3 {
                    x: coords[0],
                    y: coords[1],
                    z: coords[2],
                },
                symbol_applied,
            }));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, apll: ApllPointElement) -> Result<u64, WriteError> {
        match apll {
            ApllPointElement::ApllPoint(data) => Ok(buf.push_simple(
                "APLL_POINT",
                vec![
                    Attribute::String(data.name),
                    Attribute::List(vec![
                        Attribute::Real(data.coordinates.x),
                        Attribute::Real(data.coordinates.y),
                        Attribute::Real(data.coordinates.z),
                    ]),
                    Attribute::Enum(data.symbol_applied),
                ],
            )),
            ApllPointElement::ApllPointWithSurface(data) => {
                // Surfaces (`face_surface`) are emitted in the topology pass,
                // well before the PMI pass, so `face_ids` is populated here.
                let surface_step = buf.step_id(data.associated_surface);
                Ok(buf.push_simple(
                    "APLL_POINT_WITH_SURFACE",
                    vec![
                        Attribute::String(data.name),
                        Attribute::List(vec![
                            Attribute::Real(data.coordinates.x),
                            Attribute::Real(data.coordinates.y),
                            Attribute::Real(data.coordinates.z),
                        ]),
                        Attribute::Enum(data.symbol_applied),
                        Attribute::EntityRef(surface_step),
                    ],
                ))
            }
        }
    }
}

pub(crate) struct ApllPointWithSurfaceHandler;

/// `APLL_POINT_WITH_SURFACE(name, coordinates, symbol_applied, associated_surface)`
/// — an `apll_point` projected onto a `face_surface`. Shares the `apll_points`
/// arena (and `apll_point_id_map`) with [`ApllPointHandler`] so leader-line
/// `geometric_elements` resolve uniformly; emission is handled by
/// `ApllPointHandler::write`.
#[step_entity(name = "APLL_POINT_WITH_SURFACE")]
impl SimpleEntityHandler for ApllPointWithSurfaceHandler {
    type WriteInput = ApllPointElement;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "APLL_POINT_WITH_SURFACE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let coords = read_real_list(attrs, 1, entity_id, "coordinates")?;
        if coords.len() != 3 {
            return Err(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "APLL_POINT_WITH_SURFACE must have 3 coordinates, got {}",
                    coords.len()
                ),
            });
        }
        let symbol_applied = read_enum(attrs, 2, entity_id, "symbol_applied")?.to_owned();
        let surface_ref = read_entity_ref(attrs, 3, entity_id, "associated_surface")?;
        let Some(associated_surface) = ctx.id_cache.get::<crate::ir::id::FaceId>(surface_ref)
        else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "APLL_POINT_WITH_SURFACE.associated_surface #{surface_ref} did not resolve to \
                     a known face_surface"
                ),
            });
            return Ok(());
        };
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .apll_points
            .push(ApllPointElement::ApllPointWithSurface(
                ApllPointWithSurfaceData {
                    name,
                    coordinates: Point3 {
                        x: coords[0],
                        y: coords[1],
                        z: coords[2],
                    },
                    symbol_applied,
                    associated_surface,
                },
            ));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _apll: ApllPointElement) -> Result<u64, WriteError> {
        unreachable!("APLL_POINT_WITH_SURFACE is emitted via ApllPointHandler::write")
    }
}

pub(crate) struct AnnotationToModelLeaderLineHandler;

/// `ANNOTATION_TO_MODEL_LEADER_LINE(name, geometric_elements)` — a PMI leader
/// line. `geometric_elements` is the inherited `LIST [2:?] OF des_apll_point_select`
/// resolved to the `apll_points` arena; unresolved members skip.
#[step_entity(name = "ANNOTATION_TO_MODEL_LEADER_LINE")]
impl SimpleEntityHandler for AnnotationToModelLeaderLineHandler {
    type WriteInput = AnnotationPlaceholderLeaderLine;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "ANNOTATION_TO_MODEL_LEADER_LINE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let elem_refs = read_entity_ref_list(attrs, 1, entity_id, "geometric_elements")?;
        let mut geometric_elements = Vec::with_capacity(elem_refs.len());
        for r in elem_refs {
            if let Some(id) = ctx.id_cache.get::<crate::ir::id::ApllPointId>(r) {
                geometric_elements.push(id);
            }
        }
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .annotation_placeholder_leader_lines
            .push(
                AnnotationPlaceholderLeaderLine::AnnotationToModelLeaderLine(
                    AnnotationToModelLeaderLine {
                        name,
                        geometric_elements,
                    },
                ),
            );
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        leader: AnnotationPlaceholderLeaderLine,
    ) -> Result<u64, WriteError> {
        match leader {
            AnnotationPlaceholderLeaderLine::AnnotationToModelLeaderLine(data) => {
                let elem_refs = data
                    .geometric_elements
                    .iter()
                    .map(|id| Attribute::EntityRef(buf.step_id(id)))
                    .collect();
                Ok(buf.push_simple(
                    "ANNOTATION_TO_MODEL_LEADER_LINE",
                    vec![Attribute::String(data.name), Attribute::List(elem_refs)],
                ))
            }
            AnnotationPlaceholderLeaderLine::AuxiliaryLeaderLine(data) => {
                let elem_refs = data
                    .geometric_elements
                    .iter()
                    .map(|id| Attribute::EntityRef(buf.step_id(id)))
                    .collect();
                // `controlling_leader_line` points at another member of the same
                // arena. Topo order processes it first (lower arena index), so
                // its step id is already in the partially-built cache; `.get`
                // keeps any ordering violation a visible dangling 0, not a panic.
                let controlling_step = buf.step_id(data.controlling_leader_line);
                Ok(buf.push_simple(
                    "AUXILIARY_LEADER_LINE",
                    vec![
                        Attribute::String(data.name),
                        Attribute::List(elem_refs),
                        Attribute::EntityRef(controlling_step),
                    ],
                ))
            }
        }
    }
}

pub(crate) struct AuxiliaryLeaderLineHandler;

/// `AUXILIARY_LEADER_LINE(name, geometric_elements, controlling_leader_line)`
/// — an `annotation_placeholder_leader_line` subtype following a
/// `controlling_leader_line` (another member of the same arena). Shares the
/// arena (and `annotation_placeholder_leader_line_id_map`) with
/// [`AnnotationToModelLeaderLineHandler`]; emission is handled by that
/// handler's `write`.
#[step_entity(name = "AUXILIARY_LEADER_LINE")]
impl SimpleEntityHandler for AuxiliaryLeaderLineHandler {
    type WriteInput = AnnotationPlaceholderLeaderLine;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "AUXILIARY_LEADER_LINE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let elem_refs = read_entity_ref_list(attrs, 1, entity_id, "geometric_elements")?;
        let mut geometric_elements = Vec::with_capacity(elem_refs.len());
        for r in elem_refs {
            if let Some(id) = ctx.id_cache.get::<crate::ir::id::ApllPointId>(r) {
                geometric_elements.push(id);
            }
        }
        let controlling_ref = read_entity_ref(attrs, 2, entity_id, "controlling_leader_line")?;
        let Some(controlling_leader_line) =
            ctx.id_cache
                .get::<crate::ir::id::AnnotationPlaceholderLeaderLineId>(controlling_ref)
        else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "AUXILIARY_LEADER_LINE.controlling_leader_line #{controlling_ref} did not \
                     resolve to a known leader line"
                ),
            });
            return Ok(());
        };
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .annotation_placeholder_leader_lines
            .push(AnnotationPlaceholderLeaderLine::AuxiliaryLeaderLine(
                AuxiliaryLeaderLineData {
                    name,
                    geometric_elements,
                    controlling_leader_line,
                },
            ));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        _buf: &mut WriteBuffer,
        _leader: AnnotationPlaceholderLeaderLine,
    ) -> Result<u64, WriteError> {
        unreachable!(
            "AUXILIARY_LEADER_LINE is emitted via AnnotationToModelLeaderLineHandler::write"
        )
    }
}

pub(crate) struct AnnotationPlaceholderOccurrenceHandler;

/// `ANNOTATION_PLACEHOLDER_OCCURRENCE(name, styles, item, role, line_spacing)`
/// — an `annotation_occurrence` subtype reserving a placeholder for a PMI
/// annotation. Mirrors [`DraughtingAnnotationOccurrenceHandler`] for the shared
/// `(name, styles, item)` body, plus the `role` enum token and the
/// `line_spacing` measure. `item` resolves through `resolve_representation_item_ref`
/// (a `GEOMETRIC_SET`); unresolved drops the occurrence.
#[step_entity(name = "ANNOTATION_PLACEHOLDER_OCCURRENCE")]
impl SimpleEntityHandler for AnnotationPlaceholderOccurrenceHandler {
    type WriteInput = AnnotationPlaceholderOccurrence;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 5, entity_id, "ANNOTATION_PLACEHOLDER_OCCURRENCE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;
        let role = read_enum(attrs, 3, entity_id, "role")?.to_owned();
        let line_spacing = read_real(attrs, 4, entity_id, "line_spacing")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(psa_id) = ctx
                .id_cache
                .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
            {
                styles.push(psa_id);
            }
        }
        let Some(item) = resolve_representation_item_ref(ctx, item_ref) else {
            return Ok(());
        };

        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .annotation_occurrences
            .push(AnnotationOccurrence::AnnotationPlaceholderOccurrence(
                AnnotationPlaceholderOccurrence {
                    name,
                    styles,
                    item,
                    role,
                    line_spacing,
                },
            ));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        apo: AnnotationPlaceholderOccurrence,
    ) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(apo.item)?;
        let mut style_refs = Vec::with_capacity(apo.styles.len());
        for psa_id in apo.styles {
            style_refs.push(Attribute::EntityRef(buf.step_id(psa_id)));
        }
        Ok(buf.push_simple(
            "ANNOTATION_PLACEHOLDER_OCCURRENCE",
            vec![
                Attribute::String(apo.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(item_id),
                Attribute::Enum(apo.role),
                Attribute::Real(apo.line_spacing),
            ],
        ))
    }
}

pub(crate) struct AnnotationPlaceholderOccurrenceWithLeaderLineHandler;

/// `ANNOTATION_PLACEHOLDER_OCCURRENCE_WITH_LEADER_LINE(name, styles, item, role,
/// line_spacing, leader_line)` — base APO + a `leader_line` SET. Mirrors
/// [`AnnotationPlaceholderOccurrenceHandler`] plus the 6th attr (leader-line
/// refs → `annotation_placeholder_leader_line_id_map`). Registers in
/// `annotation_occurrence_id_map` so a `_WITH_PLACEHOLDER` DMIA resolves it.
#[step_entity(name = "ANNOTATION_PLACEHOLDER_OCCURRENCE_WITH_LEADER_LINE")]
impl SimpleEntityHandler for AnnotationPlaceholderOccurrenceWithLeaderLineHandler {
    type WriteInput = AnnotationPlaceholderOccurrenceWithLeaderLine;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            6,
            entity_id,
            "ANNOTATION_PLACEHOLDER_OCCURRENCE_WITH_LEADER_LINE",
        )?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;
        let role = read_enum(attrs, 3, entity_id, "role")?.to_owned();
        let line_spacing = read_real(attrs, 4, entity_id, "line_spacing")?;
        let leader_refs = read_entity_ref_list(attrs, 5, entity_id, "leader_line")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(psa_id) = ctx
                .id_cache
                .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
            {
                styles.push(psa_id);
            }
        }
        let Some(item) = resolve_representation_item_ref(ctx, item_ref) else {
            return Ok(());
        };
        let mut leader_line = Vec::with_capacity(leader_refs.len());
        for r in leader_refs {
            if let Some(id) = ctx
                .id_cache
                .get::<crate::ir::id::AnnotationPlaceholderLeaderLineId>(r)
            {
                leader_line.push(id);
            }
        }

        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .annotation_occurrences
            .push(
                AnnotationOccurrence::AnnotationPlaceholderOccurrenceWithLeaderLine(
                    AnnotationPlaceholderOccurrenceWithLeaderLine {
                        name,
                        styles,
                        item,
                        role,
                        line_spacing,
                        leader_line,
                    },
                ),
            );
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        apo: AnnotationPlaceholderOccurrenceWithLeaderLine,
    ) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(apo.item)?;
        let mut style_refs = Vec::with_capacity(apo.styles.len());
        for psa_id in apo.styles {
            style_refs.push(Attribute::EntityRef(buf.step_id(psa_id)));
        }
        let leader_refs = apo
            .leader_line
            .iter()
            .map(|id| Attribute::EntityRef(buf.step_id(id)))
            .collect();
        Ok(buf.push_simple(
            "ANNOTATION_PLACEHOLDER_OCCURRENCE_WITH_LEADER_LINE",
            vec![
                Attribute::String(apo.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(item_id),
                Attribute::Enum(apo.role),
                Attribute::Real(apo.line_spacing),
                Attribute::List(leader_refs),
            ],
        ))
    }
}

pub(crate) struct AnnotationOccurrenceHandler;

/// `ANNOTATION_OCCURRENCE(name, styles, item)` — the plain `annotation_occurrence`
/// supertype, instantiated directly in some PMI corpora (e.g. as a
/// `DRAUGHTING_MODEL_ITEM_ASSOCIATION.identified_item` or a `DRAUGHTING_CALLOUT`
/// content). Same shape/handling as `DRAUGHTING_ANNOTATION_OCCURRENCE`.
#[step_entity(name = "ANNOTATION_OCCURRENCE")]
impl SimpleEntityHandler for AnnotationOccurrenceHandler {
    type WriteInput = PlainAnnotationOccurrence;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_annotation_occurrence(entity_id, attrs)?;
        crate::early::lower::lower_annotation_occurrence(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ao: PlainAnnotationOccurrence) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(ao.item)?;
        let mut style_refs = Vec::with_capacity(ao.styles.len());
        for psa_id in ao.styles {
            style_refs.push(buf.step_id(psa_id));
        }
        let early = crate::early::lift::lift_annotation_occurrence(ao.name, style_refs, item_id);
        Ok(crate::early::serialize::serialize_annotation_occurrence(
            buf, &early,
        ))
    }
}

pub(crate) struct LeaderCurveHandler;

/// Styled `LEADER_CURVE` — read only as the AND-combined complex
/// `(ANNOTATION_CURVE_OCCURRENCE ANNOTATION_OCCURRENCE
/// DRAUGHTING_ANNOTATION_OCCURRENCE GEOMETRIC_REPRESENTATION_ITEM LEADER_CURVE
/// REPRESENTATION_ITEM STYLED_ITEM)`, the only form in the corpus (the simple
/// single-name entity has count 0). `name` is on the `REPRESENTATION_ITEM`
/// part, `styles` + `item` on `STYLED_ITEM`; `item` narrows to a `Curve` via
/// `ctx.curve_map` (unresolved drops the occurrence, symmetric on re-read).
/// The arena id is recorded in `ctx.annotation_curve_occurrence_id_map` so the
/// `TERMINATOR_SYMBOL` / `LEADER_TERMINATOR` handlers can resolve their
/// `annotated_curve` back-reference.
#[step_entity_complex(
    name = "LEADER_CURVE",
    cases = [[
        "ANNOTATION_CURVE_OCCURRENCE",
        "ANNOTATION_OCCURRENCE",
        "DRAUGHTING_ANNOTATION_OCCURRENCE",
        "GEOMETRIC_REPRESENTATION_ITEM",
        "LEADER_CURVE",
        "REPRESENTATION_ITEM",
        "STYLED_ITEM",
    ]]
)]
impl ComplexEntityHandler for LeaderCurveHandler {
    type WriteInput = LeaderCurve;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_leader_curve(entity_id, parts)?;
        crate::early::lower::lower_leader_curve(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, lc: LeaderCurve) -> Result<u64, WriteError> {
        let curve_step = buf.emit_curve(lc.item)?;
        let mut style_refs = Vec::with_capacity(lc.styles.len());
        for psa_id in lc.styles {
            style_refs.push(buf.step_id(psa_id));
        }
        let early = crate::early::lift::lift_leader_curve(lc.name, style_refs, curve_step);
        Ok(crate::early::serialize::serialize_leader_curve(buf, &early))
    }
}

pub(crate) struct AnnotationCurveOccurrenceHandler;

/// Plain `ANNOTATION_CURVE_OCCURRENCE(name, styles, item)` — the
/// instantiable supertype (not `LEADER_CURVE`). `item` is the
/// `curve_or_curve_set` SELECT, resolved through `resolve_representation_item_ref`
/// (a plain `CURVE` or e.g. `GEOMETRIC_CURVE_SET`); unresolved item drops the
/// occurrence, symmetric on re-read. Shares the `annotation_curve_occurrence`
/// arena + `annotation_curve_occurrence_id_map` with `LEADER_CURVE` so
/// callout contents and `resolve_representation_item_ref` reach it.
#[step_entity(name = "ANNOTATION_CURVE_OCCURRENCE")]
impl SimpleEntityHandler for AnnotationCurveOccurrenceHandler {
    type WriteInput = PlainAnnotationCurveOccurrence;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_annotation_curve_occurrence(entity_id, attrs)?;
        crate::early::lower::lower_annotation_curve_occurrence(ctx, entity_id, &early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        aco: PlainAnnotationCurveOccurrence,
    ) -> Result<u64, WriteError> {
        let item_step = buf.emit_representation_item_ref(aco.item)?;
        let mut style_refs = Vec::with_capacity(aco.styles.len());
        for psa_id in aco.styles {
            style_refs.push(buf.step_id(psa_id));
        }
        let early =
            crate::early::lift::lift_annotation_curve_occurrence(aco.name, style_refs, item_step);
        Ok(crate::early::serialize::serialize_annotation_curve_occurrence(buf, &early))
    }
}

pub(crate) struct TerminatorSymbolHandler;

/// `TERMINATOR_SYMBOL(name, styles, item, annotated_curve)` — an
/// `annotation_symbol_occurrence` subtype with an `annotated_curve`
/// back-reference into the `annotation_curve_occurrence` arena.
/// Unresolved `item` (via `resolve_representation_item_ref`) or
/// `annotated_curve` (via `annotation_curve_occurrence_id_map`) drops
/// the occurrence, symmetric on re-read.
#[step_entity(name = "TERMINATOR_SYMBOL")]
impl SimpleEntityHandler for TerminatorSymbolHandler {
    type WriteInput = TerminatorSymbol;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "TERMINATOR_SYMBOL")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;
        let ac_ref = read_entity_ref(attrs, 3, entity_id, "annotated_curve")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(psa_id) = ctx
                .id_cache
                .get::<crate::ir::id::PresentationStyleAssignmentId>(r)
            {
                styles.push(psa_id);
            }
        }
        let Some(item) = resolve_representation_item_ref(ctx, item_ref) else {
            return Ok(());
        };
        let Some(annotated_curve) = ctx
            .id_cache
            .get::<crate::ir::id::AnnotationCurveOccurrenceId>(ac_ref)
        else {
            return Ok(());
        };

        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .annotation_occurrences
            .push(AnnotationOccurrence::TerminatorSymbol(TerminatorSymbol {
                name,
                styles,
                item,
                annotated_curve,
            }));
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ts: TerminatorSymbol) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(ts.item)?;
        let ac_step = buf.step_id(ts.annotated_curve);
        let mut style_refs = Vec::with_capacity(ts.styles.len());
        for psa_id in ts.styles {
            style_refs.push(Attribute::EntityRef(buf.step_id(psa_id)));
        }
        Ok(buf.push_simple(
            "TERMINATOR_SYMBOL",
            vec![
                Attribute::String(ts.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(item_id),
                Attribute::EntityRef(ac_step),
            ],
        ))
    }
}

pub(crate) struct LeaderTerminatorHandler;

/// Styled `LEADER_TERMINATOR` — read only as the AND-combined complex
/// `(ANNOTATION_OCCURRENCE ANNOTATION_SYMBOL_OCCURRENCE DRAUGHTING_ANNOTATION_OCCURRENCE
/// GEOMETRIC_REPRESENTATION_ITEM LEADER_TERMINATOR REPRESENTATION_ITEM STYLED_ITEM
/// TERMINATOR_SYMBOL)`, the only form in the corpus (the simple single-name entity
/// has count 0). `name` on `REPRESENTATION_ITEM`, `styles` + `item` on `STYLED_ITEM`
/// (`item` = `DEFINED_SYMBOL`, resolved via `resolve_representation_item_ref`),
/// `annotated_curve` on `TERMINATOR_SYMBOL` (a `LEADER_CURVE`). Unresolved `item`
/// or `annotated_curve` drops the occurrence, symmetric on re-read.
#[step_entity_complex(
    name = "LEADER_TERMINATOR",
    cases = [[
        "ANNOTATION_OCCURRENCE",
        "ANNOTATION_SYMBOL_OCCURRENCE",
        "DRAUGHTING_ANNOTATION_OCCURRENCE",
        "GEOMETRIC_REPRESENTATION_ITEM",
        "LEADER_TERMINATOR",
        "REPRESENTATION_ITEM",
        "STYLED_ITEM",
        "TERMINATOR_SYMBOL",
    ]]
)]
impl ComplexEntityHandler for LeaderTerminatorHandler {
    type WriteInput = LeaderTerminator;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_leader_terminator(entity_id, parts)?;
        crate::early::lower::lower_leader_terminator(ctx, entity_id, &early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, lt: LeaderTerminator) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(lt.item)?;
        let ac_step = buf.step_id(lt.annotated_curve);
        let mut style_refs = Vec::with_capacity(lt.styles.len());
        for psa_id in lt.styles {
            style_refs.push(buf.step_id(psa_id));
        }
        let early =
            crate::early::lift::lift_leader_terminator(lt.name, style_refs, item_id, ac_step);
        Ok(crate::early::serialize::serialize_leader_terminator(
            buf, &early,
        ))
    }
}

/// Read `contents` SET — each ref resolves either to an
/// `annotation_curve_occurrence` (`acoc_id_map`) or to an
/// `annotation_occurrence` enum entry (`ao_id_map`). Unresolved refs are
/// silently dropped (per-element drop, the occurrence itself is kept).
pub(crate) fn read_draughting_callout_contents(
    ctx: &ReaderContext,
    content_refs: &[u64],
) -> Vec<DraughtingCalloutElement> {
    let mut contents = Vec::with_capacity(content_refs.len());
    for r in content_refs {
        // Members + probe order are generated from the enum by `StepSelect`.
        // An unmodelled select member (e.g. annotation_fill_area_occurrence)
        // resolves to `None` and the element is dropped.
        if let Some(elem) = DraughtingCalloutElement::resolve_select(ctx, *r) {
            contents.push(elem);
        }
    }
    contents
}

pub(crate) struct DraughtingCalloutHandler;

/// `DRAUGHTING_CALLOUT(name, contents)` — base variant. The supertype is
/// not abstract in EXPRESS, and fixtures contain many direct
/// instances. Read into `DraughtingCallout::Plain`.
#[step_entity(name = "DRAUGHTING_CALLOUT")]
impl SimpleEntityHandler for DraughtingCalloutHandler {
    type WriteInput = DraughtingCalloutData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_draughting_callout(entity_id, attrs)?;
        crate::early::lower::lower_draughting_callout(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: DraughtingCalloutData) -> Result<u64, WriteError> {
        let contents: Vec<u64> = data
            .contents
            .iter()
            .map(|elem| elem.emit_select(buf))
            .collect();
        let early = crate::early::lift::lift_draughting_callout(data.name, contents);
        Ok(crate::early::serialize::serialize_draughting_callout(
            buf, &early,
        ))
    }
}

pub(crate) struct LeaderDirectedCalloutHandler;

/// `LEADER_DIRECTED_CALLOUT(name, contents)` — same shape as the base
/// supertype. EXPRESS WHERE narrows `contents` to include a
/// `LEADER_CURVE`; the IR carries the same shape without enforcement.
#[step_entity(name = "LEADER_DIRECTED_CALLOUT")]
impl SimpleEntityHandler for LeaderDirectedCalloutHandler {
    type WriteInput = DraughtingCalloutData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_leader_directed_callout(entity_id, attrs)?;
        crate::early::lower::lower_leader_directed_callout(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: DraughtingCalloutData) -> Result<u64, WriteError> {
        let contents: Vec<u64> = data
            .contents
            .iter()
            .map(|elem| elem.emit_select(buf))
            .collect();
        let early = crate::early::lift::lift_leader_directed_callout(data.name, contents);
        Ok(crate::early::serialize::serialize_leader_directed_callout(
            buf, &early,
        ))
    }
}

pub(crate) struct DraughtingCalloutRelationshipHandler;

/// `DRAUGHTING_CALLOUT_RELATIONSHIP(name, description, relating, related)`
/// — pairs two `draughting_callout` instances. Either ref unresolved drops
/// the relationship.
#[step_entity(name = "DRAUGHTING_CALLOUT_RELATIONSHIP")]
impl SimpleEntityHandler for DraughtingCalloutRelationshipHandler {
    type WriteInput = DraughtingCalloutRelationship;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "DRAUGHTING_CALLOUT_RELATIONSHIP")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let relating_ref = read_entity_ref(attrs, 2, entity_id, "relating_draughting_callout")?;
        let related_ref = read_entity_ref(attrs, 3, entity_id, "related_draughting_callout")?;
        let Some(relating) = ctx
            .id_cache
            .get::<crate::ir::id::DraughtingCalloutId>(relating_ref)
        else {
            return Ok(());
        };
        let Some(related) = ctx
            .id_cache
            .get::<crate::ir::id::DraughtingCalloutId>(related_ref)
        else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .draughting_callout_relationships
            .push(DraughtingCalloutRelationship {
                name,
                description,
                relating,
                related,
            });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, rel: DraughtingCalloutRelationship) -> Result<u64, WriteError> {
        let relating = buf.step_id(rel.relating);
        let related = buf.step_id(rel.related);
        Ok(buf.push_simple(
            "DRAUGHTING_CALLOUT_RELATIONSHIP",
            vec![
                Attribute::String(rel.name),
                Attribute::String(rel.description),
                Attribute::EntityRef(relating),
                Attribute::EntityRef(related),
            ],
        ))
    }
}

/// Resolve an `annotation_occurrence` reference to step-io's two annotation
/// occurrence arenas: the [`AnnotationOccurrence`] enum
/// (`annotation_occurrence_id_map`) or the separate
/// `annotation_curve_occurrence` arena (`annotation_curve_occurrence_id_map`).
/// Returns `None` for an unmodelled member (e.g.
/// `annotation_fill_area_occurrence`).
fn resolve_annotation_occurrence_ref(
    ctx: &ReaderContext,
    entity_ref: u64,
) -> Option<AnnotationOccurrenceRef> {
    // Members + probe order are generated from the enum by `StepSelect`.
    AnnotationOccurrenceRef::resolve_select(ctx, entity_ref)
}

pub(crate) struct AnnotationOccurrenceAssociativityHandler;

/// `ANNOTATION_OCCURRENCE_ASSOCIATIVITY(name, description, relating, related)`
/// — pairs two `annotation_occurrence` instances. Either ref that resolves to
/// none of the modelled annotation occurrence arenas drops the associativity.
#[step_entity(name = "ANNOTATION_OCCURRENCE_ASSOCIATIVITY")]
impl SimpleEntityHandler for AnnotationOccurrenceAssociativityHandler {
    type WriteInput = AnnotationOccurrenceAssociativity;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "ANNOTATION_OCCURRENCE_ASSOCIATIVITY")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let relating_ref = read_entity_ref(attrs, 2, entity_id, "relating_annotation_occurrence")?;
        let related_ref = read_entity_ref(attrs, 3, entity_id, "related_annotation_occurrence")?;
        let Some(relating) = resolve_annotation_occurrence_ref(ctx, relating_ref) else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "ANNOTATION_OCCURRENCE_ASSOCIATIVITY relating #{relating_ref} \
                     resolves to no modelled annotation occurrence — skipping"
                ),
            });
            return Ok(());
        };
        let Some(related) = resolve_annotation_occurrence_ref(ctx, related_ref) else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "ANNOTATION_OCCURRENCE_ASSOCIATIVITY related #{related_ref} \
                     resolves to no modelled annotation occurrence — skipping"
                ),
            });
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .annotation_occurrence_associativities
            .push(AnnotationOccurrenceAssociativity {
                name,
                description,
                relating,
                related,
            });
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        aoa: AnnotationOccurrenceAssociativity,
    ) -> Result<u64, WriteError> {
        let relating = emit_annotation_occurrence_ref(buf, aoa.relating);
        let related = emit_annotation_occurrence_ref(buf, aoa.related);
        Ok(buf.push_simple(
            "ANNOTATION_OCCURRENCE_ASSOCIATIVITY",
            vec![
                Attribute::String(aoa.name),
                Attribute::String(aoa.description),
                Attribute::EntityRef(relating),
                Attribute::EntityRef(related),
            ],
        ))
    }
}

/// Emit an [`AnnotationOccurrenceRef`] as the step id of the matching
/// occurrence, via the writer's two annotation occurrence step-id caches
/// (`ao_step_ids` / `acoc_step_ids`).
fn emit_annotation_occurrence_ref(buf: &WriteBuffer, r: AnnotationOccurrenceRef) -> u64 {
    r.emit_select(buf)
}

pub(crate) struct MeasureQualificationHandler;

/// `MEASURE_QUALIFICATION(name, description, qualified_measure, qualifiers)`
/// — `qualified_measure` resolves via `mwu_id_map`; `qualifiers` SET
/// members resolve through `type_qualifier_id_map` /
/// `value_format_type_qualifier_id_map`. The other two `value_qualifier`
/// SELECT members (`precision_qualifier` / `uncertainty_qualifier`)
/// have corpus 0 and are silently dropped (`ApprovalItem` precedent).
#[step_entity(name = "MEASURE_QUALIFICATION")]
impl SimpleEntityHandler for MeasureQualificationHandler {
    type WriteInput = MeasureQualification;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_measure_qualification(entity_id, attrs)?;
        crate::early::lower::lower_measure_qualification(ctx, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, mq: MeasureQualification) -> Result<u64, WriteError> {
        let qm_step = buf.step_id(mq.qualified_measure);
        let qualifiers: Vec<u64> = mq
            .qualifiers
            .into_iter()
            .map(|q| q.emit_select(buf))
            .collect();
        let early = crate::early::lift::lift_measure_qualification(
            mq.name,
            mq.description,
            qm_step,
            qualifiers,
        );
        Ok(crate::early::serialize::serialize_measure_qualification(
            buf, &early,
        ))
    }
}

pub(crate) struct ProjectedZoneDefinitionHandler;

/// `PROJECTED_ZONE_DEFINITION(zone, boundaries, projection_end, projected_length)`
/// — `zone` resolves via `tolerance_zone_id_map`, `boundaries` /
/// `projection_end` via `resolve_shape_aspect_ref`, `projected_length` via
/// `mwu_id_map`. Required refs (`zone` / `projection_end` / `projected_length`)
/// unresolved drop the occurrence; individual boundary refs skip
/// silently.
#[step_entity(name = "PROJECTED_ZONE_DEFINITION")]
impl SimpleEntityHandler for ProjectedZoneDefinitionHandler {
    type WriteInput = ProjectedZoneDefinition;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "PROJECTED_ZONE_DEFINITION")?;
        let zone_ref = read_entity_ref(attrs, 0, entity_id, "zone")?;
        let boundary_refs = read_entity_ref_list(attrs, 1, entity_id, "boundaries")?;
        let projection_end_ref = read_entity_ref(attrs, 2, entity_id, "projection_end")?;
        let projected_length_ref = read_entity_ref(attrs, 3, entity_id, "projected_length")?;
        let Some(zone) = ctx.id_cache.get::<crate::ir::id::ToleranceZoneId>(zone_ref) else {
            return Ok(());
        };
        let Some(projection_end) = resolve_shape_aspect_ref(ctx, projection_end_ref) else {
            return Ok(());
        };
        // projected_length is a measure_with_unit but exporters also emit the
        // complex MEASURE_REPRESENTATION_ITEM form (in repr_item_id_map, not
        // mwu_id_map) — resolve through both paths like a tolerance magnitude.
        let Some(projected_length) = resolve_tolerance_magnitude(ctx, projected_length_ref) else {
            return Ok(());
        };
        let mut boundaries = Vec::with_capacity(boundary_refs.len());
        for r in boundary_refs {
            if let Some(sar) = resolve_shape_aspect_ref(ctx, r) {
                boundaries.push(sar);
            }
        }
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .tolerance_zone_definitions
            .push(ProjectedZoneDefinition {
                zone,
                boundaries,
                projection_end,
                projected_length,
            });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, pzd: ProjectedZoneDefinition) -> Result<u64, WriteError> {
        let zone_step = buf.step_id(pzd.zone);
        let projection_end_step = buf.emit_shape_aspect_ref(pzd.projection_end);
        let projected_length_step = emit_tolerance_magnitude(buf, &pzd.projected_length);
        let mut boundary_refs = Vec::with_capacity(pzd.boundaries.len());
        for sar in pzd.boundaries {
            boundary_refs.push(Attribute::EntityRef(buf.emit_shape_aspect_ref(sar)));
        }
        Ok(buf.push_simple(
            "PROJECTED_ZONE_DEFINITION",
            vec![
                Attribute::EntityRef(zone_step),
                Attribute::List(boundary_refs),
                Attribute::EntityRef(projection_end_step),
                Attribute::EntityRef(projected_length_step),
            ],
        ))
    }
}

pub(crate) struct GeometricToleranceRelationshipHandler;

/// `GEOMETRIC_TOLERANCE_RELATIONSHIP(name, description, relating, related)`
/// — pairs two `geometric_tolerance` entries. Each ref resolves via
/// `resolve_geometric_tolerance_ref` (`Plain` vs `WithDatumReference` branch).
/// Either side unresolved drops the relationship, symmetric on re-read.
#[step_entity(name = "GEOMETRIC_TOLERANCE_RELATIONSHIP")]
impl SimpleEntityHandler for GeometricToleranceRelationshipHandler {
    type WriteInput = GeometricToleranceRelationship;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_geometric_tolerance_relationship(entity_id, attrs)?;
        crate::early::lower::lower_geometric_tolerance_relationship(ctx, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        rel: GeometricToleranceRelationship,
    ) -> Result<u64, WriteError> {
        let relating_step = match rel.relating {
            GeometricToleranceRef::Plain(id) => buf.step_id(id),
            GeometricToleranceRef::WithDatumReference(id) => buf.step_id(id),
        };
        let related_step = match rel.related {
            GeometricToleranceRef::Plain(id) => buf.step_id(id),
            GeometricToleranceRef::WithDatumReference(id) => buf.step_id(id),
        };
        let early = crate::early::lift::lift_geometric_tolerance_relationship(
            rel.name,
            rel.description,
            relating_step,
            related_step,
        );
        Ok(crate::early::serialize::serialize_geometric_tolerance_relationship(buf, &early))
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
#[step_entity(name = "DATUM")]
impl SimpleEntityHandler for DatumHandler {
    type WriteInput = DatumWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_datum(entity_id, attrs)?;
        crate::early::lower::lower_datum(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, input: DatumWriteInput) -> Result<u64, WriteError> {
        let early = crate::early::lift::lift_datum(
            input.name,
            input.description,
            input.pds_step_id,
            input.product_definitional,
            input.identification,
        );
        Ok(crate::early::serialize::serialize_datum(buf, &early))
    }
}

pub(crate) struct DatumFeatureHandler;

pub(crate) struct DatumFeatureWriteInput {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) pds_step_id: u64,
    pub(crate) product_definitional: bool,
    /// `DATUM_FEATURE` or `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE`, resolved
    /// from the IR `DatumFeature` variant at the emit site.
    pub(crate) entity_name: &'static str,
}

/// `DATUM_FEATURE(name, description, of_shape, product_definitional)` — a
/// `shape_aspect` subtype naming the physical feature realising a datum.
/// Same 4-attr `shape_aspect` body and `of_shape → ProductId` resolution as
/// `SHAPE_ASPECT`; an unresolved `of_shape` drops the datum feature,
/// symmetric on re-read. Registered into `datum_feature_id_map` so a
/// `shape_aspect` ref (e.g. `geometric_tolerance.toleranced_shape_aspect`)
/// resolves onto it through `resolve_shape_aspect_ref`. Shares the arena
/// with the `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE` subtype through the
/// [`DatumFeature`](crate::ir::DatumFeature) variants.
#[step_entity(name = "DATUM_FEATURE")]
impl SimpleEntityHandler for DatumFeatureHandler {
    type WriteInput = DatumFeatureWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_datum_feature(entity_id, attrs)?;
        crate::early::lower::lower_datum_feature(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, input: DatumFeatureWriteInput) -> Result<u64, WriteError> {
        Ok(write_datum_feature(buf, input))
    }
}

pub(crate) struct DimensionalSizeWithDatumFeatureHandler;

/// `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE` — `datum_feature` arena's `in_enum`
/// subtype per the ir.toml blueprint. Multiple-inheritance entity (both
/// `datum_feature` and `dimensional_size`): the 6 flattened attrs are the
/// 4-attr `shape_aspect` body + `dimensional_size.applies_to` (EXPRESS WR1
/// `:=: SELF`) + `dimensional_size.name`. Registered in both
/// `datum_feature_id_map` (`shape_aspect` references) and resolvable as a
/// `dimensional_characteristic` (`resolve_dimensional_characteristic` probes
/// the arena). Emitted by `emit_datum_features`.
#[step_entity(name = "DIMENSIONAL_SIZE_WITH_DATUM_FEATURE")]
impl SimpleEntityHandler for DimensionalSizeWithDatumFeatureHandler {
    type WriteInput = DatumFeatureWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 6, entity_id, "DIMENSIONAL_SIZE_WITH_DATUM_FEATURE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let of_shape_ref = read_entity_ref(attrs, 2, entity_id, "of_shape")?;
        let product_definitional = read_bool(attrs, 3, entity_id, "product_definitional")?;
        // attrs[4] = applies_to (WR1: SELF), resolved after self-registration.
        let applies_to_ref = read_entity_ref(attrs, 4, entity_id, "applies_to")?;
        let size_name = read_string_or_unset(attrs, 5, entity_id, "size_name")?.to_owned();

        // of_shape → PRODUCT_DEFINITION_SHAPE → ProductId (typed one-probe).
        let Some(target) = ctx.product_of_pds(of_shape_ref) else {
            return Ok(());
        };

        // Push and register first so the WR1 self-reference resolves, then fill
        // applies_to (chicken-and-egg: the id only exists after the push).
        let df_id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .datum_features
            .push(DatumFeature::DimensionalSizeWithDatumFeature(
                DimensionalSizeWithDatumFeatureData {
                    base: crate::ir::DatumFeatureData {
                        name,
                        description,
                        target,
                        product_definitional,
                    },
                    applies_to: ShapeAspectRef::DatumFeature(
                        // placeholder, overwritten below once the id is known
                        crate::ir::DatumFeatureId(0),
                    ),
                    size_name,
                },
            ));
        ctx.id_cache.insert(entity_id, df_id);
        let applies_to = resolve_shape_aspect_ref(ctx, applies_to_ref)
            .unwrap_or(ShapeAspectRef::DatumFeature(df_id));
        if let DatumFeature::DimensionalSizeWithDatumFeature(d) =
            &mut ctx.pmi.get_or_insert_with(PmiPool::default).datum_features[df_id]
        {
            d.applies_to = applies_to;
        }
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _input: DatumFeatureWriteInput) -> Result<u64, WriteError> {
        unreachable!("DIMENSIONAL_SIZE_WITH_DATUM_FEATURE is emitted via emit_datum_features")
    }
}

/// Shared writer for the `datum_feature` family. The STEP entity name is
/// resolved from the IR variant at the emit site and carried on `input`.
fn write_datum_feature(buf: &mut WriteBuffer, input: DatumFeatureWriteInput) -> u64 {
    if input.entity_name == "DATUM_FEATURE" {
        let early = crate::early::lift::lift_datum_feature(
            input.name,
            input.description,
            input.pds_step_id,
            input.product_definitional,
        );
        return crate::early::serialize::serialize_datum_feature(buf, &early);
    }
    let bool_attr = if input.product_definitional { "T" } else { "F" };
    buf.push_simple(
        input.entity_name,
        vec![
            Attribute::String(input.name),
            Attribute::String(input.description),
            Attribute::EntityRef(input.pds_step_id),
            Attribute::Enum(bool_attr.into()),
        ],
    )
}

/// Emit a `DimensionalSize` under the STEP entity name its `kind` selects.
fn write_dimensional_size(buf: &mut WriteBuffer, ds: DimensionalSize) -> u64 {
    let applies_to = buf.emit_shape_aspect_ref(ds.applies_to);
    match ds.kind {
        DimensionalSizeKind::Plain => {
            let early = crate::early::lift::lift_dimensional_size(applies_to, ds.name);
            crate::early::serialize::serialize_dimensional_size(buf, &early)
        }
        DimensionalSizeKind::Angular(sel) => {
            let early = crate::early::lift::lift_angular_size(applies_to, ds.name, sel);
            crate::early::serialize::serialize_angular_size(buf, &early)
        }
    }
}

pub(crate) struct DimensionalSizeHandler;

#[step_entity(name = "DIMENSIONAL_SIZE")]
impl SimpleEntityHandler for DimensionalSizeHandler {
    type WriteInput = DimensionalSize;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_dimensional_size(entity_id, attrs)?;
        crate::early::lower::lower_dimensional_size(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ds: DimensionalSize) -> Result<u64, WriteError> {
        Ok(write_dimensional_size(buf, ds))
    }
}

pub(crate) struct AngularSizeHandler;

#[step_entity(name = "ANGULAR_SIZE")]
impl SimpleEntityHandler for AngularSizeHandler {
    type WriteInput = DimensionalSize;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_angular_size(entity_id, attrs)?;
        crate::early::lower::lower_angular_size(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ds: DimensionalSize) -> Result<u64, WriteError> {
        Ok(write_dimensional_size(buf, ds))
    }
}

/// Emit a `DimensionalLocation` under the STEP entity name its variant
/// selects, returning the STEP id. Shared by all three family handlers.
fn write_dimensional_location(buf: &mut WriteBuffer, dl: DimensionalLocation) -> u64 {
    match dl {
        DimensionalLocation::Plain(d) => {
            let relating = buf.emit_shape_aspect_ref(d.relating_shape_aspect);
            let related = buf.emit_shape_aspect_ref(d.related_shape_aspect);
            let early = crate::early::lift::lift_dimensional_location(d, relating, related);
            crate::early::serialize::serialize_dimensional_location(buf, &early)
        }
        DimensionalLocation::Directed(d) => {
            let relating = buf.emit_shape_aspect_ref(d.relating_shape_aspect);
            let related = buf.emit_shape_aspect_ref(d.related_shape_aspect);
            let early =
                crate::early::lift::lift_directed_dimensional_location(d, relating, related);
            crate::early::serialize::serialize_directed_dimensional_location(buf, &early)
        }
        DimensionalLocation::Angular(d) => {
            let relating = buf.emit_shape_aspect_ref(d.relating_shape_aspect);
            let related = buf.emit_shape_aspect_ref(d.related_shape_aspect);
            let early = crate::early::lift::lift_angular_location(d, relating, related);
            crate::early::serialize::serialize_angular_location(buf, &early)
        }
    }
}

pub(crate) struct DimensionalLocationHandler;

#[step_entity(name = "DIMENSIONAL_LOCATION")]
impl SimpleEntityHandler for DimensionalLocationHandler {
    type WriteInput = DimensionalLocation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_dimensional_location(entity_id, attrs)?;
        crate::early::lower::lower_dimensional_location(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dl: DimensionalLocation) -> Result<u64, WriteError> {
        Ok(write_dimensional_location(buf, dl))
    }
}

pub(crate) struct DirectedDimensionalLocationHandler;

#[step_entity(name = "DIRECTED_DIMENSIONAL_LOCATION")]
impl SimpleEntityHandler for DirectedDimensionalLocationHandler {
    type WriteInput = DimensionalLocation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_directed_dimensional_location(entity_id, attrs)?;
        crate::early::lower::lower_directed_dimensional_location(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dl: DimensionalLocation) -> Result<u64, WriteError> {
        Ok(write_dimensional_location(buf, dl))
    }
}

pub(crate) struct AngularLocationHandler;

#[step_entity(name = "ANGULAR_LOCATION")]
impl SimpleEntityHandler for AngularLocationHandler {
    type WriteInput = DimensionalLocation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_angular_location(entity_id, attrs)?;
        crate::early::lower::lower_angular_location(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, dl: DimensionalLocation) -> Result<u64, WriteError> {
        Ok(write_dimensional_location(buf, dl))
    }
}

/// Resolve a `geometric_tolerance.magnitude` ref (`ref_measure_with_unit`).
/// A plain `*_MEASURE_WITH_UNIT` resolves through `mwu_id_map` (units pool); a
/// `MEASURE_REPRESENTATION_ITEM` (simple or complex) through the
/// `representation_item` arena (`repr_item_id_map`). `None` when the ref
/// resolves to neither.
pub(crate) fn resolve_tolerance_magnitude(
    ctx: &ReaderContext,
    item_ref: u64,
) -> Option<ToleranceMagnitude> {
    if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::MeasureWithUnitId>(item_ref)
    {
        return Some(ToleranceMagnitude::MeasureWithUnit(id));
    }
    // MEASURE_REPRESENTATION_ITEM lives in the representation_item arena.
    // Guard on the variant: repr_item_id_map also holds QRI / VRI.
    if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationItemId>(item_ref)
    {
        if matches!(
            ctx.representation_items[id],
            crate::ir::representation_item::RepresentationItem::MeasureRepresentationItem(_)
        ) {
            return Some(ToleranceMagnitude::RepresentationItem(id));
        }
    }
    None
}

/// Push a `GeometricTolerance` into the `pmi` pool and register its
/// `#N → GeometricToleranceId` so `TOLERANCE_ZONE.defining_tolerance` can
/// resolve a `ref_geometric_tolerance` onto it.
pub(crate) fn push_geometric_tolerance(
    ctx: &mut ReaderContext,
    entity_id: u64,
    gt: GeometricTolerance,
) {
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .geometric_tolerances
        .push(gt);
    ctx.id_cache.insert(entity_id, id);
}

/// Push a `GeometricToleranceWithDatumReference` into the `pmi` pool and
/// register its `#N → GeometricToleranceWithDatumReferenceId` — see
/// [`push_geometric_tolerance`].
fn push_gt_with_datum_reference(
    ctx: &mut ReaderContext,
    entity_id: u64,
    gt: GeometricToleranceWithDatumReference,
) {
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .geometric_tolerance_with_datum_references
        .push(gt);
    ctx.id_cache.insert(entity_id, id);
}

/// Resolve a `ref_geometric_tolerance` (`TOLERANCE_ZONE.defining_tolerance`)
/// to a [`GeometricToleranceRef`] — step-io splits geometric tolerances
/// across the form-tolerance and datum-referencing arenas.
pub(crate) fn resolve_geometric_tolerance_ref(
    ctx: &ReaderContext,
    item_ref: u64,
) -> Option<GeometricToleranceRef> {
    if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::GeometricToleranceId>(item_ref)
    {
        return Some(GeometricToleranceRef::Plain(id));
    }
    ctx.id_cache
        .get::<crate::ir::GeometricToleranceWithDatumReferenceId>(item_ref)
        .map(GeometricToleranceRef::WithDatumReference)
}

/// Resolve a `geometric_tolerance.toleranced_shape_aspect` ref — the
/// `geometric_tolerance_target` SELECT. Tries `shape_aspect` (the common case)
/// first, then `product_definition_shape` via `property_def_step_to_id`. The
/// PDS branch is gated on the arena variant so a tolerance targeting a non-PDS
/// `PROPERTY_DEFINITION` (which also lives in `property_def_step_to_id`) does
/// not mis-resolve. `None` when the ref is neither.
pub(crate) fn resolve_geometric_tolerance_target(
    ctx: &ReaderContext,
    item_ref: u64,
) -> Option<GeometricToleranceTarget> {
    if let Some(sa) = resolve_shape_aspect_ref(ctx, item_ref) {
        return Some(GeometricToleranceTarget::ShapeAspect(sa));
    }
    let pd_id = ctx
        .id_cache
        .get::<crate::ir::id::PropertyDefinitionId>(item_ref)?;
    let pool = ctx.properties.as_ref()?;
    if matches!(
        pool.property_definitions[pd_id],
        crate::ir::property::PropertyDefinition::ProductDefinitionShape(_)
    ) {
        return Some(GeometricToleranceTarget::ProductDefinitionShape(pd_id));
    }
    None
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
        GeometricTolerance::SurfaceProfile(d) => ("SURFACE_PROFILE_TOLERANCE", d),
    };
    // A `MeasureWithUnit` magnitude is already emitted by the units pass —
    // reference its cached step id. A `Measure` magnitude has no arena entry;
    // emit the (simple) MRI inline here.
    let magnitude = match data.magnitude {
        ToleranceMagnitude::MeasureWithUnit(id) => buf.step_id(id),
        ToleranceMagnitude::RepresentationItem(id) => buf.step_id(id),
    };
    let shape_aspect = buf.emit_geometric_tolerance_target(data.toleranced_shape_aspect);
    let has_unit_size = data.unit_size.is_some();
    let has_area_unit = data.defined_area_unit.is_some();
    let has_modifiers = !data.modifiers.is_empty();
    if !has_unit_size && !has_area_unit && !has_modifiers {
        // Simple 4-attr emit goes through the generated serialize (the
        // complex-MI branch below stays hand-built).
        return match entity_name {
            "FLATNESS_TOLERANCE" => crate::early::serialize::serialize_flatness_tolerance(
                buf,
                &crate::early::lift::lift_flatness_tolerance(
                    data.name,
                    data.description,
                    magnitude,
                    shape_aspect,
                ),
            ),
            "STRAIGHTNESS_TOLERANCE" => crate::early::serialize::serialize_straightness_tolerance(
                buf,
                &crate::early::lift::lift_straightness_tolerance(
                    data.name,
                    data.description,
                    magnitude,
                    shape_aspect,
                ),
            ),
            "ROUNDNESS_TOLERANCE" => crate::early::serialize::serialize_roundness_tolerance(
                buf,
                &crate::early::lift::lift_roundness_tolerance(
                    data.name,
                    data.description,
                    magnitude,
                    shape_aspect,
                ),
            ),
            "CYLINDRICITY_TOLERANCE" => crate::early::serialize::serialize_cylindricity_tolerance(
                buf,
                &crate::early::lift::lift_cylindricity_tolerance(
                    data.name,
                    data.description,
                    magnitude,
                    shape_aspect,
                ),
            ),
            _ => crate::early::serialize::serialize_surface_profile_tolerance(
                buf,
                &crate::early::lift::lift_surface_profile_tolerance(
                    data.name,
                    data.description,
                    magnitude,
                    shape_aspect,
                ),
            ),
        };
    }
    // Complex MI emit. Part order follows EXPRESS supertype order:
    // GT → [WDU] → [WDAU] → [WM] → LEAF.
    let mut parts: Vec<(String, Vec<Attribute>)> = Vec::with_capacity(5);
    parts.push((
        "GEOMETRIC_TOLERANCE".into(),
        vec![
            Attribute::String(data.name),
            Attribute::String(data.description),
            Attribute::EntityRef(magnitude),
            Attribute::EntityRef(shape_aspect),
        ],
    ));
    if let Some(unit_size) = data.unit_size {
        let unit_size_step = emit_tolerance_magnitude(buf, &unit_size);
        parts.push((
            "GEOMETRIC_TOLERANCE_WITH_DEFINED_UNIT".into(),
            vec![Attribute::EntityRef(unit_size_step)],
        ));
        if let Some(area_unit) = &data.defined_area_unit {
            parts.push((
                "GEOMETRIC_TOLERANCE_WITH_DEFINED_AREA_UNIT".into(),
                emit_defined_area_unit(buf, area_unit),
            ));
        }
    }
    if has_modifiers {
        parts.push((
            "GEOMETRIC_TOLERANCE_WITH_MODIFIERS".into(),
            vec![Attribute::List(emit_modifier_set(&data.modifiers))],
        ));
    }
    parts.push((entity_name.into(), vec![]));
    let n = buf.fresh();
    buf.entities.push(crate::writer::entity::WriterEntity {
        id: n,
        body: crate::writer::entity::WriterBody::Complex { parts },
    });
    n
}

/// Encode `DefinedAreaUnit` as the two-attr body of
/// `GEOMETRIC_TOLERANCE_WITH_DEFINED_AREA_UNIT`. `second_unit_size`
/// resolves through `emit_tolerance_magnitude` (units-pool or repr-item
/// arena; None → `$`).
fn emit_defined_area_unit(
    buf: &WriteBuffer,
    area_unit: &crate::ir::DefinedAreaUnit,
) -> Vec<Attribute> {
    use crate::ir::AreaUnitType;
    let area_token = match &area_unit.area_type {
        AreaUnitType::Circular => "CIRCULAR",
        AreaUnitType::Rectangular => "RECTANGULAR",
        AreaUnitType::Square => "SQUARE",
        AreaUnitType::Other(s) => s.as_str(),
    };
    let second = match &area_unit.second_unit_size {
        Some(m) => Attribute::EntityRef(emit_tolerance_magnitude(buf, m)),
        None => Attribute::Unset,
    };
    vec![Attribute::Enum(area_token.into()), second]
}

/// Encode a `GeometricToleranceModifier` Vec as the `Attribute::List` that
/// occupies attr[0] of `GEOMETRIC_TOLERANCE_WITH_MODIFIERS`. Mirrors the
/// reader's `read_optional_modifiers` token decoding so round-trip is
/// lossless (Other variants preserve the source token verbatim).
fn emit_modifier_set(modifiers: &[crate::ir::GeometricToleranceModifier]) -> Vec<Attribute> {
    use crate::ir::GeometricToleranceModifier;
    modifiers
        .iter()
        .map(|m| {
            let token = match m {
                GeometricToleranceModifier::MaximumMaterialRequirement => {
                    "MAXIMUM_MATERIAL_REQUIREMENT"
                }
                GeometricToleranceModifier::LeastMaterialRequirement => {
                    "LEAST_MATERIAL_REQUIREMENT"
                }
                GeometricToleranceModifier::ReciprocityRequirement => "RECIPROCITY_REQUIREMENT",
                GeometricToleranceModifier::Other(s) => s.as_str(),
            };
            Attribute::Enum(token.into())
        })
        .collect()
}

pub(crate) struct FlatnessToleranceHandler;

#[step_entity(name = "FLATNESS_TOLERANCE")]
impl SimpleEntityHandler for FlatnessToleranceHandler {
    type WriteInput = GeometricTolerance;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_flatness_tolerance(entity_id, attrs)?;
        crate::early::lower::lower_flatness_tolerance(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gt: GeometricTolerance) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance(buf, gt))
    }
}

pub(crate) struct SurfaceProfileToleranceSimpleHandler;

/// Plain (datum-free) `SURFACE_PROFILE_TOLERANCE(name, description, magnitude,
/// toleranced_shape_aspect)` — a standalone `geometric_tolerance` subtype
/// (4 corpus standalone instances). Mirrors [`FlatnessToleranceHandler`];
/// coexists with the complex datum-referencing form
/// [`SurfaceProfileToleranceHandler`] (simple vs complex dispatch). Recovering
/// it resolves `DRAUGHTING_MODEL_ITEM_ASSOCIATION` / `PROPERTY_DEFINITION`
/// references (their cascades) via `resolve_geometric_tolerance_ref`.
#[step_entity(name = "SURFACE_PROFILE_TOLERANCE")]
impl SimpleEntityHandler for SurfaceProfileToleranceSimpleHandler {
    type WriteInput = GeometricTolerance;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_surface_profile_tolerance(entity_id, attrs)?;
        crate::early::lower::lower_surface_profile_tolerance(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gt: GeometricTolerance) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance(buf, gt))
    }
}

pub(crate) struct StraightnessToleranceHandler;

#[step_entity(name = "STRAIGHTNESS_TOLERANCE")]
impl SimpleEntityHandler for StraightnessToleranceHandler {
    type WriteInput = GeometricTolerance;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_straightness_tolerance(entity_id, attrs)?;
        crate::early::lower::lower_straightness_tolerance(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gt: GeometricTolerance) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance(buf, gt))
    }
}

pub(crate) struct RoundnessToleranceHandler;

#[step_entity(name = "ROUNDNESS_TOLERANCE")]
impl SimpleEntityHandler for RoundnessToleranceHandler {
    type WriteInput = GeometricTolerance;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_roundness_tolerance(entity_id, attrs)?;
        crate::early::lower::lower_roundness_tolerance(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gt: GeometricTolerance) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance(buf, gt))
    }
}

pub(crate) struct CylindricityToleranceHandler;

#[step_entity(name = "CYLINDRICITY_TOLERANCE")]
impl SimpleEntityHandler for CylindricityToleranceHandler {
    type WriteInput = GeometricTolerance;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early = crate::early::bind::bind_cylindricity_tolerance(entity_id, attrs)?;
        crate::early::lower::lower_cylindricity_tolerance(ctx, entity_id, early);
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
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    graph: &EntityGraph,
    entity_name: &'static str,
) -> Result<Option<GeneralDatumReferenceData>, ConvertError> {
    check_count(attrs, 6, entity_id, entity_name)?;
    let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
    // NsCase::GeneralDatumReferenceOfShapeUnset of_shape is a mandatory
    // shape_aspect attribute (EXPRESS + UNIQUE constraint); some NIST exports
    // (ctc_05) emit `$`. Classify as non-standard input rather than an
    // AttributeType defect; the owning entity carries no resolvable product.
    if matches!(attrs.get(2), Some(Attribute::Unset | Attribute::Derived)) {
        ctx.ns_record(
            crate::reader::NsCase::GeneralDatumReferenceOfShapeUnset,
            entity_name.into(),
            "dropped (of_shape Unset — EXPRESS shape_aspect.of_shape required)",
        );
        return Ok(None);
    }
    let of_shape_ref = read_entity_ref(attrs, 2, entity_id, "of_shape")?;
    let product_definitional = read_bool(attrs, 3, entity_id, "product_definitional")?;
    // attr 5 (`modifiers`) — datum_reference_modifier set, not modelled.

    // of_shape → PRODUCT_DEFINITION_SHAPE → ProductId (typed one-probe).
    let Some(target) = ctx.product_of_pds(of_shape_ref) else {
        return Ok(None);
    };
    // base — `datum_or_common_datum` SELECT: a single `DATUM` ref, or a
    // `COMMON_DATUM_LIST` (a Typed list of `datum_reference_element`s, an "A-B"
    // composite datum). A base outside these forms drops the owner.
    let base = match attrs.get(4) {
        Some(Attribute::EntityRef(r)) => {
            let Some(datum_id) = ctx.id_cache.get::<crate::ir::DatumId>(*r) else {
                return Ok(None);
            };
            GeneralDatumBase::Datum(datum_id)
        }
        Some(Attribute::Typed { type_name, value }) if type_name == "COMMON_DATUM_LIST" => {
            let Attribute::List(items) = value.as_ref() else {
                return Ok(None);
            };
            let mut ids = Vec::with_capacity(items.len());
            for item in items {
                let Attribute::EntityRef(r) = item else {
                    return Ok(None);
                };
                let Some(gdr_id) = ctx.id_cache.get::<crate::ir::GeneralDatumReferenceId>(*r)
                else {
                    // The member dropped. Record the cascade only when it is an
                    // of_shape=$ sibling (NsCase::GeneralDatumReferenceOfShapeUnset);
                    // a member dropped for a different reason (e.g. unmodelled
                    // datum) stays a plain drop, not a normalization.
                    let member_of_shape_unset = matches!(
                        graph.get(*r),
                        Some(RawEntity::Simple { attributes, .. })
                            if matches!(
                                attributes.get(2),
                                Some(Attribute::Unset | Attribute::Derived)
                            )
                    );
                    if member_of_shape_unset {
                        ctx.ns_record(
                            crate::reader::NsCase::GeneralDatumReferenceOfShapeUnset,
                            entity_name.into(),
                            "dropped (common_datum_list member of_shape Unset — cascade)",
                        );
                    }
                    return Ok(None);
                };
                ids.push(gdr_id);
            }
            GeneralDatumBase::CommonDatumList(ids)
        }
        _ => return Ok(None),
    };

    Ok(Some(GeneralDatumReferenceData {
        name,
        description,
        target,
        product_definitional,
        base,
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
    let base_attr = match data.base {
        GeneralDatumBase::Datum(id) => Attribute::EntityRef(buf.step_id(id)),
        GeneralDatumBase::CommonDatumList(ids) => Attribute::Typed {
            type_name: "COMMON_DATUM_LIST".to_string(),
            value: Box::new(Attribute::List(
                ids.iter()
                    .map(|id| {
                        let step = buf.step_id(id);
                        // Invariant: list members (datum_reference_elements) are
                        // referenced-before-referrer, so emitted first in the
                        // arena-order loop — their step ids are non-zero here.
                        debug_assert_ne!(step, 0, "common datum element emitted after compartment");
                        Attribute::EntityRef(step)
                    })
                    .collect(),
            )),
        },
    };
    let bool_attr = if data.product_definitional { "T" } else { "F" };
    buf.push_simple(
        entity_name,
        vec![
            Attribute::String(data.name),
            Attribute::String(data.description),
            Attribute::EntityRef(pds_step_id),
            Attribute::Enum(bool_attr.into()),
            base_attr,
            // modifiers — not modelled, always emitted as `$`.
            Attribute::Unset,
        ],
    )
}

pub(crate) struct DatumReferenceCompartmentHandler;

#[step_entity(name = "DATUM_REFERENCE_COMPARTMENT")]
impl SimpleEntityHandler for DatumReferenceCompartmentHandler {
    type WriteInput = GeneralDatumReference;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_general_datum_reference_data(
            ctx,
            entity_id,
            attrs,
            graph,
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
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gdr: GeneralDatumReference) -> Result<u64, WriteError> {
        Ok(write_general_datum_reference(buf, gdr))
    }
}

pub(crate) struct DatumReferenceElementHandler;

#[step_entity(name = "DATUM_REFERENCE_ELEMENT")]
impl SimpleEntityHandler for DatumReferenceElementHandler {
    type WriteInput = GeneralDatumReference;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_general_datum_reference_data(
            ctx,
            entity_id,
            attrs,
            graph,
            "DATUM_REFERENCE_ELEMENT",
        )?
        else {
            return Ok(());
        };
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .general_datum_references
            .push(GeneralDatumReference::Element(data));
        ctx.id_cache.insert(entity_id, id);
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
#[allow(clippy::too_many_arguments)]
fn build_gt_with_datum_reference_data(
    ctx: &ReaderContext,
    name: String,
    description: String,
    magnitude_ref: u64,
    shape_aspect_ref: u64,
    datum_system_refs: &[u64],
    modifiers: Vec<crate::ir::GeometricToleranceModifier>,
    displacement: Option<ToleranceMagnitude>,
) -> Option<GeometricToleranceWithDatumReferenceData> {
    let magnitude = resolve_tolerance_magnitude(ctx, magnitude_ref)?;
    let toleranced_shape_aspect = resolve_geometric_tolerance_target(ctx, shape_aspect_ref)?;
    let mut datum_system = Vec::with_capacity(datum_system_refs.len());
    for &r in datum_system_refs {
        if let Some(id) = ctx.id_cache.get::<crate::ir::DatumSystemId>(r) {
            datum_system.push(id);
        }
    }
    Some(GeometricToleranceWithDatumReferenceData {
        name,
        description,
        magnitude,
        toleranced_shape_aspect,
        datum_system,
        modifiers,
        displacement,
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
        Vec::new(),
        None,
    ))
}

/// Read the multiple-inheritance complex form `(GEOMETRIC_TOLERANCE
/// GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE <leaf>)` — the encoding
/// `POSITION` / `SURFACE_PROFILE` / `LINE_PROFILE` tolerances take. `Ok(None)`
/// when a ref does not resolve. An optional
/// `GEOMETRIC_TOLERANCE_WITH_MODIFIERS` part populates the `modifiers` Vec
/// when present (phase gt-modifiers).
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
    let modifiers = read_optional_modifiers(parts, entity_id)?;
    let displacement = read_optional_displacement(ctx, parts, entity_id)?;
    Ok(build_gt_with_datum_reference_data(
        ctx,
        name,
        description,
        magnitude_ref,
        shape_aspect_ref,
        &datum_system_refs,
        modifiers,
        displacement,
    ))
}

/// Read the `GEOMETRIC_TOLERANCE_WITH_MODIFIERS.modifiers` set from a
/// complex MI's parts. Empty Vec when the part is absent (silent —
/// modifier presence is optional per the ir.toml blueprint /
/// EXPRESS ANDOR group 3). Unknown modifier tokens land in
/// `Other(raw)` so the round-trip preserves the source text verbatim.
fn read_optional_modifiers(
    parts: &[RawEntityPart],
    entity_id: u64,
) -> Result<Vec<crate::ir::GeometricToleranceModifier>, ConvertError> {
    use crate::ir::GeometricToleranceModifier;
    let Some(attrs) = find_part_attrs(parts, "GEOMETRIC_TOLERANCE_WITH_MODIFIERS") else {
        return Ok(Vec::new());
    };
    check_count(attrs, 1, entity_id, "GEOMETRIC_TOLERANCE_WITH_MODIFIERS")?;
    let Some(Attribute::List(raw)) = attrs.first() else {
        return Ok(Vec::new());
    };
    let mut modifiers = Vec::with_capacity(raw.len());
    for item in raw {
        let token = match item {
            Attribute::Enum(s) => s.as_str(),
            _ => continue,
        };
        modifiers.push(match token {
            "MAXIMUM_MATERIAL_REQUIREMENT" => {
                GeometricToleranceModifier::MaximumMaterialRequirement
            }
            "LEAST_MATERIAL_REQUIREMENT" => GeometricToleranceModifier::LeastMaterialRequirement,
            "RECIPROCITY_REQUIREMENT" => GeometricToleranceModifier::ReciprocityRequirement,
            other => GeometricToleranceModifier::Other(other.to_owned()),
        });
    }
    Ok(modifiers)
}

/// Read the `GEOMETRIC_TOLERANCE_WITH_DEFINED_UNIT.unit_size` part —
/// `ref_measure_with_unit`. Returns `None` when the part is absent or the
/// ref resolves to neither a units-pool `MEASURE_WITH_UNIT` nor a
/// `MEASURE_REPRESENTATION_ITEM` (same 2-path resolution as `magnitude`).
fn read_optional_unit_size(
    ctx: &ReaderContext,
    parts: &[RawEntityPart],
    entity_id: u64,
) -> Result<Option<ToleranceMagnitude>, ConvertError> {
    let Some(attrs) = find_part_attrs(parts, "GEOMETRIC_TOLERANCE_WITH_DEFINED_UNIT") else {
        return Ok(None);
    };
    check_count(attrs, 1, entity_id, "GEOMETRIC_TOLERANCE_WITH_DEFINED_UNIT")?;
    let unit_ref = read_entity_ref(attrs, 0, entity_id, "unit_size")?;
    Ok(resolve_tolerance_magnitude(ctx, unit_ref))
}

/// Read the `GEOMETRIC_TOLERANCE_WITH_DEFINED_AREA_UNIT` part —
/// `area_type` enum + optional `second_unit_size` (`length_measure_with_unit`).
/// The EXPRESS WHERE clause makes `second_unit_size` mandatory iff
/// `area_type == rectangular`; reader preserves whatever the source
/// emitted (warn on mismatch is left to future schema validation).
fn read_optional_defined_area_unit(
    ctx: &ReaderContext,
    parts: &[RawEntityPart],
    entity_id: u64,
) -> Result<Option<crate::ir::DefinedAreaUnit>, ConvertError> {
    use crate::ir::AreaUnitType;
    let Some(attrs) = find_part_attrs(parts, "GEOMETRIC_TOLERANCE_WITH_DEFINED_AREA_UNIT") else {
        return Ok(None);
    };
    check_count(
        attrs,
        2,
        entity_id,
        "GEOMETRIC_TOLERANCE_WITH_DEFINED_AREA_UNIT",
    )?;
    let area_type = match attrs.first() {
        Some(Attribute::Enum(s)) => match s.as_str() {
            "CIRCULAR" => AreaUnitType::Circular,
            "RECTANGULAR" => AreaUnitType::Rectangular,
            "SQUARE" => AreaUnitType::Square,
            other => AreaUnitType::Other(other.to_owned()),
        },
        _ => return Ok(None),
    };
    let second_unit_size = match attrs.get(1) {
        Some(Attribute::EntityRef(n)) => resolve_tolerance_magnitude(ctx, *n),
        _ => None,
    };
    Ok(Some(crate::ir::DefinedAreaUnit {
        area_type,
        second_unit_size,
    }))
}

/// Read the `UNEQUALLY_DISPOSED_GEOMETRIC_TOLERANCE.displacement` part —
/// `ref_length_measure_with_unit`. Returns `None` when the part is absent
/// or the ref resolves to neither a units-pool `MEASURE_WITH_UNIT` nor a
/// `MEASURE_REPRESENTATION_ITEM` (same 2-path resolution as `magnitude`).
fn read_optional_displacement(
    ctx: &ReaderContext,
    parts: &[RawEntityPart],
    entity_id: u64,
) -> Result<Option<ToleranceMagnitude>, ConvertError> {
    let Some(attrs) = find_part_attrs(parts, "UNEQUALLY_DISPOSED_GEOMETRIC_TOLERANCE") else {
        return Ok(None);
    };
    check_count(
        attrs,
        1,
        entity_id,
        "UNEQUALLY_DISPOSED_GEOMETRIC_TOLERANCE",
    )?;
    let unit_ref = read_entity_ref(attrs, 0, entity_id, "displacement")?;
    Ok(resolve_tolerance_magnitude(ctx, unit_ref))
}

/// Read the form-tolerance complex form `(GEOMETRIC_TOLERANCE
/// [GEOMETRIC_TOLERANCE_WITH_MODIFIERS] <leaf>)` — used by the new
/// FLATNESS / ROUNDNESS complex handlers (form-tolerance + modifier).
fn read_gt_data_complex(
    ctx: &ReaderContext,
    entity_id: u64,
    parts: &[RawEntityPart],
) -> Result<Option<GeometricToleranceData>, ConvertError> {
    let gt_attrs = require_part_attrs(parts, "GEOMETRIC_TOLERANCE", entity_id)?;
    check_count(gt_attrs, 4, entity_id, "GEOMETRIC_TOLERANCE")?;
    let name = read_string_or_unset(gt_attrs, 0, entity_id, "name")?.to_owned();
    let description = read_string_or_unset(gt_attrs, 1, entity_id, "description")?.to_owned();
    let magnitude_ref = read_entity_ref(gt_attrs, 2, entity_id, "magnitude")?;
    let shape_aspect_ref = read_entity_ref(gt_attrs, 3, entity_id, "toleranced_shape_aspect")?;
    let Some(magnitude) = resolve_tolerance_magnitude(ctx, magnitude_ref) else {
        return Ok(None);
    };
    let Some(toleranced_shape_aspect) = resolve_geometric_tolerance_target(ctx, shape_aspect_ref)
    else {
        return Ok(None);
    };
    let modifiers = read_optional_modifiers(parts, entity_id)?;
    let unit_size = read_optional_unit_size(ctx, parts, entity_id)?;
    // WDAU cascades from WDU per EXPRESS — drop WDAU when WDU's ref did
    // not resolve. Mirrors the writer's nested emit (WDAU only inside
    // the WDU branch). Without this guard, an IR with (unit_size: None,
    // defined_area_unit: Some(_)) would write as simple form and re-read
    // as (None, None) — IR mismatch (round-trip FAIL).
    let defined_area_unit = if unit_size.is_some() {
        read_optional_defined_area_unit(ctx, parts, entity_id)?
    } else {
        None
    };
    Ok(Some(GeometricToleranceData {
        name,
        description,
        magnitude,
        toleranced_shape_aspect,
        modifiers,
        unit_size,
        defined_area_unit,
    }))
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
        ToleranceMagnitude::MeasureWithUnit(id) => buf.step_id(id),
        ToleranceMagnitude::RepresentationItem(id) => buf.step_id(id),
    };
    let shape_aspect = buf.emit_geometric_tolerance_target(data.toleranced_shape_aspect);
    let mut datum_system_refs = Vec::with_capacity(data.datum_system.len());
    for ds_id in &data.datum_system {
        datum_system_refs.push(Attribute::EntityRef(buf.step_id(ds_id)));
    }
    let force_complex = is_complex || !data.modifiers.is_empty() || data.displacement.is_some();
    if force_complex {
        let mut parts = Vec::with_capacity(5);
        parts.push((
            "GEOMETRIC_TOLERANCE".into(),
            vec![
                Attribute::String(data.name),
                Attribute::String(data.description),
                Attribute::EntityRef(magnitude),
                Attribute::EntityRef(shape_aspect),
            ],
        ));
        parts.push((
            "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE".into(),
            vec![Attribute::List(datum_system_refs)],
        ));
        if !data.modifiers.is_empty() {
            parts.push((
                "GEOMETRIC_TOLERANCE_WITH_MODIFIERS".into(),
                vec![Attribute::List(emit_modifier_set(&data.modifiers))],
            ));
        }
        if let Some(disp) = &data.displacement {
            let disp_step = emit_tolerance_magnitude(buf, disp);
            parts.push((
                "UNEQUALLY_DISPOSED_GEOMETRIC_TOLERANCE".into(),
                vec![Attribute::EntityRef(disp_step)],
            ));
        }
        parts.push((type_name.into(), vec![]));
        let n = buf.fresh();
        buf.entities.push(WriterEntity {
            id: n,
            body: WriterBody::Complex { parts },
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

#[step_entity(name = "ANGULARITY_TOLERANCE")]
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
        push_gt_with_datum_reference(
            ctx,
            entity_id,
            GeometricToleranceWithDatumReference::Angularity(data),
        );
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

#[step_entity(name = "CIRCULAR_RUNOUT_TOLERANCE")]
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
        push_gt_with_datum_reference(
            ctx,
            entity_id,
            GeometricToleranceWithDatumReference::CircularRunout(data),
        );
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

#[step_entity(name = "CONCENTRICITY_TOLERANCE")]
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
        push_gt_with_datum_reference(
            ctx,
            entity_id,
            GeometricToleranceWithDatumReference::Concentricity(data),
        );
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

#[step_entity(name = "PARALLELISM_TOLERANCE")]
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
        push_gt_with_datum_reference(
            ctx,
            entity_id,
            GeometricToleranceWithDatumReference::Parallelism(data),
        );
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

#[step_entity(name = "PERPENDICULARITY_TOLERANCE")]
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
        push_gt_with_datum_reference(
            ctx,
            entity_id,
            GeometricToleranceWithDatumReference::Perpendicularity(data),
        );
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

#[step_entity(name = "SYMMETRY_TOLERANCE")]
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
        push_gt_with_datum_reference(
            ctx,
            entity_id,
            GeometricToleranceWithDatumReference::Symmetry(data),
        );
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

#[step_entity(name = "TOTAL_RUNOUT_TOLERANCE")]
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
        push_gt_with_datum_reference(
            ctx,
            entity_id,
            GeometricToleranceWithDatumReference::TotalRunout(data),
        );
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
    cases = [
        ["GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE", "GEOMETRIC_TOLERANCE_WITH_MODIFIERS", "POSITION_TOLERANCE"],
        ["GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE", "POSITION_TOLERANCE"],
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
        push_gt_with_datum_reference(
            ctx,
            entity_id,
            GeometricToleranceWithDatumReference::Position(data),
        );
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
    cases = [
        ["GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE", "GEOMETRIC_TOLERANCE_WITH_MODIFIERS", "SURFACE_PROFILE_TOLERANCE"],
        ["GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE", "SURFACE_PROFILE_TOLERANCE"],
        ["GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE", "SURFACE_PROFILE_TOLERANCE", "UNEQUALLY_DISPOSED_GEOMETRIC_TOLERANCE"],
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
        push_gt_with_datum_reference(
            ctx,
            entity_id,
            GeometricToleranceWithDatumReference::SurfaceProfile(data),
        );
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
    cases = [["GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE", "LINE_PROFILE_TOLERANCE"]]
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
        push_gt_with_datum_reference(
            ctx,
            entity_id,
            GeometricToleranceWithDatumReference::LineProfile(data),
        );
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
/// `RepresentationItem` references the `representation_item` arena's cached
/// step id. Read-only: both step-id caches are populated by earlier passes.
fn emit_tolerance_magnitude(buf: &WriteBuffer, m: &ToleranceMagnitude) -> u64 {
    match m {
        ToleranceMagnitude::MeasureWithUnit(id) => buf.step_id(id),
        ToleranceMagnitude::RepresentationItem(id) => buf.step_id(id),
    }
}

pub(crate) struct ToleranceValueHandler;

#[step_entity(name = "TOLERANCE_VALUE")]
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
        ctx.id_cache.insert(entity_id, id);
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

#[step_entity(name = "LIMITS_AND_FITS")]
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
        ctx.id_cache.insert(entity_id, id);
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
    if let Some(id) = ctx.id_cache.get::<crate::ir::ToleranceValueId>(item_ref) {
        return Some(ToleranceMethodDefinition::Value(id));
    }
    ctx.id_cache
        .get::<crate::ir::id::LimitsAndFitsId>(item_ref)
        .map(ToleranceMethodDefinition::LimitsAndFits)
}

/// Resolve a `dimensional_characteristic` SELECT ref (`PLUS_MINUS_TOLERANCE`'s
/// `toleranced_dimension`, `DIMENSIONAL_CHARACTERISTIC_REPRESENTATION`'s
/// `dimension`) — a `dimensional_location` or `dimensional_size`. A
/// `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE` is a `dimensional_size` that lives in
/// the `datum_feature` arena, so probe that too (only when the entry is the
/// DSWDF variant, never a plain `DATUM_FEATURE`).
pub(crate) fn resolve_dimensional_characteristic(
    ctx: &ReaderContext,
    item_ref: u64,
) -> Option<DimensionalCharacteristic> {
    if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::DimensionalLocationId>(item_ref)
    {
        return Some(DimensionalCharacteristic::Location(id));
    }
    if let Some(id) = ctx.id_cache.get::<crate::ir::DimensionalSizeId>(item_ref) {
        return Some(DimensionalCharacteristic::Size(id));
    }
    if let Some(df_id) = ctx.id_cache.get::<crate::ir::DatumFeatureId>(item_ref)
        && let Some(pmi) = ctx.pmi.as_ref()
        && matches!(
            pmi.datum_features[df_id],
            DatumFeature::DimensionalSizeWithDatumFeature(_)
        )
    {
        return Some(DimensionalCharacteristic::SizeWithDatumFeature(df_id));
    }
    None
}

pub(crate) struct PlusMinusToleranceHandler;

#[step_entity(name = "PLUS_MINUS_TOLERANCE")]
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
        ToleranceMethodDefinition::Value(id) => buf.step_id(id),
        ToleranceMethodDefinition::LimitsAndFits(id) => buf.step_id(id),
    };
    let dimension = match pmt.toleranced_dimension {
        DimensionalCharacteristic::Location(id) => buf.step_id(id),
        DimensionalCharacteristic::Size(id) => buf.step_id(id),
        DimensionalCharacteristic::SizeWithDatumFeature(id) => buf.step_id(id),
    };
    buf.push_simple(
        "PLUS_MINUS_TOLERANCE",
        vec![Attribute::EntityRef(range), Attribute::EntityRef(dimension)],
    )
}

// =================================================================
// Phase gt-modifiers — 5 신규 complex handler.
// modifier 가 함께 있는 simple-leaf 인스턴스를 complex MI 로 dispatch.
// required = [LEAF_NAME] (1-part) → 기존 3-part complex (Position/SP/LP)
// 와 disjoint, subset 충돌 없음. dispatch 분기: simple instance 는
// 기존 SimpleEntityHandler, complex 는 신규 ComplexEntityHandler.
// =================================================================

pub(crate) struct FlatnessToleranceComplexHandler;

#[step_entity_complex(
    name = "FLATNESS_TOLERANCE",
    cases = [
        ["FLATNESS_TOLERANCE", "GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_DEFINED_AREA_UNIT", "GEOMETRIC_TOLERANCE_WITH_DEFINED_UNIT"],
        // DEFINED_UNIT without the AREA_UNIT subtype (area is optional). Absent
        // from the corpus, exercised by writer_smoke round-trip fixtures.
        ["FLATNESS_TOLERANCE", "GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_DEFINED_UNIT"],
        ["FLATNESS_TOLERANCE", "GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_MODIFIERS"],
    ]
)]
impl ComplexEntityHandler for FlatnessToleranceComplexHandler {
    type WriteInput = GeometricTolerance;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_gt_data_complex(ctx, entity_id, parts)? else {
            return Ok(());
        };
        push_geometric_tolerance(ctx, entity_id, GeometricTolerance::Flatness(data));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gt: GeometricTolerance) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance(buf, gt))
    }
}

pub(crate) struct RoundnessToleranceComplexHandler;

#[step_entity_complex(
    name = "ROUNDNESS_TOLERANCE",
    cases = [["GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_MODIFIERS", "ROUNDNESS_TOLERANCE"]]
)]
impl ComplexEntityHandler for RoundnessToleranceComplexHandler {
    type WriteInput = GeometricTolerance;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_gt_data_complex(ctx, entity_id, parts)? else {
            return Ok(());
        };
        push_geometric_tolerance(ctx, entity_id, GeometricTolerance::Roundness(data));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gt: GeometricTolerance) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance(buf, gt))
    }
}

pub(crate) struct StraightnessToleranceComplexHandler;

#[step_entity_complex(
    name = "STRAIGHTNESS_TOLERANCE",
    cases = [["GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_DEFINED_UNIT", "STRAIGHTNESS_TOLERANCE"]]
)]
impl ComplexEntityHandler for StraightnessToleranceComplexHandler {
    type WriteInput = GeometricTolerance;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) = read_gt_data_complex(ctx, entity_id, parts)? else {
            return Ok(());
        };
        push_geometric_tolerance(ctx, entity_id, GeometricTolerance::Straightness(data));
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gt: GeometricTolerance) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance(buf, gt))
    }
}

/// Read a `geometric_tolerance_with_datum_reference` simple-leaf in
/// complex form: GT (required) + WDR (required) + optional WM. Used by
/// `PARALLELISM` / `PERPENDICULARITY` / `CIRCULAR_RUNOUT` complex handlers.
/// WDR absence drops the entry with a warning — `datum_ref` simple leaves
/// have no meaningful datum-less complex form.
fn read_gtwdr_simple_leaf_complex(
    ctx: &mut ReaderContext,
    entity_id: u64,
    parts: &[RawEntityPart],
    entity_name: &'static str,
) -> Result<Option<GeometricToleranceWithDatumReferenceData>, ConvertError> {
    let gt_attrs = require_part_attrs(parts, "GEOMETRIC_TOLERANCE", entity_id)?;
    check_count(gt_attrs, 4, entity_id, "GEOMETRIC_TOLERANCE")?;
    let name = read_string_or_unset(gt_attrs, 0, entity_id, "name")?.to_owned();
    let description = read_string_or_unset(gt_attrs, 1, entity_id, "description")?.to_owned();
    let magnitude_ref = read_entity_ref(gt_attrs, 2, entity_id, "magnitude")?;
    let shape_aspect_ref = read_entity_ref(gt_attrs, 3, entity_id, "toleranced_shape_aspect")?;
    let Some(gtwdr_attrs) = find_part_attrs(parts, "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE")
    else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "complex {entity_name} missing GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE part"
            ),
        });
        return Ok(None);
    };
    let datum_system_refs = read_entity_ref_list(gtwdr_attrs, 0, entity_id, "datum_system")?;
    let modifiers = read_optional_modifiers(parts, entity_id)?;
    let displacement = read_optional_displacement(ctx, parts, entity_id)?;
    Ok(build_gt_with_datum_reference_data(
        ctx,
        name,
        description,
        magnitude_ref,
        shape_aspect_ref,
        &datum_system_refs,
        modifiers,
        displacement,
    ))
}

pub(crate) struct ParallelismToleranceComplexHandler;

#[step_entity_complex(
    name = "PARALLELISM_TOLERANCE",
    cases = [["GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE", "GEOMETRIC_TOLERANCE_WITH_MODIFIERS", "PARALLELISM_TOLERANCE"]]
)]
impl ComplexEntityHandler for ParallelismToleranceComplexHandler {
    type WriteInput = GeometricToleranceWithDatumReference;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) =
            read_gtwdr_simple_leaf_complex(ctx, entity_id, parts, "PARALLELISM_TOLERANCE")?
        else {
            return Ok(());
        };
        push_gt_with_datum_reference(
            ctx,
            entity_id,
            GeometricToleranceWithDatumReference::Parallelism(data),
        );
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
}

pub(crate) struct PerpendicularityToleranceComplexHandler;

#[step_entity_complex(
    name = "PERPENDICULARITY_TOLERANCE",
    cases = [["GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE", "GEOMETRIC_TOLERANCE_WITH_MODIFIERS", "PERPENDICULARITY_TOLERANCE"]]
)]
impl ComplexEntityHandler for PerpendicularityToleranceComplexHandler {
    type WriteInput = GeometricToleranceWithDatumReference;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) =
            read_gtwdr_simple_leaf_complex(ctx, entity_id, parts, "PERPENDICULARITY_TOLERANCE")?
        else {
            return Ok(());
        };
        push_gt_with_datum_reference(
            ctx,
            entity_id,
            GeometricToleranceWithDatumReference::Perpendicularity(data),
        );
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
}

pub(crate) struct CircularRunoutToleranceComplexHandler;

#[step_entity_complex(
    name = "CIRCULAR_RUNOUT_TOLERANCE",
    cases = [["CIRCULAR_RUNOUT_TOLERANCE", "GEOMETRIC_TOLERANCE", "GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE", "GEOMETRIC_TOLERANCE_WITH_MODIFIERS"]]
)]
impl ComplexEntityHandler for CircularRunoutToleranceComplexHandler {
    type WriteInput = GeometricToleranceWithDatumReference;

    fn read_complex(
        ctx: &mut ReaderContext,
        entity_id: u64,
        parts: &[RawEntityPart],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let Some(data) =
            read_gtwdr_simple_leaf_complex(ctx, entity_id, parts, "CIRCULAR_RUNOUT_TOLERANCE")?
        else {
            return Ok(());
        };
        push_gt_with_datum_reference(
            ctx,
            entity_id,
            GeometricToleranceWithDatumReference::CircularRunout(data),
        );
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
}

/// Read the shared 5-attribute `draughting_model_item_association` body
/// (`name, description, definition, used_representation, identified_item`),
/// returning `annotation_placeholder: None`. Shared by the plain DMIA handler
/// and the `_WITH_PLACEHOLDER` subtype handler (which overrides the placeholder).
/// Returns `Ok(None)` on any unresolved ref (drop, symmetric on re-read).
fn read_dmia_base(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
) -> Result<Option<DraughtingModelItemAssociation>, ConvertError> {
    let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
    let description = match &attrs[1] {
        Attribute::Unset => None,
        Attribute::String(s) => Some(s.clone()),
        _ => return Ok(None),
    };
    let def_ref = read_entity_ref(attrs, 2, entity_id, "definition")?;
    let definition = if let Some(id) = ctx.id_cache.get::<crate::ir::id::RepresentationId>(def_ref)
    {
        DraughtingModelItemDefinition::Representation(id)
    } else if let Some(id) = ctx.id_cache.get::<crate::ir::DimensionalSizeId>(def_ref) {
        DraughtingModelItemDefinition::DimensionalSize(id)
    } else if let Some(sa_ref) = resolve_shape_aspect_ref(ctx, def_ref) {
        // shape_aspect member — any concrete subtype (datum / all_around /
        // datum_feature / …) via the shared ShapeAspectRef.
        DraughtingModelItemDefinition::ShapeAspect(sa_ref)
    } else if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::id::PropertyDefinitionId>(def_ref)
    {
        DraughtingModelItemDefinition::PropertyDefinition(id)
    } else if let Some(id) = ctx
        .id_cache
        .get::<crate::ir::DimensionalLocationId>(def_ref)
    {
        DraughtingModelItemDefinition::DimensionalLocation(id)
    } else if let Some(gt_ref) = resolve_geometric_tolerance_ref(ctx, def_ref) {
        // geometric_tolerance member — Plain or WithDatumReference complex MI.
        DraughtingModelItemDefinition::GeometricTolerance(gt_ref)
    } else {
        ctx.warnings.push(ConvertError::UnexpectedEntityForm {
            entity_id,
            detail: format!(
                "DRAUGHTING_MODEL_ITEM_ASSOCIATION definition #{def_ref} \
                 resolves to none of the 6 modelled SELECT members — skipping"
            ),
        });
        return Ok(None);
    };
    let used_ref = read_entity_ref(attrs, 3, entity_id, "used_representation")?;
    let Some(used_representation) = ctx
        .id_cache
        .get::<crate::ir::id::RepresentationId>(used_ref)
    else {
        return Ok(None);
    };
    let item_ref = read_entity_ref(attrs, 4, entity_id, "identified_item")?;
    // Members + probe order are generated from the enum by `StepSelect`.
    let Some(identified_item) = DraughtingModelIdentifiedItem::resolve_select(ctx, item_ref) else {
        return Ok(None);
    };
    Ok(Some(DraughtingModelItemAssociation {
        name,
        description,
        definition,
        used_representation,
        identified_item,
        annotation_placeholder: None,
    }))
}

pub(crate) struct DraughtingModelItemAssociationHandler;

#[step_entity(name = "DRAUGHTING_MODEL_ITEM_ASSOCIATION")]
impl SimpleEntityHandler for DraughtingModelItemAssociationHandler {
    type WriteInput = DraughtingModelItemAssociation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 5, entity_id, "DRAUGHTING_MODEL_ITEM_ASSOCIATION")?;
        let Some(dmia) = read_dmia_base(ctx, entity_id, attrs)? else {
            return Ok(());
        };
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .draughting_model_item_associations
            .push(dmia);
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        dmia: DraughtingModelItemAssociation,
    ) -> Result<u64, WriteError> {
        let def_step = match dmia.definition {
            DraughtingModelItemDefinition::Representation(id) => buf.step_id(id),
            DraughtingModelItemDefinition::DimensionalSize(id) => buf.step_id(id),
            DraughtingModelItemDefinition::ShapeAspect(sa_ref) => buf.emit_shape_aspect_ref(sa_ref),
            DraughtingModelItemDefinition::PropertyDefinition(id) => buf.step_id(id),
            DraughtingModelItemDefinition::DimensionalLocation(id) => buf.step_id(id),
            DraughtingModelItemDefinition::GeometricTolerance(r) => match r {
                GeometricToleranceRef::Plain(id) => buf.step_id(id),
                GeometricToleranceRef::WithDatumReference(id) => buf.step_id(id),
            },
        };
        let used_step = buf.step_id(dmia.used_representation);
        let item_step = dmia.identified_item.emit_select(buf);
        let description_attr = match dmia.description {
            Some(s) => Attribute::String(s),
            None => Attribute::Unset,
        };
        let mut body = vec![
            Attribute::String(dmia.name),
            description_attr,
            Attribute::EntityRef(def_step),
            Attribute::EntityRef(used_step),
            Attribute::EntityRef(item_step),
        ];
        // `nested_field`: the `_WITH_PLACEHOLDER` subtype appends the
        // annotation_placeholder ref and emits under the subtype name.
        match dmia.annotation_placeholder {
            Some(ph) => {
                body.push(Attribute::EntityRef(buf.step_id(ph)));
                Ok(buf.push_simple("DRAUGHTING_MODEL_ITEM_ASSOCIATION_WITH_PLACEHOLDER", body))
            }
            None => Ok(buf.push_simple("DRAUGHTING_MODEL_ITEM_ASSOCIATION", body)),
        }
    }
}

pub(crate) struct DraughtingModelItemAssociationWithPlaceholderHandler;

/// `DRAUGHTING_MODEL_ITEM_ASSOCIATION_WITH_PLACEHOLDER(name, description,
/// definition, used_representation, identified_item, annotation_placeholder)`
/// — blueprint `nested_field` subtype of `DRAUGHTING_MODEL_ITEM_ASSOCIATION`
/// carrying an `ANNOTATION_PLACEHOLDER_OCCURRENCE`. Shares the base body via
/// [`read_dmia_base`] and the same arena / `dmia_id_map`; the writer is on the
/// base handler (it branches on `annotation_placeholder`).
#[step_entity(name = "DRAUGHTING_MODEL_ITEM_ASSOCIATION_WITH_PLACEHOLDER")]
impl SimpleEntityHandler for DraughtingModelItemAssociationWithPlaceholderHandler {
    type WriteInput = DraughtingModelItemAssociation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(
            attrs,
            6,
            entity_id,
            "DRAUGHTING_MODEL_ITEM_ASSOCIATION_WITH_PLACEHOLDER",
        )?;
        let Some(mut dmia) = read_dmia_base(ctx, entity_id, attrs)? else {
            return Ok(());
        };
        let ph_ref = read_entity_ref(attrs, 5, entity_id, "annotation_placeholder")?;
        let Some(ph_id) = ctx
            .id_cache
            .get::<crate::ir::id::AnnotationOccurrenceId>(ph_ref)
        else {
            return Ok(());
        };
        dmia.annotation_placeholder = Some(ph_id);
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .draughting_model_item_associations
            .push(dmia);
        ctx.id_cache.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        dmia: DraughtingModelItemAssociation,
    ) -> Result<u64, WriteError> {
        // Unused: the arena emit (`emit_dmia`) always routes through the base
        // handler, which branches on `annotation_placeholder`. Delegate for
        // trait completeness.
        DraughtingModelItemAssociationHandler::write(buf, dmia)
    }
}
