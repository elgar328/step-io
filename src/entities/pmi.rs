//! `pmi` pool entity handlers — Pass 8.
//!
//! Three dependency-free `single_struct` primitives — `TOLERANCE_ZONE_FORM`,
//! `TYPE_QUALIFIER`, `VALUE_FORMAT_TYPE_QUALIFIER` — each a 1-attr string
//! entity pushed into [`PmiPool`]. They have no entity references; the
//! GD&T entities that consume them arrive in later phases.

use crate::entities::shape_rep::shape_aspect_relationship::resolve_shape_aspect_ref;
use crate::entities::visualization::styled_item::resolve_representation_item_ref;
use crate::entities::{ComplexEntityHandler, SimpleEntityHandler};
use crate::ir::PmiPool;
use crate::ir::attr::{
    check_count, read_bool, read_entity_ref, read_entity_ref_list, read_enum, read_string_or_unset,
};
use crate::ir::error::ConvertError;
use crate::ir::pmi::{
    AngleSelection, AngularLocationData, AnnotationOccurrence, AnnotationPlane,
    AnnotationSymbolOccurrence, AnnotationTextOccurrence, Datum, DatumFeature,
    DimensionalCharacteristic, DimensionalLocation, DimensionalLocationData, DimensionalSize,
    DimensionalSizeKind, DraughtingAnnotationOccurrence, DraughtingCallout, DraughtingCalloutData,
    DraughtingCalloutElement, DraughtingCalloutRelationship, DraughtingModelIdentifiedItem,
    DraughtingModelItemAssociation, DraughtingModelItemDefinition, DraughtingPreDefinedTextFont,
    GeneralDatumBase, GeneralDatumReference, GeneralDatumReferenceData, GeometricTolerance,
    GeometricToleranceData, GeometricToleranceRef, GeometricToleranceRelationship,
    GeometricToleranceWithDatumReference, GeometricToleranceWithDatumReferenceData, LeaderCurve,
    LeaderTerminator, LimitsAndFits, MeasureQualification, PlainAnnotationOccurrence,
    PlusMinusTolerance, ProjectedZoneDefinition, TerminatorSymbol, TessellatedAnnotationOccurrence,
    ToleranceMagnitude, ToleranceMethodDefinition, ToleranceValue, ToleranceZoneForm,
    TypeQualifier, ValueFormatTypeQualifier, ValueQualifier,
};
use crate::parser::entity::{Attribute, EntityGraph, RawEntityPart};
use crate::reader::{ReaderContext, find_part_attrs, require_part_attrs};
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
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .tolerance_zone_forms
            .push(ToleranceZoneForm { name });
        ctx.tolerance_zone_form_id_map.insert(entity_id, id);
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
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .type_qualifiers
            .push(TypeQualifier { name });
        ctx.type_qualifier_id_map.insert(entity_id, id);
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
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .value_format_type_qualifiers
            .push(ValueFormatTypeQualifier { format_type });
        ctx.value_format_type_qualifier_id_map.insert(entity_id, id);
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
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .draughting_pre_defined_text_fonts
            .push(DraughtingPreDefinedTextFont { name });
        ctx.dptf_id_map.insert(entity_id, id);
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

        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .annotation_occurrences
            .push(AnnotationOccurrence::AnnotationPlane(AnnotationPlane {
                name,
                styles,
                item,
            }));
        ctx.annotation_occurrence_id_map.insert(entity_id, id);
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

        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .annotation_occurrences
            .push(AnnotationOccurrence::TessellatedAnnotationOccurrence(
                TessellatedAnnotationOccurrence { name, styles, item },
            ));
        ctx.annotation_occurrence_id_map.insert(entity_id, id);
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

pub(crate) struct AnnotationSymbolOccurrenceHandler;

/// `ANNOTATION_SYMBOL_OCCURRENCE(name, styles, item)` — an
/// `annotation_occurrence` subtype whose `item` is the
/// `annotation_symbol_occurrence_item` SELECT. step-io resolves `item`
/// through the generic `representation_item` resolver
/// (`resolve_representation_item_ref`); occurrences whose `item` does not
/// resolve are silently dropped, symmetric on re-read.
#[step_entity(name = "ANNOTATION_SYMBOL_OCCURRENCE", pass = Pass7AnnotationPlane)]
impl SimpleEntityHandler for AnnotationSymbolOccurrenceHandler {
    type WriteInput = AnnotationSymbolOccurrence;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "ANNOTATION_SYMBOL_OCCURRENCE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(&psa_id) = ctx.viz_psa_id_map.get(&r) {
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
            .push(AnnotationOccurrence::AnnotationSymbolOccurrence(
                AnnotationSymbolOccurrence { name, styles, item },
            ));
        ctx.annotation_occurrence_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, aso: AnnotationSymbolOccurrence) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(aso.item)?;
        let mut style_refs = Vec::with_capacity(aso.styles.len());
        for psa_id in aso.styles {
            style_refs.push(Attribute::EntityRef(buf.psa_step_ids[psa_id.0 as usize]));
        }
        Ok(buf.push_simple(
            "ANNOTATION_SYMBOL_OCCURRENCE",
            vec![
                Attribute::String(aso.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(item_id),
            ],
        ))
    }
}

