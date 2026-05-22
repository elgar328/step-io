use super::arena::{Arena, define_id};
use super::assembly::{
    Product, ProductContext, ProductDefinitionContext, ProductDefinitionContextAssociation,
    ProductDefinitionContextRole, ProductDefinitionRelationship,
};
use super::form_features::Step;
use super::geometry::{
    Axis1Placement, Axis2Placement2d, Axis2Placement3d, Curve, Curve2d, Direction2, Direction3,
    PlanarExtent, Point2, Point3, Surface, Vertex,
};
use super::plm::{
    Address, ApplicationContext, ApplicationProtocolDefinition, AppliedDocumentReference,
    AppliedExternalIdentificationAssignment, AppliedGroupAssignment, Approval, ApprovalAssignment,
    ApprovalDateTime, ApprovalPersonOrganization, ApprovalRole, ApprovalStatus, CalendarDate,
    CoordinatedUniversalTimeOffset, DateAndTime, DateAndTimeAssignment, DateTimeRole, Document,
    DocumentProductEquivalence, DocumentRepresentationType, DocumentType, ExternalSource, Group,
    IdentificationRole, LocalTime, ObjectRole, Organization, Person, PersonAndOrganization,
    PersonAndOrganizationAssignment, PersonAndOrganizationRole, RoleAssociation,
    SecurityClassification, SecurityClassificationAssignment, SecurityClassificationLevel,
};
use super::pmi::{
    AnnotationOccurrence, Datum, DatumFeature, DimensionalLocation, DimensionalSize,
    DraughtingPreDefinedTextFont, GeometricTolerance, ToleranceZoneForm, TypeQualifier,
    ValueFormatTypeQualifier,
};
use super::property::{
    DescriptionAttribute, GeneralProperty, GeneralPropertyAssociation, IdAttribute, NameAttribute,
    Property,
};
use super::shape_rep::{
    AllAroundShapeAspect, CentreOfSymmetry, CompositeGroupShapeAspect, MappedItem,
    NumericRepresentationItem, Representation, RepresentationMap, ShapeAspect,
    ShapeAspectRelationship, UnitContext,
};
use super::tessellation::{
    ComplexTriangulatedFace, ComplexTriangulatedSurfaceSet, TessellatedItem,
};
use super::topology::{Edge, Face, Shell, Solid, Wire};
use super::units::{DerivedUnit, DerivedUnitElement, MeasureWithUnit, NamedUnit};
use super::visualization::{
    CameraModel, Colour, CurveFont, CurveStyle, FoundedItem, PresentationLayerAssignment,
    PresentationStyleAssignment, StyledItem, SurfaceStyleRendering,
};

// Geometry Ids (3D)
define_id!(PointId, Point3);
define_id!(DirectionId, Direction3);
define_id!(SurfaceId, Surface);
define_id!(CurveId, Curve);
define_id!(Placement3dId, Axis2Placement3d);
define_id!(Placement1dId, Axis1Placement);
define_id!(PlanarExtentId, PlanarExtent);

// Geometry Ids (2D — PCURVE parametric space)
define_id!(Point2dId, Point2);
define_id!(Direction2dId, Direction2);
define_id!(Curve2dId, Curve2d);
define_id!(Placement2dId, Axis2Placement2d);

// Topology Ids
define_id!(VertexId, Vertex);
define_id!(EdgeId, Edge);
define_id!(WireId, Wire);
define_id!(FaceId, Face);
define_id!(ShellId, Shell);
define_id!(SolidId, Solid);

// Assembly Ids
define_id!(ProductId, Product);
define_id!(ProductContextId, ProductContext);
define_id!(ProductDefinitionContextId, ProductDefinitionContext);
define_id!(ProductDefinitionContextRoleId, ProductDefinitionContextRole);
define_id!(
    ProductDefinitionContextAssociationId,
    ProductDefinitionContextAssociation
);
define_id!(
    ProductDefinitionRelationshipId,
    ProductDefinitionRelationship
);
define_id!(FeatureDefinitionId, Step);
define_id!(NameAttributeId, NameAttribute);
define_id!(DescriptionAttributeId, DescriptionAttribute);
define_id!(IdAttributeId, IdAttribute);

