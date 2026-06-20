//! `pmi` pool entity handlers.
//!
//! Three dependency-free `single_struct` primitives — `TOLERANCE_ZONE_FORM`,
//! `TYPE_QUALIFIER`, `VALUE_FORMAT_TYPE_QUALIFIER` — each a 1-attr string
//! entity pushed into [`PmiPool`]. They have no entity references; the
//! GD&T entities that consume them arrive in later phases.

use crate::early::{bind, lift, lower, serialize};
use crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::GeometricToleranceTarget;
use crate::ir::PmiPool;
use crate::ir::error::ConvertError;
use crate::ir::pmi::{
    AnnotationOccurrenceAssociativity, AnnotationOccurrenceRef, AnnotationPlaceholderLeaderLine,
    AnnotationPlaceholderOccurrence, AnnotationPlaceholderOccurrenceWithLeaderLine,
    AnnotationPlane, AnnotationSymbolOccurrence, AnnotationTextOccurrence, ApllPointElement,
    DatumFeature, DimensionalCharacteristic, DimensionalLocation, DimensionalSize,
    DimensionalSizeKind, DraughtingAnnotationOccurrence, DraughtingCalloutData,
    DraughtingCalloutElement, DraughtingCalloutRelationship, DraughtingModelItemAssociation,
    DraughtingModelItemDefinition, DraughtingPreDefinedTextFont, GeneralDatumReference,
    GeometricTolerance, GeometricToleranceRef, GeometricToleranceRelationship,
    GeometricToleranceWithDatumReference, GeometricToleranceWithDatumReferenceData, LeaderCurve,
    LeaderTerminator, LimitsAndFits, MeasureQualification, PlainAnnotationCurveOccurrence,
    PlainAnnotationOccurrence, PlusMinusTolerance, ProjectedZoneDefinition, TerminatorSymbol,
    TessellatedAnnotationOccurrence, ToleranceMagnitude, ToleranceMethodDefinition, ToleranceValue,
    ToleranceZoneForm, TypeQualifier, ValueFormatTypeQualifier,
};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::ReaderContext;
use crate::writer::WriteError;
use crate::writer::buffer::WriteBuffer;
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
        let early = bind::bind_draughting_pre_defined_text_font(entity_id, attrs)?;
        lower::lower_draughting_pre_defined_text_font(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, font: DraughtingPreDefinedTextFont) -> Result<u64, WriteError> {
        let early = lift::lift_draughting_pre_defined_text_font(font.name);
        Ok(serialize::serialize_draughting_pre_defined_text_font(
            buf, &early,
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
        let early = bind::bind_tessellated_annotation_occurrence(entity_id, attrs)?;
        lower::lower_tessellated_annotation_occurrence(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        tao: TessellatedAnnotationOccurrence,
    ) -> Result<u64, WriteError> {
        Ok(serialize::serialize_tessellated_annotation_occurrence(
            buf,
            &lift::lift_tessellated_annotation_occurrence(buf, tao),
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
        let early = bind::bind_apll_point(entity_id, attrs)?;
        lower::lower_apll_point(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, apll: ApllPointElement) -> Result<u64, WriteError> {
        // The shared arena holds both subtypes; this base handler is the sole
        // emitter and routes each variant to its own generated `serialize`
        // (the `ApllPointWithSurfaceHandler::write` is `unreachable!`).
        match apll {
            ApllPointElement::ApllPoint(data) => {
                let early =
                    lift::lift_apll_point(data.name, &data.coordinates, data.symbol_applied);
                Ok(serialize::serialize_apll_point(buf, &early))
            }
            ApllPointElement::ApllPointWithSurface(data) => {
                // Surfaces (`face_surface`) are emitted in the topology pass,
                // well before the PMI pass, so the step id is populated here.
                let surface_step = buf.step_id(data.associated_surface);
                let early = lift::lift_apll_point_with_surface(
                    data.name,
                    &data.coordinates,
                    data.symbol_applied,
                    surface_step,
                );
                Ok(serialize::serialize_apll_point_with_surface(buf, &early))
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
        let early = bind::bind_apll_point_with_surface(entity_id, attrs)?;
        lower::lower_apll_point_with_surface(ctx, entity_id, early);
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
        let early = bind::bind_annotation_to_model_leader_line(entity_id, attrs)?;
        lower::lower_annotation_to_model_leader_line(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        leader: AnnotationPlaceholderLeaderLine,
    ) -> Result<u64, WriteError> {
        // The shared arena holds both subtypes; this base handler is the sole
        // emitter and routes each variant to its own generated `serialize`
        // (the `AuxiliaryLeaderLineHandler::write` is `unreachable!`).
        match leader {
            AnnotationPlaceholderLeaderLine::AnnotationToModelLeaderLine(data) => {
                let elems = data
                    .geometric_elements
                    .iter()
                    .map(|id| buf.step_id(id))
                    .collect();
                let early = lift::lift_annotation_to_model_leader_line(data.name, elems);
                Ok(serialize::serialize_annotation_to_model_leader_line(
                    buf, &early,
                ))
            }
            AnnotationPlaceholderLeaderLine::AuxiliaryLeaderLine(data) => {
                let elems = data
                    .geometric_elements
                    .iter()
                    .map(|id| buf.step_id(id))
                    .collect();
                // `controlling_leader_line` points at another member of the same
                // arena. Topo order processes it first (lower arena index), so
                // its step id is already in the partially-built cache; `.step_id`
                // keeps any ordering violation a visible dangling 0, not a panic.
                let controlling_step = buf.step_id(data.controlling_leader_line);
                let early = lift::lift_auxiliary_leader_line(data.name, elems, controlling_step);
                Ok(serialize::serialize_auxiliary_leader_line(buf, &early))
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
        let early = bind::bind_auxiliary_leader_line(entity_id, attrs)?;
        lower::lower_auxiliary_leader_line(ctx, entity_id, early);
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
        let early = bind::bind_annotation_placeholder_occurrence(entity_id, attrs)?;
        lower::lower_annotation_placeholder_occurrence(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        apo: AnnotationPlaceholderOccurrence,
    ) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(apo.item)?;
        let style_refs = apo.styles.iter().map(|&psa| buf.step_id(psa)).collect();
        let early = lift::lift_annotation_placeholder_occurrence(
            apo.name,
            style_refs,
            item_id,
            apo.role,
            apo.line_spacing,
        );
        Ok(serialize::serialize_annotation_placeholder_occurrence(
            buf, &early,
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
        let early =
            bind::bind_annotation_placeholder_occurrence_with_leader_line(entity_id, attrs)?;
        lower::lower_annotation_placeholder_occurrence_with_leader_line(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        apo: AnnotationPlaceholderOccurrenceWithLeaderLine,
    ) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(apo.item)?;
        let style_refs = apo.styles.iter().map(|&psa| buf.step_id(psa)).collect();
        let leader_line = apo.leader_line.iter().map(|id| buf.step_id(id)).collect();
        let early = lift::lift_annotation_placeholder_occurrence_with_leader_line(
            apo.name,
            style_refs,
            item_id,
            apo.role,
            apo.line_spacing,
            leader_line,
        );
        Ok(serialize::serialize_annotation_placeholder_occurrence_with_leader_line(buf, &early))
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
        let early = bind::bind_terminator_symbol(entity_id, attrs)?;
        lower::lower_terminator_symbol(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ts: TerminatorSymbol) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(ts.item)?;
        let ac_step = buf.step_id(ts.annotated_curve);
        let style_refs = ts.styles.iter().map(|&psa| buf.step_id(psa)).collect();
        let early = lift::lift_terminator_symbol(ts.name, style_refs, item_id, ac_step);
        Ok(serialize::serialize_terminator_symbol(buf, &early))
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
        let early = bind::bind_draughting_callout_relationship(entity_id, attrs)?;
        lower::lower_draughting_callout_relationship(ctx, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, rel: DraughtingCalloutRelationship) -> Result<u64, WriteError> {
        let relating = buf.step_id(rel.relating);
        let related = buf.step_id(rel.related);
        let early = lift::lift_draughting_callout_relationship(
            rel.name,
            rel.description,
            relating,
            related,
        );
        Ok(serialize::serialize_draughting_callout_relationship(
            buf, &early,
        ))
    }
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
        let early = bind::bind_annotation_occurrence_associativity(entity_id, attrs)?;
        lower::lower_annotation_occurrence_associativity(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        aoa: AnnotationOccurrenceAssociativity,
    ) -> Result<u64, WriteError> {
        let relating = emit_annotation_occurrence_ref(buf, aoa.relating);
        let related = emit_annotation_occurrence_ref(buf, aoa.related);
        let early = lift::lift_annotation_occurrence_associativity(
            aoa.name,
            aoa.description,
            relating,
            related,
        );
        Ok(serialize::serialize_annotation_occurrence_associativity(
            buf, &early,
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
        let early = bind::bind_projected_zone_definition(entity_id, attrs)?;
        lower::lower_projected_zone_definition(ctx, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, pzd: ProjectedZoneDefinition) -> Result<u64, WriteError> {
        let zone_step = buf.step_id(pzd.zone);
        let projection_end_step = buf.emit_shape_aspect_ref(pzd.projection_end);
        let projected_length_step = emit_tolerance_magnitude(buf, &pzd.projected_length);
        let boundary_steps = pzd
            .boundaries
            .into_iter()
            .map(|sar| buf.emit_shape_aspect_ref(sar))
            .collect();
        let early = lift::lift_projected_zone_definition(
            zone_step,
            boundary_steps,
            projection_end_step,
            projected_length_step,
        );
        Ok(serialize::serialize_projected_zone_definition(buf, &early))
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
        let early = bind::bind_dimensional_size_with_datum_feature(entity_id, attrs)?;
        lower::lower_dimensional_size_with_datum_feature(ctx, entity_id, early);
        Ok(())
    }

    fn write(_buf: &mut WriteBuffer, _input: DatumFeatureWriteInput) -> Result<u64, WriteError> {
        unreachable!("DIMENSIONAL_SIZE_WITH_DATUM_FEATURE is emitted via emit_datum_features")
    }
}

/// Writer for `DATUM_FEATURE` (the only `DatumFeature::Itself` emit form; the
/// `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE` subtype emits via its own path).
fn write_datum_feature(buf: &mut WriteBuffer, input: DatumFeatureWriteInput) -> u64 {
    let early = crate::early::lift::lift_datum_feature(
        input.name,
        input.description,
        input.pds_step_id,
        input.product_definitional,
    );
    crate::early::serialize::serialize_datum_feature(buf, &early)
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
pub(crate) fn push_gt_with_datum_reference(
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
/// Emit a datum-free GT in its simple 4-attr form (no optional supertypes) via
/// the generated serialize. `magnitude` / `shape_aspect` are pre-resolved step ids.
fn write_gt_simple_form(
    buf: &mut WriteBuffer,
    entity_name: &str,
    data: crate::ir::pmi::GeometricToleranceData,
    magnitude: u64,
    shape_aspect: u64,
) -> u64 {
    match entity_name {
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
    }
}

/// Emit a migrated datum-free COMPLEX form (FLATNESS / ROUNDNESS / STRAIGHTNESS)
/// via the generated serialize. `magnitude` / `shape_aspect` are pre-resolved
/// step ids; the optional unit/area refs are resolved here.
fn write_gt_data_complex_migrated(
    buf: &mut WriteBuffer,
    entity_name: &str,
    data: crate::ir::pmi::GeometricToleranceData,
    magnitude: u64,
    shape_aspect: u64,
) -> u64 {
    let unit_size = data
        .unit_size
        .as_ref()
        .map(|us| emit_tolerance_magnitude(buf, us));
    let area = data.defined_area_unit.as_ref().map(|a| {
        (
            a.area_type.clone(),
            a.second_unit_size
                .as_ref()
                .map(|s| emit_tolerance_magnitude(buf, s)),
        )
    });
    match entity_name {
        "FLATNESS_TOLERANCE" => crate::early::serialize::serialize_flatness_tolerance_complex(
            buf,
            &crate::early::lift::lift_flatness_tolerance_complex(
                data.name,
                data.description,
                magnitude,
                shape_aspect,
                &data.modifiers,
                unit_size,
                area,
            ),
        ),
        "ROUNDNESS_TOLERANCE" => crate::early::serialize::serialize_roundness_tolerance_complex(
            buf,
            &crate::early::lift::lift_roundness_tolerance_complex(
                data.name,
                data.description,
                magnitude,
                shape_aspect,
                &data.modifiers,
            ),
        ),
        _ => crate::early::serialize::serialize_straightness_tolerance_complex(
            buf,
            &crate::early::lift::lift_straightness_tolerance_complex(
                data.name,
                data.description,
                magnitude,
                shape_aspect,
                unit_size.expect("STRAIGHTNESS complex requires unit_size"),
            ),
        ),
    }
}

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
        return write_gt_simple_form(buf, entity_name, data, magnitude, shape_aspect);
    }
    // Migrated datum-free COMPLEX leaves emit via the generated serialize
    // (lift picks the case variant). The rest stay hand-built below.
    if matches!(
        entity_name,
        "FLATNESS_TOLERANCE" | "ROUNDNESS_TOLERANCE" | "STRAIGHTNESS_TOLERANCE"
    ) {
        return write_gt_data_complex_migrated(buf, entity_name, data, magnitude, shape_aspect);
    }
    // CYLINDRICITY_TOLERANCE / datum-free SURFACE_PROFILE_TOLERANCE: the complex MI
    // form (GT + WDU/WDAU/WM parts) has no read handler — it is `UnhandledComplex`
    // (warn+drop) on read, so it is not reader-producible. A kernel-built IR that
    // sets those parts degrades to the simple form here rather than fabricating a
    // complex the reader cannot round-trip (symmetric with the read drop).
    write_gt_simple_form(buf, entity_name, data, magnitude, shape_aspect)
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

/// Non-standard `of_shape = $` guard for the two `general_datum_reference`
/// subtypes. `of_shape` is a mandatory `shape_aspect` ref (EXPRESS + UNIQUE
/// constraint) but some NIST exports (`ctc_05`) emit `$`. The generated `bind`
/// reads it as a required ref and would error on the bare `$`, so this drops
/// (recording the normalization) before `bind` — mirroring the legacy reader.
/// Returns `true` when the entry was dropped.
fn datum_reference_of_shape_unset(
    ctx: &mut ReaderContext,
    attrs: &[Attribute],
    entity_name: &'static str,
) -> bool {
    if matches!(attrs.get(2), Some(Attribute::Unset | Attribute::Derived)) {
        ctx.ns_record(
            crate::reader::NsCase::GeneralDatumReferenceOfShapeUnset,
            entity_name.into(),
            "dropped (of_shape Unset — EXPRESS shape_aspect.of_shape required)",
        );
        return true;
    }
    false
}

/// Emit a `GeneralDatumReference` under the STEP entity name its variant
/// selects, returning the STEP id. Shared by both handlers and by
/// `emit_general_datum_references`.
pub(crate) fn write_general_datum_reference(
    buf: &mut WriteBuffer,
    gdr: GeneralDatumReference,
) -> u64 {
    match gdr {
        GeneralDatumReference::Compartment(d) => {
            let early = crate::early::lift::lift_datum_reference_compartment(buf, d);
            crate::early::serialize::serialize_datum_reference_compartment(buf, &early)
        }
        GeneralDatumReference::Element(d) => {
            let early = crate::early::lift::lift_datum_reference_element(buf, d);
            crate::early::serialize::serialize_datum_reference_element(buf, &early)
        }
    }
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
        if datum_reference_of_shape_unset(ctx, attrs, "DATUM_REFERENCE_COMPARTMENT") {
            return Ok(());
        }
        let Some(early) = crate::early::bind::bind_datum_reference_compartment(entity_id, attrs)?
        else {
            return Ok(());
        };
        crate::early::lower::lower_datum_reference_compartment(ctx, entity_id, early, graph);
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
        if datum_reference_of_shape_unset(ctx, attrs, "DATUM_REFERENCE_ELEMENT") {
            return Ok(());
        }
        let Some(early) = crate::early::bind::bind_datum_reference_element(entity_id, attrs)?
        else {
            return Ok(());
        };
        crate::early::lower::lower_datum_reference_element(ctx, entity_id, early, graph);
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
pub(crate) fn build_gt_with_datum_reference_data(
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

/// Emit a `GeometricToleranceWithDatumReference`, returning the STEP id.
/// The seven direct subtypes emit as a simple 5-attr entity; `POSITION` /
/// `SURFACE_PROFILE` / `LINE_PROFILE` emit as the multiple-inheritance
/// complex `(GEOMETRIC_TOLERANCE GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE
/// <leaf>)` (parts in ISO 10303-21 alphabetical order).
#[allow(clippy::too_many_lines)]
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
    let datum_system_ids: Vec<u64> = data
        .datum_system
        .iter()
        .map(|ds_id| buf.step_id(ds_id))
        .collect();
    let force_complex = is_complex || !data.modifiers.is_empty() || data.displacement.is_some();
    if force_complex {
        // Migrated WDR simple-leaf COMPLEX forms emit via the generated serialize.
        // The rest (POSITION / SURFACE_PROFILE / LINE_PROFILE) stay hand-built below.
        match type_name {
            "PARALLELISM_TOLERANCE" => {
                return crate::early::serialize::serialize_parallelism_tolerance_complex(
                    buf,
                    &crate::early::lift::lift_parallelism_tolerance_complex(
                        data.name,
                        data.description,
                        magnitude,
                        shape_aspect,
                        datum_system_ids,
                        &data.modifiers,
                    ),
                );
            }
            "PERPENDICULARITY_TOLERANCE" => {
                return crate::early::serialize::serialize_perpendicularity_tolerance_complex(
                    buf,
                    &crate::early::lift::lift_perpendicularity_tolerance_complex(
                        data.name,
                        data.description,
                        magnitude,
                        shape_aspect,
                        datum_system_ids,
                        &data.modifiers,
                    ),
                );
            }
            "CIRCULAR_RUNOUT_TOLERANCE" => {
                return crate::early::serialize::serialize_circular_runout_tolerance_complex(
                    buf,
                    &crate::early::lift::lift_circular_runout_tolerance_complex(
                        data.name,
                        data.description,
                        magnitude,
                        shape_aspect,
                        datum_system_ids,
                        &data.modifiers,
                    ),
                );
            }
            "POSITION_TOLERANCE" => {
                return crate::early::serialize::serialize_position_tolerance_complex(
                    buf,
                    &crate::early::lift::lift_position_tolerance_complex(
                        data.name,
                        data.description,
                        magnitude,
                        shape_aspect,
                        datum_system_ids,
                        &data.modifiers,
                    ),
                );
            }
            "SURFACE_PROFILE_TOLERANCE" => {
                let displacement = data
                    .displacement
                    .as_ref()
                    .map(|d| emit_tolerance_magnitude(buf, d));
                return crate::early::serialize::serialize_surface_profile_tolerance_complex(
                    buf,
                    &crate::early::lift::lift_surface_profile_tolerance_complex(
                        data.name,
                        data.description,
                        magnitude,
                        shape_aspect,
                        datum_system_ids,
                        &data.modifiers,
                        displacement,
                    ),
                );
            }
            "LINE_PROFILE_TOLERANCE" => {
                return crate::early::serialize::serialize_line_profile_tolerance_complex(
                    buf,
                    &crate::early::lift::lift_line_profile_tolerance_complex(
                        data.name,
                        data.description,
                        magnitude,
                        shape_aspect,
                        datum_system_ids,
                    ),
                );
            }
            // ANGULARITY / CONCENTRICITY / SYMMETRY / TOTAL_RUNOUT reach here only
            // via kernel-set modifiers/displacement; their complex MI form has no
            // read handler (`UnhandledComplex` warn+drop on read), so it is not
            // reader-producible. Degrade to the simple 5-attr form below rather than
            // fabricate a complex the reader cannot round-trip (symmetric with the
            // read drop).
            _ => {}
        }
    }
    // Simple 5-attr emit via the generated serialize. Reached by the non-complex
    // subtypes and the degraded with-modifiers leaves above; POSITION /
    // SURFACE_PROFILE / LINE_PROFILE always return inside the complex match.
    {
        use crate::early::{lift, serialize};
        match type_name {
            "ANGULARITY_TOLERANCE" => serialize::serialize_angularity_tolerance(
                buf,
                &lift::lift_angularity_tolerance(
                    data.name,
                    data.description,
                    magnitude,
                    shape_aspect,
                    datum_system_ids,
                ),
            ),
            "CIRCULAR_RUNOUT_TOLERANCE" => serialize::serialize_circular_runout_tolerance(
                buf,
                &lift::lift_circular_runout_tolerance(
                    data.name,
                    data.description,
                    magnitude,
                    shape_aspect,
                    datum_system_ids,
                ),
            ),
            "CONCENTRICITY_TOLERANCE" => serialize::serialize_concentricity_tolerance(
                buf,
                &lift::lift_concentricity_tolerance(
                    data.name,
                    data.description,
                    magnitude,
                    shape_aspect,
                    datum_system_ids,
                ),
            ),
            "PARALLELISM_TOLERANCE" => serialize::serialize_parallelism_tolerance(
                buf,
                &lift::lift_parallelism_tolerance(
                    data.name,
                    data.description,
                    magnitude,
                    shape_aspect,
                    datum_system_ids,
                ),
            ),
            "PERPENDICULARITY_TOLERANCE" => serialize::serialize_perpendicularity_tolerance(
                buf,
                &lift::lift_perpendicularity_tolerance(
                    data.name,
                    data.description,
                    magnitude,
                    shape_aspect,
                    datum_system_ids,
                ),
            ),
            "SYMMETRY_TOLERANCE" => serialize::serialize_symmetry_tolerance(
                buf,
                &lift::lift_symmetry_tolerance(
                    data.name,
                    data.description,
                    magnitude,
                    shape_aspect,
                    datum_system_ids,
                ),
            ),
            "TOTAL_RUNOUT_TOLERANCE" => serialize::serialize_total_runout_tolerance(
                buf,
                &lift::lift_total_runout_tolerance(
                    data.name,
                    data.description,
                    magnitude,
                    shape_aspect,
                    datum_system_ids,
                ),
            ),
            _ => unreachable!("complex-only with-datum variant reached the simple branch"),
        }
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
        let early = crate::early::bind::bind_angularity_tolerance(entity_id, attrs)?;
        crate::early::lower::lower_angularity_tolerance(ctx, entity_id, early);
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
        let early = crate::early::bind::bind_circular_runout_tolerance(entity_id, attrs)?;
        crate::early::lower::lower_circular_runout_tolerance(ctx, entity_id, early);
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
        let early = crate::early::bind::bind_concentricity_tolerance(entity_id, attrs)?;
        crate::early::lower::lower_concentricity_tolerance(ctx, entity_id, early);
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
        let early = crate::early::bind::bind_parallelism_tolerance(entity_id, attrs)?;
        crate::early::lower::lower_parallelism_tolerance(ctx, entity_id, early);
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
        let early = crate::early::bind::bind_perpendicularity_tolerance(entity_id, attrs)?;
        crate::early::lower::lower_perpendicularity_tolerance(ctx, entity_id, early);
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
        let early = crate::early::bind::bind_symmetry_tolerance(entity_id, attrs)?;
        crate::early::lower::lower_symmetry_tolerance(ctx, entity_id, early);
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
        let early = crate::early::bind::bind_total_runout_tolerance(entity_id, attrs)?;
        crate::early::lower::lower_total_runout_tolerance(ctx, entity_id, early);
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
        let early = crate::early::bind::bind_position_tolerance_complex(entity_id, parts)?;
        crate::early::lower::lower_position_tolerance_complex(ctx, entity_id, early);
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
        let early = crate::early::bind::bind_surface_profile_tolerance_complex(entity_id, parts)?;
        crate::early::lower::lower_surface_profile_tolerance_complex(ctx, entity_id, early);
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
        let early = crate::early::bind::bind_line_profile_tolerance_complex(entity_id, parts)?;
        crate::early::lower::lower_line_profile_tolerance_complex(ctx, entity_id, early);
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
        let early = bind::bind_tolerance_value(entity_id, attrs)?;
        lower::lower_tolerance_value(ctx, entity_id, early);
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
    let early = lift::lift_tolerance_value(lower, upper);
    serialize::serialize_tolerance_value(buf, &early)
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
        let early = bind::bind_limits_and_fits(entity_id, attrs)?;
        lower::lower_limits_and_fits(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, lf: LimitsAndFits) -> Result<u64, WriteError> {
        Ok(write_limits_and_fits(buf, lf))
    }
}

/// Emit a `LIMITS_AND_FITS`, returning the STEP id.
pub(crate) fn write_limits_and_fits(buf: &mut WriteBuffer, lf: LimitsAndFits) -> u64 {
    let early = lift::lift_limits_and_fits(lf);
    serialize::serialize_limits_and_fits(buf, &early)
}

/// Resolve a `tolerance_method_definition` SELECT ref (`PLUS_MINUS_TOLERANCE`'s
/// `range`) — a `TOLERANCE_VALUE` or a `LIMITS_AND_FITS`.
pub(crate) fn resolve_tolerance_method_definition(
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
        let early = bind::bind_plus_minus_tolerance(entity_id, attrs)?;
        lower::lower_plus_minus_tolerance(ctx, early);
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
    let early = lift::lift_plus_minus_tolerance(range, dimension);
    serialize::serialize_plus_minus_tolerance(buf, &early)
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
        let early = crate::early::bind::bind_flatness_tolerance_complex(entity_id, parts)?;
        crate::early::lower::lower_flatness_tolerance_complex(ctx, entity_id, early);
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
        let early = crate::early::bind::bind_roundness_tolerance_complex(entity_id, parts)?;
        crate::early::lower::lower_roundness_tolerance_complex(ctx, entity_id, early);
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
        let early = crate::early::bind::bind_straightness_tolerance_complex(entity_id, parts)?;
        crate::early::lower::lower_straightness_tolerance_complex(ctx, entity_id, early);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, gt: GeometricTolerance) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance(buf, gt))
    }
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
        let early = crate::early::bind::bind_parallelism_tolerance_complex(entity_id, parts)?;
        crate::early::lower::lower_parallelism_tolerance_complex(ctx, entity_id, early);
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
        let early = crate::early::bind::bind_perpendicularity_tolerance_complex(entity_id, parts)?;
        crate::early::lower::lower_perpendicularity_tolerance_complex(ctx, entity_id, early);
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
        let early = crate::early::bind::bind_circular_runout_tolerance_complex(entity_id, parts)?;
        crate::early::lower::lower_circular_runout_tolerance_complex(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        gt: GeometricToleranceWithDatumReference,
    ) -> Result<u64, WriteError> {
        Ok(write_geometric_tolerance_with_datum_reference(buf, gt))
    }
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
        let early = bind::bind_draughting_model_item_association(entity_id, attrs)?;
        lower::lower_draughting_model_item_association(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        dmia: DraughtingModelItemAssociation,
    ) -> Result<u64, WriteError> {
        // The shared arena holds both subtypes; this base handler is the sole
        // emitter and routes on `annotation_placeholder` to each per-name
        // serialize (the `_WITH_PLACEHOLDER` handler's write is `unreachable!`).
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
        if let Some(ph) = dmia.annotation_placeholder {
            let ph_step = buf.step_id(ph);
            let early = lift::lift_dmia_with_placeholder(
                dmia.name,
                dmia.description,
                def_step,
                used_step,
                item_step,
                ph_step,
            );
            Ok(
                serialize::serialize_draughting_model_item_association_with_placeholder(
                    buf, &early,
                ),
            )
        } else {
            let early =
                lift::lift_dmia(dmia.name, dmia.description, def_step, used_step, item_step);
            Ok(serialize::serialize_draughting_model_item_association(
                buf, &early,
            ))
        }
    }
}

pub(crate) struct DraughtingModelItemAssociationWithPlaceholderHandler;

/// `DRAUGHTING_MODEL_ITEM_ASSOCIATION_WITH_PLACEHOLDER(name, description,
/// definition, used_representation, identified_item, annotation_placeholder)`
/// — blueprint `nested_field` subtype of `DRAUGHTING_MODEL_ITEM_ASSOCIATION`
/// carrying an `ANNOTATION_PLACEHOLDER_OCCURRENCE`. Shares the base body via
/// `lower_dmia_common` and the same arena; the writer is on the base handler
/// (it branches on `annotation_placeholder`).
#[step_entity(name = "DRAUGHTING_MODEL_ITEM_ASSOCIATION_WITH_PLACEHOLDER")]
impl SimpleEntityHandler for DraughtingModelItemAssociationWithPlaceholderHandler {
    type WriteInput = DraughtingModelItemAssociation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        let early =
            bind::bind_draughting_model_item_association_with_placeholder(entity_id, attrs)?;
        lower::lower_draughting_model_item_association_with_placeholder(ctx, entity_id, early);
        Ok(())
    }

    fn write(
        _buf: &mut WriteBuffer,
        _dmia: DraughtingModelItemAssociation,
    ) -> Result<u64, WriteError> {
        // The arena emit (`emit_dmia`) always routes through the base handler,
        // which branches on `annotation_placeholder` to the per-name serialize.
        unreachable!(
            "DRAUGHTING_MODEL_ITEM_ASSOCIATION_WITH_PLACEHOLDER is emitted via \
             DraughtingModelItemAssociationHandler::write"
        )
    }
}