pub(crate) struct AnnotationTextOccurrenceHandler;

/// `ANNOTATION_TEXT_OCCURRENCE(name, styles, item)` — an
/// `annotation_occurrence` subtype whose `item` is the
/// `annotation_text_occurrence_item` SELECT. Same resolve / drop policy
/// as `ANNOTATION_SYMBOL_OCCURRENCE`.
#[step_entity(name = "ANNOTATION_TEXT_OCCURRENCE", pass = Pass7AnnotationPlane)]
impl SimpleEntityHandler for AnnotationTextOccurrenceHandler {
    type WriteInput = AnnotationTextOccurrence;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "ANNOTATION_TEXT_OCCURRENCE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(&psa_id) = ctx.viz_psa_id_map.get(&r) {
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
            .push(AnnotationOccurrence::AnnotationTextOccurrence(
                AnnotationTextOccurrence { name, styles, item },
            ));
        ctx.annotation_occurrence_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ato: AnnotationTextOccurrence) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(ato.item)?;
        let mut style_refs = Vec::with_capacity(ato.styles.len());
        for psa_id in ato.styles {
            style_refs.push(Attribute::EntityRef(buf.psa_step_ids[psa_id.0 as usize]));
        }
        Ok(buf.push_simple(
            "ANNOTATION_TEXT_OCCURRENCE",
            vec![
                Attribute::String(ato.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(item_id),
            ],
        ))
    }
}

pub(crate) struct DraughtingAnnotationOccurrenceHandler;

/// `DRAUGHTING_ANNOTATION_OCCURRENCE(name, styles, item)` — an
/// `annotation_occurrence` subtype whose `item` is narrowed (via WHERE
/// constraints) to `ref_representation_item`. step-io resolves `item`
/// through `resolve_representation_item_ref`; unresolved items are
/// silently dropped.
#[step_entity(name = "DRAUGHTING_ANNOTATION_OCCURRENCE", pass = Pass7AnnotationPlane)]
impl SimpleEntityHandler for DraughtingAnnotationOccurrenceHandler {
    type WriteInput = DraughtingAnnotationOccurrence;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "DRAUGHTING_ANNOTATION_OCCURRENCE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(&psa_id) = ctx.viz_psa_id_map.get(&r) {
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
            .push(AnnotationOccurrence::DraughtingAnnotationOccurrence(
                DraughtingAnnotationOccurrence { name, styles, item },
            ));
        ctx.annotation_occurrence_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        dao: DraughtingAnnotationOccurrence,
    ) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(dao.item)?;
        let mut style_refs = Vec::with_capacity(dao.styles.len());
        for psa_id in dao.styles {
            style_refs.push(Attribute::EntityRef(buf.psa_step_ids[psa_id.0 as usize]));
        }
        Ok(buf.push_simple(
            "DRAUGHTING_ANNOTATION_OCCURRENCE",
            vec![
                Attribute::String(dao.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(item_id),
            ],
        ))
    }
}

pub(crate) struct AnnotationOccurrenceHandler;

/// `ANNOTATION_OCCURRENCE(name, styles, item)` — the plain `annotation_occurrence`
/// supertype, instantiated directly in some PMI corpora (e.g. as a
/// `DRAUGHTING_MODEL_ITEM_ASSOCIATION.identified_item` or a `DRAUGHTING_CALLOUT`
/// content). Same shape/handling as `DRAUGHTING_ANNOTATION_OCCURRENCE`.
#[step_entity(name = "ANNOTATION_OCCURRENCE", pass = Pass7AnnotationPlane)]
impl SimpleEntityHandler for AnnotationOccurrenceHandler {
    type WriteInput = PlainAnnotationOccurrence;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "ANNOTATION_OCCURRENCE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(&psa_id) = ctx.viz_psa_id_map.get(&r) {
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
            .push(AnnotationOccurrence::Plain(PlainAnnotationOccurrence {
                name,
                styles,
                item,
            }));
        ctx.annotation_occurrence_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ao: PlainAnnotationOccurrence) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(ao.item)?;
        let mut style_refs = Vec::with_capacity(ao.styles.len());
        for psa_id in ao.styles {
            style_refs.push(Attribute::EntityRef(buf.psa_step_ids[psa_id.0 as usize]));
        }
        Ok(buf.push_simple(
            "ANNOTATION_OCCURRENCE",
            vec![
                Attribute::String(ao.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(item_id),
            ],
        ))
    }
}

pub(crate) struct LeaderCurveHandler;