// Property Ids — `Property` (PD+PDR+REP collapsed) arena + the AP242
// user-defined-attribute pair (Phase property-3).
define_id!(PropertyId, Property);
define_id!(GeneralPropertyId, GeneralProperty);
define_id!(GeneralPropertyAssociationId, GeneralPropertyAssociation);

// Unit context Ids — multi-context support (one entry per
// REPRESENTATION_CONTEXT in the source file).
define_id!(UnitContextId, UnitContext);

// PMI Ids — anchor for future Tolerance / Datum / GD&T work.
define_id!(ShapeAspectId, ShapeAspect);

// SHAPE_ASPECT subtype Ids — distinct arena per ir.toml blueprint.
define_id!(CompositeShapeAspectId, CompositeGroupShapeAspect);
define_id!(DerivedShapeAspectId, CentreOfSymmetry);
define_id!(ContinuousShapeAspectId, AllAroundShapeAspect);

// SHAPE_ASPECT_RELATIONSHIP arena (phase shape-aspect-ref).
define_id!(ShapeAspectRelationshipId, ShapeAspectRelationship);

// pmi pool Ids — tolerance / qualifier primitives (Phase pmi-primitives).
define_id!(ToleranceZoneFormId, ToleranceZoneForm);
define_id!(TypeQualifierId, TypeQualifier);
define_id!(ValueFormatTypeQualifierId, ValueFormatTypeQualifier);

// annotation_occurrence enum_base arena (Phase annotation-plane).
define_id!(AnnotationOccurrenceId, AnnotationOccurrence);

// datum arena (Phase datum).
define_id!(DatumId, Datum);

// datum_feature arena (Phase datum-feature).
define_id!(DatumFeatureId, DatumFeature);

// dimensional_size arena (Phase dimensional-size).
define_id!(DimensionalSizeId, DimensionalSize);

// dimensional_location arena (Phase dimensional-location).
define_id!(DimensionalLocationId, DimensionalLocation);

// geometric_tolerance enum_base arena (Phase geometric-tolerance).
define_id!(GeometricToleranceId, GeometricTolerance);

// draughting_pre_defined_text_font arena (Phase text-font).
define_id!(DraughtingPreDefinedTextFontId, DraughtingPreDefinedTextFont);

// REPRESENTATION arena — unified subtype storage (representation-refactor).
define_id!(RepresentationId, Representation);

// REPRESENTATION_MAP + MAPPED_ITEM arenas (phase mapped-item).
define_id!(RepresentationMapId, RepresentationMap);
define_id!(MappedItemId, MappedItem);

// representation_item value-items (phase numeric-representation-item).
define_id!(NumericRepresentationItemId, NumericRepresentationItem);

// tessellation arenas (phase tessellation / tessellation-2).
define_id!(TessellatedItemId, TessellatedItem);
define_id!(TessellatedFaceId, ComplexTriangulatedFace);
define_id!(TessellatedSurfaceSetId, ComplexTriangulatedSurfaceSet);

// Visualization Ids — Colour enum arena (ColourRgb + DraughtingPreDefinedColour).
define_id!(ColourId, Colour);

// Visualization Ids — CURVE_STYLE chain.
define_id!(CurveFontId, CurveFont);
define_id!(CurveStyleId, CurveStyle);

// Visualization Ids — STYLED_ITEM enum arena (Plain + future variants).
define_id!(StyledItemId, StyledItem);

// Visualization Ids — PRESENTATION_STYLE_ASSIGNMENT enum arena
// (Itself + PresentationStyleByContext).
define_id!(PresentationStyleAssignmentId, PresentationStyleAssignment);

// Visualization Ids — SURFACE_STYLE_RENDERING enum arena
// (Itself + SurfaceStyleRenderingWithProperties).
define_id!(SurfaceStyleRenderingId, SurfaceStyleRendering);

// Visualization Ids — founded_item enum arena (AP214 founded-item
// supertype; E1 covers FillAreaStyle + SurfaceStyleFillArea).
define_id!(FoundedItemId, FoundedItem);

// Visualization Ids — camera_model enum arena (phase camera-model-d3).
define_id!(CameraModelId, CameraModel);

// Visualization Ids — PRESENTATION_LAYER_ASSIGNMENT arena (top-level;
// no other entity refs it, the id exists for blueprint symmetry).
define_id!(PresentationLayerAssignmentId, PresentationLayerAssignment);

// plm Ids — Date/Time primitives (Phase plm-1a).
define_id!(DateId, CalendarDate);
define_id!(LocalTimeId, LocalTime);
define_id!(
    CoordinatedUniversalTimeOffsetId,
    CoordinatedUniversalTimeOffset
);
define_id!(DateAndTimeId, DateAndTime);
define_id!(DateTimeRoleId, DateTimeRole);
define_id!(DateAndTimeAssignmentId, DateAndTimeAssignment);
define_id!(PersonId, Person);
define_id!(OrganizationId, Organization);
define_id!(PersonAndOrganizationId, PersonAndOrganization);
define_id!(PersonAndOrganizationRoleId, PersonAndOrganizationRole);
define_id!(
    PersonAndOrganizationAssignmentId,
    PersonAndOrganizationAssignment
);

// plm Ids — Approval primitives + linkers (Phase plm-3a).
define_id!(ApprovalStatusId, ApprovalStatus);
define_id!(ApprovalRoleId, ApprovalRole);
define_id!(ApprovalId, Approval);
define_id!(ApprovalDateTimeId, ApprovalDateTime);
define_id!(ApprovalPersonOrganizationId, ApprovalPersonOrganization);
define_id!(ApprovalAssignmentId, ApprovalAssignment);

// plm Ids — Security cluster (Phase plm-4).
define_id!(SecurityClassificationLevelId, SecurityClassificationLevel);
define_id!(SecurityClassificationId, SecurityClassification);
define_id!(
    SecurityClassificationAssignmentId,
    SecurityClassificationAssignment
);

// plm Ids — Identification cluster (Phase plm-5).
define_id!(IdentificationRoleId, IdentificationRole);
define_id!(ExternalSourceId, ExternalSource);
define_id!(
    IdentificationAssignmentId,
    AppliedExternalIdentificationAssignment
);

// plm Ids — Document cluster (Phase plm-6).
define_id!(DocumentTypeId, DocumentType);
define_id!(DocumentId, Document);
define_id!(DocumentRepresentationTypeId, DocumentRepresentationType);
define_id!(DocumentProductEquivalenceId, DocumentProductEquivalence);
define_id!(DocumentReferenceId, AppliedDocumentReference);

// plm Ids — Group cluster (Phase plm-7).
define_id!(GroupId, Group);
define_id!(GroupAssignmentId, AppliedGroupAssignment);

// plm Ids — Role cluster (Phase plm-8).
define_id!(ObjectRoleId, ObjectRole);
define_id!(RoleAssociationId, RoleAssociation);

// plm Ids — Address cluster (Phase plm-9).
define_id!(AddressId, Address);

// plm Ids — Application cluster (Phase plm-10).
define_id!(ApplicationContextId, ApplicationContext);
define_id!(
    ApplicationProtocolDefinitionId,
    ApplicationProtocolDefinition
);

// units pool Ids (Phase units-1) — per-instance `NAMED_UNIT` complexes,
// `MEASURE_WITH_UNIT` subtypes, and `DERIVED_UNIT_ELEMENT`.
define_id!(NamedUnitId, NamedUnit);
define_id!(MeasureWithUnitId, MeasureWithUnit);
define_id!(DerivedUnitElementId, DerivedUnitElement);
define_id!(DerivedUnitId, DerivedUnit);