/// `LEADER_CURVE(name, styles, item)` — sole occupant of the
/// `annotation_curve_occurrence` arena. `item` resolves through
/// `ctx.curve_map`; unresolved items drop the occurrence, symmetric on
/// re-read. The arena id is recorded in
/// `ctx.annotation_curve_occurrence_id_map` so the `Pass7AnnotationPlane`
/// `TERMINATOR_SYMBOL` / `LEADER_TERMINATOR` handlers can resolve their
/// `annotated_curve` back-reference.
#[step_entity(name = "LEADER_CURVE", pass = Pass7AnnotationCurve)]
impl SimpleEntityHandler for LeaderCurveHandler {
    type WriteInput = LeaderCurve;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 3, entity_id, "LEADER_CURVE")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(&psa_id) = ctx.viz_psa_id_map.get(&r) {
                styles.push(psa_id);
            }
        }
        let Some(&item) = ctx.curve_map.get(&item_ref) else {
            return Ok(()); // item unresolved — drop the occurrence
        };

        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .annotation_curve_occurrences
            .push(LeaderCurve { name, styles, item });
        ctx.annotation_curve_occurrence_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, lc: LeaderCurve) -> Result<u64, WriteError> {
        let curve_step = buf.emit_curve(lc.item)?;
        let mut style_refs = Vec::with_capacity(lc.styles.len());
        for psa_id in lc.styles {
            style_refs.push(Attribute::EntityRef(buf.psa_step_ids[psa_id.0 as usize]));
        }
        Ok(buf.push_simple(
            "LEADER_CURVE",
            vec![
                Attribute::String(lc.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(curve_step),
            ],
        ))
    }
}

pub(crate) struct TerminatorSymbolHandler;

/// `TERMINATOR_SYMBOL(name, styles, item, annotated_curve)` — an
/// `annotation_symbol_occurrence` subtype with an `annotated_curve`
/// back-reference into the `annotation_curve_occurrence` arena.
/// Unresolved `item` (via `resolve_representation_item_ref`) or
/// `annotated_curve` (via `annotation_curve_occurrence_id_map`) drops
/// the occurrence, symmetric on re-read.
#[step_entity(name = "TERMINATOR_SYMBOL", pass = Pass7AnnotationPlane)]
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
            if let Some(&psa_id) = ctx.viz_psa_id_map.get(&r) {
                styles.push(psa_id);
            }
        }
        let Some(item) = resolve_representation_item_ref(ctx, item_ref) else {
            return Ok(());
        };
        let Some(&annotated_curve) = ctx.annotation_curve_occurrence_id_map.get(&ac_ref) else {
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
        ctx.annotation_occurrence_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, ts: TerminatorSymbol) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(ts.item)?;
        let ac_step = buf.acoc_step_ids[ts.annotated_curve.0 as usize];
        let mut style_refs = Vec::with_capacity(ts.styles.len());
        for psa_id in ts.styles {
            style_refs.push(Attribute::EntityRef(buf.psa_step_ids[psa_id.0 as usize]));
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

/// `LEADER_TERMINATOR(name, styles, item, annotated_curve)` — a
/// `terminator_symbol` subtype. Same shape and resolve / drop policy as
/// `TerminatorSymbol`; the EXPRESS WHERE narrowing `annotated_curve` to
/// `LEADER_CURVE` is not enforced at IR level.
#[step_entity(name = "LEADER_TERMINATOR", pass = Pass7AnnotationPlane)]
impl SimpleEntityHandler for LeaderTerminatorHandler {
    type WriteInput = LeaderTerminator;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "LEADER_TERMINATOR")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let style_refs = read_entity_ref_list(attrs, 1, entity_id, "styles")?;
        let item_ref = read_entity_ref(attrs, 2, entity_id, "item")?;
        let ac_ref = read_entity_ref(attrs, 3, entity_id, "annotated_curve")?;

        let mut styles = Vec::with_capacity(style_refs.len());
        for r in style_refs {
            if let Some(&psa_id) = ctx.viz_psa_id_map.get(&r) {
                styles.push(psa_id);
            }
        }
        let Some(item) = resolve_representation_item_ref(ctx, item_ref) else {
            return Ok(());
        };
        let Some(&annotated_curve) = ctx.annotation_curve_occurrence_id_map.get(&ac_ref) else {
            return Ok(());
        };

        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .annotation_occurrences
            .push(AnnotationOccurrence::LeaderTerminator(LeaderTerminator {
                name,
                styles,
                item,
                annotated_curve,
            }));
        ctx.annotation_occurrence_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, lt: LeaderTerminator) -> Result<u64, WriteError> {
        let item_id = buf.emit_representation_item_ref(lt.item)?;
        let ac_step = buf.acoc_step_ids[lt.annotated_curve.0 as usize];
        let mut style_refs = Vec::with_capacity(lt.styles.len());
        for psa_id in lt.styles {
            style_refs.push(Attribute::EntityRef(buf.psa_step_ids[psa_id.0 as usize]));
        }
        Ok(buf.push_simple(
            "LEADER_TERMINATOR",
            vec![
                Attribute::String(lt.name),
                Attribute::List(style_refs),
                Attribute::EntityRef(item_id),
                Attribute::EntityRef(ac_step),
            ],
        ))
    }
}

/// Read `contents` SET — each ref resolves either to an
/// `annotation_curve_occurrence` (`acoc_id_map`) or to an
/// `annotation_occurrence` enum entry (`ao_id_map`). Unresolved refs are
/// silently dropped (per-element drop, the occurrence itself is kept).
fn read_draughting_callout_contents(
    ctx: &ReaderContext,
    content_refs: &[u64],
) -> Vec<DraughtingCalloutElement> {
    let mut contents = Vec::with_capacity(content_refs.len());
    for r in content_refs {
        if let Some(&id) = ctx.annotation_curve_occurrence_id_map.get(r) {
            contents.push(DraughtingCalloutElement::AnnotationCurveOccurrence(id));
        } else if let Some(&id) = ctx.annotation_occurrence_id_map.get(r) {
            contents.push(DraughtingCalloutElement::AnnotationOccurrence(id));
        }
        // else: unmodelled select member (e.g. annotation_fill_area_occurrence)
        // — drop the element.
    }
    contents
}

/// Emit `contents` SET attribute — each `DraughtingCalloutElement`
/// becomes an `EntityRef` into the matching step-id cache.
fn emit_draughting_callout_contents(
    buf: &WriteBuffer,
    contents: &[DraughtingCalloutElement],
) -> Vec<Attribute> {
    let mut refs = Vec::with_capacity(contents.len());
    for elem in contents {
        let step = match elem {
            DraughtingCalloutElement::AnnotationCurveOccurrence(id) => {
                buf.acoc_step_ids[id.0 as usize]
            }
            DraughtingCalloutElement::AnnotationOccurrence(id) => buf.ao_step_ids[id.0 as usize],
        };
        refs.push(Attribute::EntityRef(step));
    }
    refs
}

pub(crate) struct DraughtingCalloutHandler;

/// `DRAUGHTING_CALLOUT(name, contents)` — base variant. The supertype is
/// not abstract in EXPRESS, and fixtures contain many direct
/// instances. Read into `DraughtingCallout::Plain`.
#[step_entity(name = "DRAUGHTING_CALLOUT", pass = Pass7DraughtingCallout)]
impl SimpleEntityHandler for DraughtingCalloutHandler {
    type WriteInput = DraughtingCalloutData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "DRAUGHTING_CALLOUT")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let content_refs = read_entity_ref_list(attrs, 1, entity_id, "contents")?;
        let contents = read_draughting_callout_contents(ctx, &content_refs);
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .draughting_callouts
            .push(DraughtingCallout::Plain(DraughtingCalloutData {
                name,
                contents,
            }));
        ctx.draughting_callout_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: DraughtingCalloutData) -> Result<u64, WriteError> {
        let contents = emit_draughting_callout_contents(buf, &data.contents);
        Ok(buf.push_simple(
            "DRAUGHTING_CALLOUT",
            vec![Attribute::String(data.name), Attribute::List(contents)],
        ))
    }
}

pub(crate) struct LeaderDirectedCalloutHandler;

/// `LEADER_DIRECTED_CALLOUT(name, contents)` — same shape as the base
/// supertype. EXPRESS WHERE narrows `contents` to include a
/// `LEADER_CURVE`; the IR carries the same shape without enforcement.
#[step_entity(name = "LEADER_DIRECTED_CALLOUT", pass = Pass7DraughtingCallout)]
impl SimpleEntityHandler for LeaderDirectedCalloutHandler {
    type WriteInput = DraughtingCalloutData;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 2, entity_id, "LEADER_DIRECTED_CALLOUT")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let content_refs = read_entity_ref_list(attrs, 1, entity_id, "contents")?;
        let contents = read_draughting_callout_contents(ctx, &content_refs);
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .draughting_callouts
            .push(DraughtingCallout::LeaderDirected(DraughtingCalloutData {
                name,
                contents,
            }));
        ctx.draughting_callout_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, data: DraughtingCalloutData) -> Result<u64, WriteError> {
        let contents = emit_draughting_callout_contents(buf, &data.contents);
        Ok(buf.push_simple(
            "LEADER_DIRECTED_CALLOUT",
            vec![Attribute::String(data.name), Attribute::List(contents)],
        ))
    }
}

pub(crate) struct DraughtingCalloutRelationshipHandler;

/// `DRAUGHTING_CALLOUT_RELATIONSHIP(name, description, relating, related)`
/// — pairs two `draughting_callout` instances. Either ref unresolved drops
/// the relationship.
#[step_entity(
    name = "DRAUGHTING_CALLOUT_RELATIONSHIP",
    pass = Pass8DraughtingCalloutRelationship
)]
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
        let Some(&relating) = ctx.draughting_callout_id_map.get(&relating_ref) else {
            return Ok(());
        };
        let Some(&related) = ctx.draughting_callout_id_map.get(&related_ref) else {
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
        let relating = buf.draughting_callout_step_ids[rel.relating.0 as usize];
        let related = buf.draughting_callout_step_ids[rel.related.0 as usize];
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

pub(crate) struct MeasureQualificationHandler;

/// `MEASURE_QUALIFICATION(name, description, qualified_measure, qualifiers)`
/// — `qualified_measure` resolves via `mwu_id_map`; `qualifiers` SET
/// members resolve through `type_qualifier_id_map` /
/// `value_format_type_qualifier_id_map`. The other two `value_qualifier`
/// SELECT members (`precision_qualifier` / `uncertainty_qualifier`)
/// have corpus 0 and are silently dropped (`ApprovalItem` precedent).
#[step_entity(name = "MEASURE_QUALIFICATION", pass = Pass8MeasureQualification)]
impl SimpleEntityHandler for MeasureQualificationHandler {
    type WriteInput = MeasureQualification;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "MEASURE_QUALIFICATION")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let qm_ref = read_entity_ref(attrs, 2, entity_id, "qualified_measure")?;
        let qualifier_refs = read_entity_ref_list(attrs, 3, entity_id, "qualifiers")?;
        let Some(&qualified_measure) = ctx.mwu_id_map.get(&qm_ref) else {
            return Ok(());
        };
        let mut qualifiers = Vec::with_capacity(qualifier_refs.len());
        for r in qualifier_refs {
            if let Some(&id) = ctx.type_qualifier_id_map.get(&r) {
                qualifiers.push(ValueQualifier::TypeQualifier(id));
            } else if let Some(&id) = ctx.value_format_type_qualifier_id_map.get(&r) {
                qualifiers.push(ValueQualifier::ValueFormatTypeQualifier(id));
            }
            // else: precision_qualifier / uncertainty_qualifier (corpus 0,
            // not modelled) — silently drop the SELECT member.
        }
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .measure_qualifications
            .push(MeasureQualification {
                name,
                description,
                qualified_measure,
                qualifiers,
            });
        Ok(())
    }

    fn write(buf: &mut WriteBuffer, mq: MeasureQualification) -> Result<u64, WriteError> {
        let qm_step = buf.mwu_step_ids[mq.qualified_measure.0 as usize];
        let mut qualifier_refs = Vec::with_capacity(mq.qualifiers.len());
        for q in mq.qualifiers {
            let step = match q {
                ValueQualifier::TypeQualifier(id) => buf.type_qualifier_step_ids[id.0 as usize],
                ValueQualifier::ValueFormatTypeQualifier(id) => {
                    buf.value_format_type_qualifier_step_ids[id.0 as usize]
                }
            };
            qualifier_refs.push(Attribute::EntityRef(step));
        }
        Ok(buf.push_simple(
            "MEASURE_QUALIFICATION",
            vec![
                Attribute::String(mq.name),
                Attribute::String(mq.description),
                Attribute::EntityRef(qm_step),
                Attribute::List(qualifier_refs),
            ],
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
#[step_entity(name = "PROJECTED_ZONE_DEFINITION", pass = Pass8ProjectedZoneDefinition)]
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
        let Some(&zone) = ctx.tolerance_zone_id_map.get(&zone_ref) else {
            return Ok(());
        };
        let Some(projection_end) = resolve_shape_aspect_ref(ctx, projection_end_ref) else {
            return Ok(());
        };
        let Some(&projected_length) = ctx.mwu_id_map.get(&projected_length_ref) else {
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
        let zone_step = buf.tolerance_zone_step_ids[pzd.zone.0 as usize];
        let projection_end_step = buf.emit_shape_aspect_ref(pzd.projection_end);
        let projected_length_step = buf.mwu_step_ids[pzd.projected_length.0 as usize];
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
#[step_entity(name = "GEOMETRIC_TOLERANCE_RELATIONSHIP", pass = Pass8GtRelationship)]
impl SimpleEntityHandler for GeometricToleranceRelationshipHandler {
    type WriteInput = GeometricToleranceRelationship;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 4, entity_id, "GEOMETRIC_TOLERANCE_RELATIONSHIP")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = read_string_or_unset(attrs, 1, entity_id, "description")?.to_owned();
        let relating_ref = read_entity_ref(attrs, 2, entity_id, "relating_geometric_tolerance")?;
        let related_ref = read_entity_ref(attrs, 3, entity_id, "related_geometric_tolerance")?;
        let Some(relating) = resolve_geometric_tolerance_ref(ctx, relating_ref) else {
            return Ok(());
        };
        let Some(related) = resolve_geometric_tolerance_ref(ctx, related_ref) else {
            return Ok(());
        };
        ctx.pmi
            .get_or_insert_with(PmiPool::default)
            .geometric_tolerance_relationships
            .push(GeometricToleranceRelationship {
                name,
                description,
                relating,
                related,
            });
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        rel: GeometricToleranceRelationship,
    ) -> Result<u64, WriteError> {
        let relating_step = match rel.relating {
            GeometricToleranceRef::Plain(id) => buf.geometric_tolerance_step_ids[id.0 as usize],
            GeometricToleranceRef::WithDatumReference(id) => {
                buf.geometric_tolerance_with_datum_reference_step_ids[id.0 as usize]
            }
        };
        let related_step = match rel.related {
            GeometricToleranceRef::Plain(id) => buf.geometric_tolerance_step_ids[id.0 as usize],
            GeometricToleranceRef::WithDatumReference(id) => {
                buf.geometric_tolerance_with_datum_reference_step_ids[id.0 as usize]
            }
        };
        Ok(buf.push_simple(
            "GEOMETRIC_TOLERANCE_RELATIONSHIP",
            vec![
                Attribute::String(rel.name),
                Attribute::String(rel.description),
                Attribute::EntityRef(relating_step),
                Attribute::EntityRef(related_step),
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

pub(crate) struct DatumFeatureWriteInput {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) pds_step_id: u64,
    pub(crate) product_definitional: bool,
    pub(crate) kind: crate::ir::DatumFeatureKind,
}

/// `DATUM_FEATURE(name, description, of_shape, product_definitional)` — a
/// `shape_aspect` subtype naming the physical feature realising a datum.
/// Same 4-attr `shape_aspect` body and `of_shape → ProductId` resolution as
/// `SHAPE_ASPECT`; an unresolved `of_shape` drops the datum feature,
/// symmetric on re-read. Registered into `datum_feature_id_map` so a
/// `shape_aspect` ref (e.g. `geometric_tolerance.toleranced_shape_aspect`)
/// resolves onto it through `resolve_shape_aspect_ref`. Shares the arena
/// with the `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE` subtype through
/// [`DatumFeatureKind`](crate::ir::DatumFeatureKind).
#[step_entity(name = "DATUM_FEATURE", pass = Pass8ShapeAspect)]
impl SimpleEntityHandler for DatumFeatureHandler {
    type WriteInput = DatumFeatureWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_datum_feature_variant(
            ctx,
            entity_id,
            attrs,
            "DATUM_FEATURE",
            crate::ir::DatumFeatureKind::Plain,
        )
    }

    fn write(buf: &mut WriteBuffer, input: DatumFeatureWriteInput) -> Result<u64, WriteError> {
        Ok(write_datum_feature(buf, input))
    }
}

pub(crate) struct DimensionalSizeWithDatumFeatureHandler;

/// `DIMENSIONAL_SIZE_WITH_DATUM_FEATURE` — `datum_feature` arena's
/// `in_enum` subtype per the ir.toml blueprint. Shares the 4-attr
/// `shape_aspect` body
/// and the [`DatumFeatureId`](crate::ir::DatumFeatureId) namespace with
/// plain `DATUM_FEATURE`; the kind discriminant captures the subtype.
#[step_entity(name = "DIMENSIONAL_SIZE_WITH_DATUM_FEATURE", pass = Pass8ShapeAspect)]
impl SimpleEntityHandler for DimensionalSizeWithDatumFeatureHandler {
    type WriteInput = DatumFeatureWriteInput;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        read_datum_feature_variant(
            ctx,
            entity_id,
            attrs,
            "DIMENSIONAL_SIZE_WITH_DATUM_FEATURE",
            crate::ir::DatumFeatureKind::DimensionalSizeWithDatumFeature,
        )
    }

    fn write(buf: &mut WriteBuffer, input: DatumFeatureWriteInput) -> Result<u64, WriteError> {
        Ok(write_datum_feature(buf, input))
    }
}

/// Shared 4-attr `shape_aspect` body read + arena push for the
/// `datum_feature` family. Drops the entry when the `of_shape` chain
/// fails to resolve (kernel-built IR / malformed sources).
fn read_datum_feature_variant(
    ctx: &mut ReaderContext,
    entity_id: u64,
    attrs: &[Attribute],
    entity_name: &'static str,
    kind: crate::ir::DatumFeatureKind,
) -> Result<(), ConvertError> {
    check_count(attrs, 4, entity_id, entity_name)?;
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
            kind,
        });
    ctx.datum_feature_id_map.insert(entity_id, id);
    Ok(())
}

/// Shared writer for the `datum_feature` family. Dispatches the STEP
/// entity name on `kind`.
fn write_datum_feature(buf: &mut WriteBuffer, input: DatumFeatureWriteInput) -> u64 {
    let entity_name = match input.kind {
        crate::ir::DatumFeatureKind::Plain => "DATUM_FEATURE",
        crate::ir::DatumFeatureKind::DimensionalSizeWithDatumFeature => {
            "DIMENSIONAL_SIZE_WITH_DATUM_FEATURE"
        }
    };
    let bool_attr = if input.product_definitional { "T" } else { "F" };
    buf.push_simple(
        entity_name,
        vec![
            Attribute::String(input.name),
            Attribute::String(input.description),
            Attribute::EntityRef(input.pds_step_id),
            Attribute::Enum(bool_attr.into()),
        ],
    )
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

/// Push a `GeometricTolerance` into the `pmi` pool and register its
/// `#N → GeometricToleranceId` so `TOLERANCE_ZONE.defining_tolerance` can
/// resolve a `ref_geometric_tolerance` onto it.
fn push_geometric_tolerance(ctx: &mut ReaderContext, entity_id: u64, gt: GeometricTolerance) {
    let id = ctx
        .pmi
        .get_or_insert_with(PmiPool::default)
        .geometric_tolerances
        .push(gt);
    ctx.geometric_tolerance_id_map.insert(entity_id, id);
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
    ctx.geometric_tolerance_with_datum_reference_id_map
        .insert(entity_id, id);
}

/// Resolve a `ref_geometric_tolerance` (`TOLERANCE_ZONE.defining_tolerance`)
/// to a [`GeometricToleranceRef`] — step-io splits geometric tolerances
/// across the form-tolerance and datum-referencing arenas.
pub(crate) fn resolve_geometric_tolerance_ref(
    ctx: &ReaderContext,
    item_ref: u64,
) -> Option<GeometricToleranceRef> {
    if let Some(&id) = ctx.geometric_tolerance_id_map.get(&item_ref) {
        return Some(GeometricToleranceRef::Plain(id));
    }
    ctx.geometric_tolerance_with_datum_reference_id_map
        .get(&item_ref)
        .copied()
        .map(GeometricToleranceRef::WithDatumReference)
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
        modifiers: Vec::new(),
        unit_size: None,
        defined_area_unit: None,
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
    let has_unit_size = data.unit_size.is_some();
    let has_area_unit = data.defined_area_unit.is_some();
    let has_modifiers = !data.modifiers.is_empty();
    if !has_unit_size && !has_area_unit && !has_modifiers {
        return buf.push_simple(
            entity_name,
            vec![
                Attribute::String(data.name),
                Attribute::String(data.description),
                Attribute::EntityRef(magnitude),
                Attribute::EntityRef(shape_aspect),
            ],
        );
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
    if let Some(unit_size_id) = data.unit_size {
        let unit_size_step = buf.mwu_step_ids[unit_size_id.0 as usize];
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
/// resolves through the writer's `mwu_step_ids` cache (None → `$`).
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
    let second = match area_unit.second_unit_size {
        Some(id) => Attribute::EntityRef(buf.mwu_step_ids[id.0 as usize]),
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
        push_geometric_tolerance(ctx, entity_id, GeometricTolerance::Flatness(data));
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
        push_geometric_tolerance(ctx, entity_id, GeometricTolerance::Straightness(data));
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
        push_geometric_tolerance(ctx, entity_id, GeometricTolerance::Roundness(data));
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
        push_geometric_tolerance(ctx, entity_id, GeometricTolerance::Cylindricity(data));
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
#[allow(clippy::too_many_arguments)]
fn build_gt_with_datum_reference_data(
    ctx: &ReaderContext,
    name: String,
    description: String,
    magnitude_ref: u64,
    shape_aspect_ref: u64,
    datum_system_refs: &[u64],
    modifiers: Vec<crate::ir::GeometricToleranceModifier>,
    displacement: Option<crate::ir::id::MeasureWithUnitId>,
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
/// `ref_measure_with_unit`. Returns `None` when the part is absent or
/// the ref does not resolve through `mwu_id_map`.
fn read_optional_unit_size(
    ctx: &ReaderContext,
    parts: &[RawEntityPart],
    entity_id: u64,
) -> Result<Option<crate::ir::id::MeasureWithUnitId>, ConvertError> {
    let Some(attrs) = find_part_attrs(parts, "GEOMETRIC_TOLERANCE_WITH_DEFINED_UNIT") else {
        return Ok(None);
    };
    check_count(attrs, 1, entity_id, "GEOMETRIC_TOLERANCE_WITH_DEFINED_UNIT")?;
    let unit_ref = read_entity_ref(attrs, 0, entity_id, "unit_size")?;
    Ok(ctx.mwu_id_map.get(&unit_ref).copied())
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
        Some(Attribute::EntityRef(n)) => ctx.mwu_id_map.get(n).copied(),
        _ => None,
    };
    Ok(Some(crate::ir::DefinedAreaUnit {
        area_type,
        second_unit_size,
    }))
}

/// Read the `UNEQUALLY_DISPOSED_GEOMETRIC_TOLERANCE.displacement` part —
/// `ref_length_measure_with_unit`. Returns `None` when the part is
/// absent or the ref does not resolve through `mwu_id_map`.
fn read_optional_displacement(
    ctx: &ReaderContext,
    parts: &[RawEntityPart],
    entity_id: u64,
) -> Result<Option<crate::ir::id::MeasureWithUnitId>, ConvertError> {
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
    Ok(ctx.mwu_id_map.get(&unit_ref).copied())
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
    let Some(toleranced_shape_aspect) = resolve_shape_aspect_ref(ctx, shape_aspect_ref) else {
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
        if let Some(disp_id) = data.displacement {
            let disp_step = buf.mwu_step_ids[disp_id.0 as usize];
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
    pass = Pass8GtWithDatumReference,
    required = ["FLATNESS_TOLERANCE"]
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
    pass = Pass8GtWithDatumReference,
    required = ["ROUNDNESS_TOLERANCE"]
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
    pass = Pass8GtWithDatumReference,
    required = ["STRAIGHTNESS_TOLERANCE"]
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
    pass = Pass8GtWithDatumReference,
    required = ["PARALLELISM_TOLERANCE"]
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
    pass = Pass8GtWithDatumReference,
    required = ["PERPENDICULARITY_TOLERANCE"]
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
    pass = Pass8GtWithDatumReference,
    required = ["CIRCULAR_RUNOUT_TOLERANCE"]
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

pub(crate) struct DraughtingModelItemAssociationHandler;

#[step_entity(name = "DRAUGHTING_MODEL_ITEM_ASSOCIATION", pass = Pass8Dmia)]
impl SimpleEntityHandler for DraughtingModelItemAssociationHandler {
    type WriteInput = DraughtingModelItemAssociation;

    fn read(
        ctx: &mut ReaderContext,
        entity_id: u64,
        attrs: &[Attribute],
        _graph: &EntityGraph,
    ) -> Result<(), ConvertError> {
        check_count(attrs, 5, entity_id, "DRAUGHTING_MODEL_ITEM_ASSOCIATION")?;
        let name = read_string_or_unset(attrs, 0, entity_id, "name")?.to_owned();
        let description = match &attrs[1] {
            Attribute::Unset => None,
            Attribute::String(s) => Some(s.clone()),
            _ => return Ok(()),
        };
        let def_ref = read_entity_ref(attrs, 2, entity_id, "definition")?;
        let definition = if let Some(&id) = ctx.repr_id_map.get(&def_ref) {
            DraughtingModelItemDefinition::Representation(id)
        } else if let Some(&id) = ctx.composite_shape_aspect_id_map.get(&def_ref) {
            DraughtingModelItemDefinition::CompositeShapeAspect(id)
        } else if let Some(&id) = ctx.dimensional_size_id_map.get(&def_ref) {
            DraughtingModelItemDefinition::DimensionalSize(id)
        } else if let Some(&id) = ctx.shape_aspect_id_map.get(&def_ref) {
            DraughtingModelItemDefinition::ShapeAspect(id)
        } else if let Some(&id) = ctx.datum_feature_id_map.get(&def_ref) {
            DraughtingModelItemDefinition::DatumFeature(id)
        } else if let Some(&id) = ctx.property_def_step_to_id.get(&def_ref) {
            DraughtingModelItemDefinition::PropertyDefinition(id)
        } else if let Some(&id) = ctx.dimensional_location_id_map.get(&def_ref) {
            DraughtingModelItemDefinition::DimensionalLocation(id)
        } else if let Some(gt_ref) = resolve_geometric_tolerance_ref(ctx, def_ref) {
            // geometric_tolerance member — Plain or WithDatumReference complex MI.
            DraughtingModelItemDefinition::GeometricTolerance(gt_ref)
        } else {
            ctx.warnings.push(ConvertError::UnexpectedEntityForm {
                entity_id,
                detail: format!(
                    "DRAUGHTING_MODEL_ITEM_ASSOCIATION definition #{def_ref} \
                     resolves to none of the 8 modelled SELECT members — skipping"
                ),
            });
            return Ok(());
        };
        let used_ref = read_entity_ref(attrs, 3, entity_id, "used_representation")?;
        let Some(&used_representation) = ctx.repr_id_map.get(&used_ref) else {
            return Ok(());
        };
        let item_ref = read_entity_ref(attrs, 4, entity_id, "identified_item")?;
        let identified_item = if let Some(&id) = ctx.annotation_occurrence_id_map.get(&item_ref) {
            DraughtingModelIdentifiedItem::AnnotationOccurrence(id)
        } else if let Some(&id) = ctx.draughting_callout_id_map.get(&item_ref) {
            DraughtingModelIdentifiedItem::DraughtingCallout(id)
        } else {
            return Ok(());
        };
        let id = ctx
            .pmi
            .get_or_insert_with(PmiPool::default)
            .draughting_model_item_associations
            .push(DraughtingModelItemAssociation {
                name,
                description,
                definition,
                used_representation,
                identified_item,
            });
        ctx.dmia_id_map.insert(entity_id, id);
        Ok(())
    }

    fn write(
        buf: &mut WriteBuffer,
        dmia: DraughtingModelItemAssociation,
    ) -> Result<u64, WriteError> {
        let def_step = match dmia.definition {
            DraughtingModelItemDefinition::Representation(id) => {
                buf.representation_step_ids[id.0 as usize]
            }
            DraughtingModelItemDefinition::CompositeShapeAspect(id) => {
                buf.composite_shape_aspect_step_ids[id.0 as usize]
            }
            DraughtingModelItemDefinition::DimensionalSize(id) => {
                buf.dimensional_size_step_ids[id.0 as usize]
            }
            DraughtingModelItemDefinition::ShapeAspect(id) => {
                buf.shape_aspect_step_ids[id.0 as usize]
            }
            DraughtingModelItemDefinition::DatumFeature(id) => {
                buf.datum_feature_step_ids[id.0 as usize]
            }
            DraughtingModelItemDefinition::PropertyDefinition(id) => {
                buf.property_definition_step_ids[id.0 as usize]
            }
            DraughtingModelItemDefinition::DimensionalLocation(id) => {
                buf.dimensional_location_step_ids[id.0 as usize]
            }
            DraughtingModelItemDefinition::GeometricTolerance(r) => match r {
                GeometricToleranceRef::Plain(id) => buf.geometric_tolerance_step_ids[id.0 as usize],
                GeometricToleranceRef::WithDatumReference(id) => {
                    buf.geometric_tolerance_with_datum_reference_step_ids[id.0 as usize]
                }
            },
        };
        let used_step = buf.representation_step_ids[dmia.used_representation.0 as usize];
        let item_step = match dmia.identified_item {
            DraughtingModelIdentifiedItem::AnnotationOccurrence(id) => {
                buf.ao_step_ids[id.0 as usize]
            }
            DraughtingModelIdentifiedItem::DraughtingCallout(id) => {
                buf.draughting_callout_step_ids[id.0 as usize]
            }
        };
        let description_attr = match dmia.description {
            Some(s) => Attribute::String(s),
            None => Attribute::Unset,
        };
        Ok(buf.push_simple(
            "DRAUGHTING_MODEL_ITEM_ASSOCIATION",
            vec![
                Attribute::String(dmia.name),
                description_attr,
                Attribute::EntityRef(def_step),
                Attribute::EntityRef(used_step),
                Attribute::EntityRef(item_step),
            ],
        ))
    }
}
