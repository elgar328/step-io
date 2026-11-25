# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `read()` recognizes AP242 edition 4 in `FILE_SCHEMA`: module version 7
  (`{ 1 0 10303 442 7 1 4 }`) as edition 4 IS, versions 5 and 6 as edition 4
  of an unknown stage.
- The model reads more entities within step-io's support boundary, each
  with an `Ap242Author` constructor: the round, composite and counter-bored,
  -drilled or -sunk hole features (`ROUND_HOLE`, `BASIC_ROUND_HOLE`,
  `EXPLICIT_ROUND_HOLE`, `COMPOSITE_HOLE`, `EXPLICIT_COMPOSITE_HOLE`,
  `COUNTERBORE_HOLE_DEFINITION`, `COUNTERDRILL_HOLE_DEFINITION`,
  `COUNTERSINK_HOLE_DEFINITION`, `ROUND_HOLE_BOTTOM_CONDITION`,
  `COMPOUND_FEATURE`, `FEATURE_DEFINITION`),
  `POSITIVE_LENGTH_MEASURE_WITH_UNIT`, `POSITIVE_PLANE_ANGLE_MEASURE_WITH_UNIT`,
  `EXTERNAL_IMAGE_PLACEMENT_IN_CALLOUT` and
  `ACTION_RESOURCE_REQUIREMENT_RELATIONSHIP`. `ROUND_HOLE_BOTTOM_CONDITION`,
  `EXTERNAL_IMAGE_PLACEMENT_IN_CALLOUT` and
  `ACTION_RESOURCE_REQUIREMENT_RELATIONSHIP` are new in edition 4; the others
  are older, and come in because edition 4 references them. `EntityKey`, the
  reference enums that admit them, and `PartialEntity` for those that can be
  part of a complex instance gain arms for them:
  - the hole features: `CharacterizedDefinitionRef`,
    `DocumentReferenceItemRef` and
    `ItemIdentifiedRepresentationUsageDefinitionRef`
  - the positive measures: `MeasureWithUnitRef`, `LengthMeasureWithUnitRef`
    (the length one), `LengthOrPlaneAngleMeasureWithUnitSelectRef`,
    `SizeSelectRef` and `GroupableItemRef`
  - `ExternalImagePlacementInCallout`: `RepresentationItemRef` and the
    enums that list representation items, such as `StyledItemRef`,
    `LayeredItemRef`, `InvisibleItemRef` and `AnnotationOccurrenceRef`
  - `RoundHoleBottomCondition`: `IdAttributeSelectRef`;
    `ActionResourceRequirementRelationship`:
    `CharacterizedResourceDefinitionRef`
- `Ap242Author` constructors for what edition 4 allows and edition 2 did not:
  `add_apll_point`, `add_apll_point_with_surface`,
  `add_annotation_placeholder_occurrence_with_leader_line`,
  `add_annotation_to_annotation_leader_line`,
  `add_annotation_to_model_leader_line` and `add_auxiliary_leader_line`.
- A styled item may style a set of items (`SET_REPRESENTATION_ITEM`), as AP242
  edition 4 allows: `StyledItemTargetOrRepresentationItemRef` (below) has a
  `SetRepresentationItem` arm, and a colour styled onto a set reaches each of
  its items.
- `GeometricToleranceModifier` gains edition 4's values: `ContinuousFeatures`,
  `DerivedFeature`, `EachElement`, `Individually`, `OffsetZone`,
  `SeparateZones`, `Stock`, `UnspecifiedAngularToleranceZoneOffset`,
  `UnspecifiedLinearToleranceZoneOffset` and `VariableAngle`.
- Reference enums with an aggregate arm
  (`StyledItemTargetOrRepresentationItemRef`, `DatumOrCommonDatumRef`,
  `CompoundItemDefinitionRef`, `TransformationRef`,
  `ItemIdentifiedRepresentationUsageSelectRef`) have `resolve()`, which views
  the aggregate as its elements; `entity_key()` stays with enums whose every
  arm is one entity.
- `AuthorError::NarrowedReference`: a reference names a type that AP242 does
  not allow in an attribute it narrows. It is raised where the argument's
  type cannot rule that out: an argument that is a complex instance, a part
  given to `add_complex`, and the struct a self-referencing constructor
  (`add_…_cyclic`) takes, which has the model's types.
- `Ap242ApprovedItemRef`, `Ap242ConstructiveGeometryRepresentationRef`,
  `Ap242DerivedShapeAspectRef`, `Ap242DraughtingModelItemSelectRef`,
  `Ap242AdvancedBrepShapeRepresentationItemsSelectRef`,
  `Ap242AdvancedFaceGeometrySelectRef`, `Ap242FaceOrSurfaceRef`,
  `Ap242ManifoldSurfaceShapeRepresentationSelectRef`,
  `Ap242MechanicalDesignGeometricPresentationRepresentationItemsRef`,
  `Ap242MechanicalDesignShadedPresentationRepresentationItemsRef`,
  `Ap242ShapeDimensionRepresentationItemRef`,
  `Ap242ShapeRepresentationWithParametersItemsRef`,
  `Ap242CompoundItemDefinitionRef`, `Ap242DatumOrCommonDatumRef` and
  `Ap242TransformationRef`: what AP242 edition 4 allows where the model, read
  from every edition, allows more. Each converts into the model's reference
  with `From`, as do `PropertyDefinitionRef` into `RepresentedDefinitionRef`,
  `ShapeRepresentationRef` into `RepresentationRef`, `ShapeModelRef` into
  `RepresentationOrRepresentationReferenceRef`, and
  `LengthMeasureWithUnitRef` and `LengthOrPlaneAngleMeasureWithUnitSelectRef`
  into `MeasureWithUnitRef`.
- `Ap242AnnotationOccurrenceAssociativityRelatedAnnotationOccurrenceRef`,
  `Ap242BoundedCurveRef`, `Ap242BoundedSurfaceCurveCurve3dRef`,
  `Ap242CameraUsageMappedRepresentationRef`,
  `Ap242ConstructiveGeometryRepresentationItemsRef`,
  `Ap242GeometricCurveSetElementsRef`,
  `Ap242GeometricallyBoundedSurfaceShapeRepresentationItemsRef`,
  `Ap242GeometricallyBoundedWireframeShapeRepresentationItemsRef`,
  `Ap242GlobalUnitAssignedContextRef`, `Ap242LeaderCurveRef`,
  `Ap242LengthUnitRef`, `Ap242MassUnitRef`,
  `Ap242MeasureQualificationQualifiedMeasureRef`,
  `Ap242OrientedClosedShellClosedShellElementRef`,
  `Ap242OrientedEdgeEdgeElementRef`,
  `Ap242ParametricRepresentationContextRef`, `Ap242PlaneAngleUnitRef`,
  `Ap242ProductDefinitionOccurrenceDefinitionRef`,
  `Ap242ProductDefinitionShapeDefinitionRef`,
  `Ap242ProductDefinitionUsageRef`, `Ap242RatioUnitRef`,
  `Ap242StyledItemItemRef`, `Ap242StyledItemItemSetRepresentationItemRef`
  and `Ap242SurfaceCurveCurve3dRef`: what an AP242 edition 4 `WHERE` rule
  lets an attribute, or each element of a set it holds, reference. Each
  converts into the model's reference with `From`, as does
  `GeometricRepresentationContextRef` into `RepresentationContextRef`.
- `StyledItemTargetOrRepresentationItemRef`: what a styled item's `item` takes
  in the model — AP242's `styled_item_target` or, as AP203 and AP214 allow,
  any representation item.

### Changed

- Output is AP242 edition 4 (ISO 10303-242:2025): `Ap242Author::finish` and
  `StepBuilder` stamp `FILE_SCHEMA` `{ 1 0 10303 442 7 1 4 }`, and
  `StepBuilder` writes the application protocol definition's year as 2025.
  The constructors follow edition 4's types and `WHERE` rules; the entries
  below say what changes for them.
- The model reads the union of seven editions, AP242 edition 4 included. A
  complex instance is allowed wherever the EXPRESS rules let the entity
  combine in any edition, so `PartialEntity` gains a variant for each entity
  some edition lets combine (82 that were modeled already, such as
  `CartesianPoint`, `Circle` and `Line`), and `CartesianPointRef`,
  `Axis1PlacementRef`, `Axis2PlacementRef`, `Axis2Placement3dRef`,
  `CoordinatesListRef`, `DatumReferenceCompartmentRef`,
  `DatumReferenceElementRef`, `UncertaintyMeasureWithUnitRef`,
  `TrimmingSelectRef`, `VertexLoopRef`, `ToleranceSelectRef`,
  `SymbolTargetRef`, `DesApllPointSelectRef`,
  `AnnotationPlaceholderLeaderLineRef`, `AnnotationToModelLeaderLineRef` and
  `CameraModelD3MultiClippingIntersectionSelectRef` gain a `Complex` arm.
  `GeometricToleranceTargetRef` gains edition 4's shape aspect relationships
  (`ShapeAspectRelationship`, `ShapeAspectAssociativity`,
  `ShapeAspectDerivingRelationship`, `FeatureForDatumTargetRelationship`).
  The scene reads a complex point, mesh coordinate list or model uncertainty
  from its parts, and reports a complex placement or datum reference it
  cannot follow as a warning.
- `Ap242Author` constructors take AP242 edition 4's narrower reference type
  where the model's is wider, and store it as the model's:
  - `add_shape_definition_representation`: `definition` takes a
    `PropertyDefinitionRef`, `used_representation` a
    `ShapeRepresentationRef`
  - `add_advanced_brep_shape_representation` and
    `add_manifold_surface_shape_representation`: `items` takes the matching
    `Ap242…SelectRef`
  - `add_advanced_face`: `face_geometry` takes an
    `Ap242AdvancedFaceGeometrySelectRef` (an elementary, B-spline or swept
    surface)
  - `add_complex_triangulated_face`: `geometric_link` takes an
    `Ap242FaceOrSurfaceRef` (a face surface or a surface, where edition 2
    allowed any face)
  - `add_compound_representation_item`: `item_element` takes an
    `Ap242CompoundItemDefinitionRef` (a list or set of items, not one alone);
    `add_datum_reference_compartment` and `add_datum_reference_element`:
    `base` takes an `Ap242DatumOrCommonDatumRef` (a datum reference element
    only in a list)
  - `add_draughting_model`,
    `add_mechanical_design_geometric_presentation_representation`,
    `add_mechanical_design_shaded_presentation_representation`,
    `add_shape_dimension_representation` and
    `add_shape_representation_with_parameters`: `items` takes the matching
    `Ap242…Ref`
  - `add_shape_aspect_deriving_relationship`: `relating_shape_aspect` takes an
    `Ap242DerivedShapeAspectRef`
  - `add_constructive_geometry_representation_relationship`: `rep_1` takes a
    `ShapeModelRef`, `rep_2` an `Ap242ConstructiveGeometryRepresentationRef`
  - `add_cc_design_approval`: `items` takes `Ap242ApprovedItemRef`s
  - `add_representation_relationship_with_transformation`:
    `transformation_operator` takes an `Ap242TransformationRef` (not a
    `list_item_defined_transformation`, which edition 2 allowed)
- `Ap242Author` constructors also take a narrower reference type where an
  AP242 edition 4 `WHERE` rule limits the types an attribute, each of its
  elements, or each element of a set it holds may reference, whether for
  every instance or by the instance's own type (such as
  `'LENGTH_UNIT' IN TYPEOF(SELF\measure_with_unit.unit_component)` or
  `DRAUGHTING_ANNOTATION_OCCURRENCE.WR7`). Rules on one indexed element, on
  two attributes together or on a path through another entity are not
  checked. The parts of a complex argument count together: it is rejected on
  insertion with `AuthorError::NarrowedReference` if a part is of a type the
  rule excludes, or if the rule asks for exactly one of several types and its
  parts are of more than one of them:
  - `add_styled_item`, `add_over_riding_styled_item`,
    `add_context_dependent_over_riding_styled_item`,
    `add_annotation_occurrence` and `add_draughting_annotation_occurrence`:
    `item` takes an `Ap242StyledItemItemRef`: a mapped item or a geometric
    item that is not a styled item, or a set of mapped and geometric items
    (`STYLED_ITEM.WR2`, `WR3`; edition 4 no longer exempts a
    context-dependent over-riding styled item)
  - `add_length_measure_with_unit`, `add_positive_length_measure_with_unit`,
    `add_mass_measure_with_unit`, `add_plane_angle_measure_with_unit`,
    `add_positive_plane_angle_measure_with_unit` and
    `add_ratio_measure_with_unit`: `unit_component` takes the matching
    `Ap242…UnitRef`
  - `add_product_definition_occurrence`: `definition` takes an
    `Ap242ProductDefinitionOccurrenceDefinitionRef`
  - `add_constructive_geometry_representation`: `items` takes an
    `Ap242ConstructiveGeometryRepresentationItemsRef`. Edition 4's
    `CONSTRUCTIVE_GEOMETRY_REPRESENTATION.WR2` asks each item to be exactly
    one of a list that names supertypes with their subtypes, which would
    leave out a cartesian point, a line, a circle or a plane; this is a known
    issue of the edition, and the rule is read as asking for at least one
  - `add_geometrically_bounded_surface_shape_representation` and
    `add_geometrically_bounded_wireframe_shape_representation`: `items`
    takes the matching `Ap242…ItemsRef`
  - `add_product_definition_shape`: `definition` takes an
    `Ap242ProductDefinitionShapeDefinitionRef` (a product definition or its
    relationship, or a characterized object; not a shape aspect)
  - `add_constructive_geometry_representation`,
    `add_definitional_representation` and
    `add_tessellated_shape_representation`: `context_of_items` takes a
    `GeometricRepresentationContextRef`,
    `Ap242ParametricRepresentationContextRef` and
    `Ap242GlobalUnitAssignedContextRef` respectively
  - `add_surface_curve`, `add_intersection_curve` and `add_seam_curve`:
    `curve_3d` takes an `Ap242SurfaceCurveCurve3dRef` (not a pcurve);
    `add_bounded_surface_curve` an `Ap242BoundedSurfaceCurveCurve3dRef`;
    `add_composite_curve_segment`: `parent_curve` an `Ap242BoundedCurveRef`
  - `add_oriented_edge` and `add_oriented_closed_shell`: `edge_element` and
    `closed_shell_element` take a type that is not itself oriented
  - `add_configuration_effectivity` (`usage`), `add_geometric_curve_set`
    (`elements`), `add_leader_terminator` (`annotated_curve`),
    `add_annotation_occurrence_associativity`
    (`related_annotation_occurrence`), `add_measure_qualification`
    (`qualified_measure`) and `add_camera_usage` (`mapped_representation`):
    the matching `Ap242…Ref`
- `add_complex` and the self-referencing constructors check the same
  narrowings at run time, each element of a set included. In `add_complex`,
  a rule that depends on the instance's own type is checked against its
  parts: `DRAUGHTING_ANNOTATION_OCCURRENCE.WR7` (exactly one composite text
  or text literal) applies where a part is an annotation text occurrence.
- `add_complex` rejects a part of an entity AP242 declares `ABSTRACT` unless
  a part of one of its subtypes is beside it, with
  `AuthorError::InvalidComplex`.
- Attributes that every STEP edition narrows with a redeclaration
  (`SELF\supertype.attribute : narrower;`) now have the narrower type in the
  model, so both reading and `Ap242Author` take only what the schema allows
  there:
  - `PresentationArea`, `PresentationRepresentation` and `PresentationView`
    `context_of_items`: `GeometricRepresentationContextRef`
  - `ModelGeometricView` and `DefaultModelGeometricView` `rep`:
    `DraughtingModelRef`; `item`: `CameraModelRef`
  - `CameraImage` and `CameraImage3dWithScale` `mapping_source`:
    `CameraUsageRef`; `mapping_target`: `PlanarBoxRef`
  - `CameraUsage::mapping_origin`: `CameraModelRef`
  - `MechanicalDesignPresentationRepresentationWithDraughting::items`:
    `Vec<CameraModelD3Ref>`
  - `DraughtingModelItemAssociationWithPlaceholder` `used_representation`:
    `DraughtingModelRef`; `identified_item`: `DraughtingCalloutRef`
  - `FeatureForDatumTargetRelationship::related_shape_aspect`:
    `DatumTargetRef`
  - `AnnotationPlaceholderOccurrence(WithLeaderLine)::item`:
    `GeometricSetRef`; `TessellatedAnnotationOccurrence::item`:
    `TessellatedGeometricSetRef`
- Attributes an older STEP edition declares wider than AP242 now have the
  wider type in the model, so reading keeps what any edition allows; the
  `Ap242Author` constructors still take AP242's type:
  - `magnitude` of every geometric tolerance (and of the
    `GEOMETRIC_TOLERANCE` part): `Option<MeasureWithUnitRef>`, as AP203 and
    AP214 declare it, where AP242 takes a length measure
  - `GeometricToleranceWithDefinedUnit` and
    `GeometricToleranceWithDefinedAreaUnit` `unit_size`, and
    `ProjectedZoneDefinition::projected_length`: `MeasureWithUnitRef`
  - `item` of `StyledItem`, `OverRidingStyledItem`,
    `ContextDependentOverRidingStyledItem`, `AnnotationOccurrence` and
    `DraughtingAnnotationOccurrence` (and of the `STYLED_ITEM` part):
    `StyledItemTargetOrRepresentationItemRef`
  - `ConstructiveGeometryRepresentationRelationship::rep_1`:
    `RepresentationOrRepresentationReferenceRef`, as
    `representation_relationship` declares it; `Ap242Author` still takes
    AP242's `ShapeModelRef`

### Removed

- The constructors of the entities AP242 edition 4 declares `ABSTRACT`,
  which exist only as part of an instance of a subtype:
  `add_action_assignment`, `add_action_request_assignment`,
  `add_approval_assignment`, `add_camera_model`,
  `add_date_and_time_assignment`, `add_document_product_association`,
  `add_document_reference`, `add_expression`,
  `add_external_identification_assignment`, `add_founded_item`,
  `add_general_datum_reference`, `add_generic_expression`,
  `add_generic_literal`, `add_generic_product_definition_reference`,
  `add_geometric_tolerance`, `add_group_assignment`,
  `add_identification_assignment`, `add_literal_number`,
  `add_numeric_expression`, `add_person_and_organization_assignment`,
  `add_presented_item`, `add_security_classification_assignment`,
  `add_simple_generic_expression`, `add_simple_numeric_expression`,
  `add_tessellated_face`, `add_tessellated_item`,
  `add_tessellated_surface_set`, `add_texture_style_specification` and
  `add_texture_style_tessellation_specification`. Use a subtype's
  constructor, or `add_complex` with a subtype part beside the abstract one.
  `GENERIC_PRODUCT_DEFINITION_REFERENCE`, `TEXTURE_STYLE_SPECIFICATION` and
  `TEXTURE_STYLE_TESSELLATION_SPECIFICATION` can no longer be authored:
  step-io models none of their subtypes that are not themselves `ABSTRACT`.
- `ANNOTATION_FILL_AREA_OCCURRENCE` and `ANNOTATION_SYMBOL` are no longer
  modeled. Every edition requires them to reference types step-io does not
  model (`ANNOTATION_FILL_AREA`, `SYMBOL_REPRESENTATION_MAP`), so a standard
  instance was already dropped when read. Their model types, the
  `add_annotation_fill_area_occurrence`, `add_annotation_symbol` and
  `add_annotation_symbol_cyclic` constructors, and their variants in
  `EntityKey`, `PartialEntity` and the reference enums that listed them are
  gone.
- `StyledItemTargetRef`, replaced by `StyledItemTargetOrRepresentationItemRef`
  in the model and `Ap242StyledItemItemRef` in the constructors.
- `ConstructiveGeometryRepresentationOrShapeRepresentationRef`, replaced by
  `RepresentationOrRepresentationReferenceRef` in the model and
  `ShapeModelRef` in `add_constructive_geometry_representation_relationship`.
- `add_oriented_edge_cyclic`, `add_oriented_closed_shell_cyclic`,
  `add_product_definition_shape_cyclic`,
  `add_product_definition_occurrence_cyclic`,
  `add_annotation_placeholder_occurrence_cyclic`, `add_camera_image_cyclic`
  and `add_camera_image3d_with_scale_cyclic`: AP242's types and `WHERE` rules
  do not let these entities reference themselves.

### Fixed

- `read()` drops an entity whose reference, in one of the attributes every
  STEP edition narrows (listed under Changed), names a type the narrower
  attribute does not allow, and reports it as a `NonstandardReference`. Such
  an entity used to be kept, with a reference the standard does not allow.
- The same holds inside a complex entity: when one part narrows an attribute
  another part declares, and every edition shares that narrowing, `read()`
  drops the entity if the reference falls outside it (for example
  `(PRESENTATION_REPRESENTATION REPRESENTATION)` over a context that is not
  geometric).
- `read()` drops an entity whose reference points at a complex entity none
  of whose parts the schema allows there, and reports it as a
  `NonstandardReference`, as it already did for a simple entity of the wrong
  type. Such an entity used to be kept, and the reference went missing from
  the scene without a report: a face's surface read as `Other`, or a loop or
  edge was left out.
- `read()` keeps a geometric tolerance whose magnitude or unit size, or a
  projected zone whose projected length, is a plain `MEASURE_WITH_UNIT`, and
  a styled item that styles a representation item outside AP242's
  `styled_item_target`, as AP203 and AP214 allow. These used to be dropped as
  a `NonstandardReference`.
- `Tolerance::magnitude` reads a magnitude given as a plain
  `MEASURE_WITH_UNIT` when its value is a length measure.
- The scene reads a measure written as a complex instance, such as
  `(LENGTH_MEASURE_WITH_UNIT() MEASURE_REPRESENTATION_ITEM()
  MEASURE_WITH_UNIT(LENGTH_MEASURE(5.), #u) REPRESENTATION_ITEM(''))`, from
  its `MEASURE_WITH_UNIT` part: `Dimension::value`, `Tolerance::magnitude`,
  a curve width, and a conversion-based unit's factor. `Dimension::value` and
  `Tolerance::magnitude` used to give `None` for such a measure, which is how
  NIST's test files write them. `Scene::units` also reads a conversion factor
  given as any other subtype of `MEASURE_WITH_UNIT`, such as a
  `POSITIVE_LENGTH_MEASURE_WITH_UNIT`.
- `Ap242Author` checks the size of a list held in a SELECT, and rejects one
  outside its bounds with `AuthorError::Cardinality`: a `COMMON_DATUM_LIST`
  needs at least two elements, a `LIST_REPRESENTATION_ITEM`,
  `SET_REPRESENTATION_ITEM` or `SET_ITEM_DEFINED_TRANSFORMATION` at least
  one. It used to check only the elements.
- `add_person_and_organization_address` rejects `organizations` or `people`
  that do not hold exactly one element with `AuthorError::Cardinality`, as
  AP242 requires; it used to accept more than one.

## [0.3.1] - 2026-11-18

### Added

- `parser::stream()` reads a file one raw entity at a time, in file order.
  The `Stream` it returns is an iterator of entities and shows the header,
  schema, `ANCHOR` and `REFERENCE` sections up front and the parser's
  warnings so far; any error, including a file that does not end as Part 21
  requires, is its last item. It holds one entity at a time, so apart from
  the warnings it collects, the memory it takes does not grow with the file.
  Unlike `parser::parse()`, it does not check that each id appears once.

### Changed

- `parser::parse()` gathers the file's entities in one list on the way to
  the `Graph`, where it used to fill two, so parsing a large file takes less
  memory at its peak.

## [0.3.0] - 2026-10-21

0.3.0 reshapes the public API: each item has one path, the model is read-only
outside the crate, reading and writing stream, and many scene and builder
methods take or return different types. Changed and Removed below say what to
write instead.

### Added

- `Orientation` (`Forward` or `Reversed`) says which way a face, a face's
  boundary loop, an edge in a loop, or a shell in a solid is used. Reading
  and authoring both use it, so an orientation read from a file can be
  passed straight to `StepBuilder`.
- `Report::parse_warnings` lists what the parser repaired or skipped while
  reading, such as a blank attribute or an undecodable string escape. `read()`
  used to discard these.
- `ParseWarning` implements `Display`, printing a one-line message with the
  line and column, and is re-exported as `step_io::ParseWarning`.
- `ParseWarning` gains two warnings for strings: `MalformedStringEscape`, for
  an escape that cannot be decoded and is kept as written, and
  `LowercaseHexEscape`, for a hex escape with lower-case digits, which is
  decoded all the same.
- `PARABOLA` is read and written. A file using one used to lose the curve,
  and with it the edges and faces built on it; Open CASCADE writes a
  parabola wherever a plane cuts a cone parallel to a side.
- `CurveKind::Parabola` gives a parabola with its focal distance.
- `to_nurbs()` converts parabolas and hyperbolas exactly: the span of an
  edge, a trimmed curve or a composite curve's segment becomes a quadratic
  B-spline for a parabola and rational quadratic pieces for a hyperbola. A
  trim given as a point is followed rather than its parameter: writers
  disagree on a parabola's parameter, Open CASCADE writing `2F·u` where the
  standard has `u`.
- `to_nurbs()` converts a `DEGENERATE_TOROIDAL_SURFACE` — a torus whose tube
  crosses its axis — exactly: only the part its `select_outer` picks, the
  outer lobe (`.T.`) or the inner spindle (`.F.`), on the torus's own
  parametrization, so a face's `same_sense` keeps its meaning.
- `to_nurbs()` converts an `OFFSET_SURFACE` exactly when its basis is a
  plane, cylinder, cone, sphere or torus: the offset is the same kind of
  surface — a plane moved along its normal, the others with their radius
  changed — and a face on one converts as a face on that surface would. An
  offset that would turn the surface inside out or grow a torus tube past
  the axis is not converted, nor is an offset of a free-form surface, which
  has no exact rational form.
- `Surface::basis_surface()` gives the surface an `OFFSET_SURFACE` is offset
  from, so a kernel can build the offset from it and the distance.
- `Shell` is a shell of a solid or of a surface model, closed or open
  (`is_closed()`). `Solid::outer()` gives the outer shell and
  `Solid::voids()` each cavity's shell, both with the `Orientation` the
  solid uses the shell in: `Reversed` when it is reached through an
  `ORIENTED_CLOSED_SHELL` with `.F.`, as Open CASCADE writes every cavity,
  while other writers keep `.T.`. A face in a shell points the way its own
  orientation and the shell's combine to. A shell reaches back to its
  `solid()` or `surface_model()` and has colour, transparency, layer and
  visibility; `Face::shell()` gives the shell listing a face.
- `SurfaceModel` is a surface body (`SHELL_BASED_SURFACE_MODEL`) with its
  `shells()`, each with the `Orientation` the model uses it in, its `faces()`,
  and colour, transparency, layer and visibility. `Scene::all_surface_models()`
  lists them, and `ProductDef::surface_models()` and
  `MappedInstance::surface_models()` reach them from a part, through manifold
  surface representations and the representations bridged to them;
  `ProductDef::solids()` and `MappedInstance::solids()` now follow the same
  representations.
- `GeometricSet` is a `GEOMETRIC_SET` or `GEOMETRIC_CURVE_SET` — a wireframe,
  a sketch, a set of points kept outside any b-rep. Its `elements()` are
  `SetElement`s: a `Curve`, a `Point`, or anything else by key. It has colour,
  layer and visibility. `Scene::all_geometric_sets()` lists them, and
  `ProductDef::geometric_sets()` and `MappedInstance::geometric_sets()` reach
  them from a part.
- Presentation on more handles: `Edge` reads the curve style applied to the
  `EDGE_CURVE` itself (`color()`, `width()`, `line_font()`), `Point` reads a
  point style's marker colour, `Mesh` and `MeshGroup` read colour and
  transparency, and these, together with `Curve`, have `layer()` and
  `is_visible()`.
- `FeatureGeometry::Mesh` and `FeatureGeometry::GeometricSet` give a feature's
  display mesh or geometric set, which used to come back as `Other`.
- `Bound::vertex()` gives the vertex of a `VERTEX_LOOP` boundary, such as a
  cone's apex, which has no edges.
- `Curve::curve_3d()` reaches the 3D curve of a `SURFACE_CURVE`,
  `SEAM_CURVE`, `BOUNDED_SURFACE_CURVE` or `INTERSECTION_CURVE` as a `Curve`;
  this used to take the raw model.
- Every scene handle has a `key()` — `Contributor` and `Approver` gain one —
  and implements `Debug`, printing its type and key.
  `FeatureGeometry` and `Target` implement `Debug` as well, and
  `FeatureGeometry` is `Clone` and `Copy`.
- `Version::product()` reaches the product a version belongs to.
- `EntityKey::keyword()` gives the STEP keyword of the entity type a key
  names, such as `ADVANCED_FACE`, or `None` for a complex instance.
- `CurveInput::Parabola` and `CurveInput::Hyperbola` let `StepBuilder::edge`
  write an edge along a parabola or a hyperbola branch. Its `same_sense` says
  whether the edge runs with the curve's parameter, found from the vertices;
  a zero focal distance, a semi-axis that is not positive, or vertices at the
  same parameter are refused with `AuthorError::InvalidInput`.
- `StepBuilder::part_ids()` gives the `PRODUCT`, version, `PRODUCT_DEFINITION`
  and `PRODUCT_DEFINITION_SHAPE` the builder made for a part (`PartIds`), so
  that what the builder does not cover, such as a property or a shape
  aspect, can be attached through `author()`.
- The `Ap242Author` constructors, and `add_complex` for the parts of a complex
  instance, check the size of a list nested in an aggregate as they check
  the aggregate's: a row of B-spline control points or weights, a triangle
  strip or fan, a line strip. A row of one control point is refused with
  `AuthorError::Cardinality`, and so is a triangle strip of two points.

### Changed

- `read()` and `parser::parse()` take any `std::io::Read` — a `File`, a
  `&[u8]`, … — and read it a chunk at a time, so the file is never held in
  memory. They used to take the whole file, `read()` as bytes and `parse()`
  as a `&str`: pass `File::open(path)?` instead of `&std::fs::read(path)?`,
  and `text.as_bytes()` for text already in memory. A failing input is the
  new `Error::Io`, which holds the input's `std::io::Error`.
- `read()` streams the file instead of holding all of it as raw entities
  until the model is built, and takes less than half as long: a 110 MB file
  of one million entities now reads in 0.7 s instead of 1.7 s, and the
  process peaks at about 115 MB of memory instead of 760 MB (with the
  smaller model described below). It reads the same entities as before.
- The `Graph` that `parse()` returns takes about 40% less memory, 300 MB
  instead of 480 MB for the 110 MB file above, as its attribute lists are
  allocated at their exact length.
- Each entity, with the blanks and comments before it, reads as UTF-8 when
  its bytes are valid UTF-8 and as ISO 8859-1 otherwise; the whole file used
  to read one way or the other. A UTF-8 file with one broken character now
  reads the rest of its text correctly instead of reading all of it as
  ISO 8859-1. In ISO 8859-1 text a `Span` now counts the file's bytes; it
  used to count those of a decoded copy.
- Section markers (`HEADER`, `DATA`, `ENDSEC`, …) ignore ASCII case only, so
  `ENDſEC` no longer reads as `ENDSEC`.
- Strings are decoded when read: `\X2\…\X0\`, `\X4\…\X0\`, `\X\HH`,
  `\S\` and `\\` become the characters they stand for, and line breaks that
  writers insert to wrap long strings are dropped. `parse()` and `read()`
  return the decoded text, so code that decoded these escapes itself should
  stop doing so.
- A file whose `FILE_SCHEMA` names no schema reads as `ApFamily::Other`; it
  used to read as AP214, which was `ApFamily`'s default. `ApFamily` has no
  default now.
- `SchemaId::raw` is the `Vec<String>` of `FILE_SCHEMA` names, empty when the
  file gives none.
- `SchemaId` prints only what the file pins, `AP214` or `AP214 (CD)` rather
  than `AP214 ed? (?)`, and an unrecognised schema prints its names rather
  than `unrecognized schema`.
- `Report`'s counts are `entities_read`, `entities_added` and
  `entities_kept` (were `n_in`, `n_synth` and `validated`), and
  `DropReason::key` is `DropReason::detail`. `DropKind` and `DropReason`
  implement `Display` (`cascade: via-dangling`).
- `Report::normalized` (was `norm`) lists each note with the id of the
  entity it fixed, in the order of the entities in the file; it used to list
  the notes alone, the entity-specific fixes first and the generic ones
  after, each in id order. The entity-specific notes read as sentences like
  the others (`missing surface colour set to an unspecified COLOUR`).
- `Report::dropped` is sorted by entity id.
- An entity dropped because it references an entity that normalization
  dropped now carries that entity's reason in its cascade label, such as
  `via-required reference is missing`, instead of `via-dangling`, which now
  means only a reference to an id the file does not define.
- `Error::DuplicateEntityId` gives the `line` of the repeat and the
  `first_line` of the entity it repeats instead of a `span`.
- `Error` no longer implements `Clone` and `PartialEq`, as `Error::Io` holds
  an `std::io::Error`; match on its variants instead. Its message includes
  a wrapped lexical or input error's, and `source()` is `None`, so an error
  chain prints each message once.
- `Error::UnexpectedToken` names the token `found` as a `String`, about as
  written in the file (`FOO`, `')'`, `#12`), instead of a `TokenKind`, and
  the message quotes a punctuation mark it expected (`expected ';'`).
- A string the file never closes is `LexErrorKind::UnterminatedString` and
  a malformed binary literal `InvalidBinary`; both used to be
  `UnexpectedCharacter`.
- `Span::start` and `Span::end` are `u64`, so offsets past 4 GB stay exact on
  32-bit targets.
- These types are `#[non_exhaustive]`, so later variants and fields can be
  added without a breaking release: the enums `Error`, `LexErrorKind`,
  `ParseWarning`, `AuthorError`, `DropKind`, `ApFamily`, `Stage`,
  `CurveKind`, `SurfaceKind`, `FeatureKind`, `ToleranceKind` and
  `FeatureGeometry`, which a `match` covers with a wildcard arm, and the
  structs `Report`, `DropReason`, `LexError`, `SchemaId`, `FileHeader`,
  `parser::Graph`, `Units`, `Unit` and `ApprovalDate`, whose fields read as
  before but which can no longer be built with a struct literal outside
  step-io.
- Each public item has one path. The generated model is `step_io::model`
  (was `step_io::generated::model`, and `generated::resolve` for the
  `…RefView` types). The `reader`, `emit`, `header`, `refgraph` and
  `generated` modules are private: what they held is at the crate root.
  `scene`'s submodules are private and all their types are at `scene::`,
  such as `scene::Face` (was `scene::geometry::Face`) and `scene::ProductDef`
  (was `scene::product::ProductDef`). `build` no longer re-exports
  `NurbsCurve`, `NurbsSurface` and `Rgb`; use `scene::`. `StepBuilder`,
  `StepModel` and `EntityKey` stay at the crate root as well as in their
  modules.
- The `parser` module keeps what reading a raw file needs: `parse`, `Graph`
  and its entity types, the `Span` that errors carry, and
  `MAX_NESTING_DEPTH`. `Error`, `LexError`, `LexErrorKind`, `ParseWarning`,
  `SchemaId`, `ApFamily` and `Stage` are at the crate root only.
- The model is read-only outside the crate. Its per-type entity lists
  (the `…_arena` fields) and `Arena` are no longer public; read the model
  with `model.all::<T>()`, which lists the entities of type `T` with their
  ids (was `model.t_arena.items`), and `model.get(id)` or `model[id]` (was
  `model.t_arena.get(id.0)`), through the new `Entity` and `EntityId`
  traits. New entities enter only through the `Ap242Author` constructors,
  which check them; pushing to an arena skipped the checks, and a model
  changed that way could make `write` panic or write a non-conforming file.
- Entity ids are 32-bit: each `…Id` wraps a `u32`, and references and
  `EntityKey` take 8 bytes instead of 16. A file of more than about four
  billion entities fails to read with the new `Error::TooLarge`.
- String fields of the generated model are `Text`s: indices into the
  model's text table, which stores each distinct string once instead of
  allocating every field (a typical file repeats a few hundred distinct
  strings, such as `NONE`, across a million fields). Read one with
  `model.text(e.name)` instead of `&e.name`; the scene handles' `name()` and
  the like still return `&str`. `MeasureValue`, `StringSelectValue` and
  `IntMeasureValue` hold `Text`s too; for values built directly, such as
  complex parts, make the `Text` with the new `Ap242Author::text`.
- A list of at most three reals — a point's `coordinates`, a direction's
  `direction_ratios`, a mesh vertex or normal — is a `Reals3` held inline
  instead of a `Vec<f64>`, so it no longer allocates. It reads as a `&[f64]`;
  build one from an array with `.into()` or from a slice with
  `Reals3::from_slice`. `Ap242Author` takes `Reals3` for these fields and
  now also checks the length of each inner list, such as a mesh vertex.
- On read, such a list with more than three values is dropped as a
  nonstandard value (`list has more values than its type allows`).
- The types of a complex instance take Part 21's names: `ComplexEntity`
  (was `ComplexUnit`) and its `ComplexEntityId`, `PartialEntity` (was
  `UnitPart`) for each entity type it combines, and `EntityKey::ComplexEntity`.
  They hold every complex instance, not only units.
- `StepModel::ref_graph()`, which the scene handles build on first use,
  holds its index in two flat arrays instead of a map of lists: for a model
  of one million entities it takes 16 MB instead of 97 MB and builds in
  13 ms instead of 67 ms. `RefGraph::referrers()` returns the same entities
  in the same order.
- A hyperbola's `CurveKind` is `Hyperbola`, with its semi-axes, instead of
  `Other("HYPERBOLA")`.
- A degenerate torus's `SurfaceKind` is `DegenerateToroidal` instead of
  `Other("DEGENERATE_TOROIDAL_SURFACE")`.
- An offset surface's `SurfaceKind` is `Offset` instead of
  `Other("OFFSET_SURFACE")`.
- `Face::same_sense()` is replaced by `Face::orientation()`, and
  `Bound::orientation()` and `Bound::oriented_edges()` return `Orientation`
  instead of `bool`. `Edge::same_sense()`, which relates an edge to its curve,
  stays a `bool`.
- `Solid::voids()` returns an iterator of `(Shell, Orientation)` pairs, one
  per cavity, instead of a `Vec` of face lists. It used to drop the
  orientation, so a cavity's faces could not be told to point into the
  hole. Use `shell.faces()` for the faces.
- `Scene::dimensions()`, `tolerances()`, `datums()` and `features()` are
  renamed `all_dimensions()`, `all_tolerances()`, `all_datums()` and
  `all_features()`, as every other model-wide list is named `all_*`. The
  same names on a `ProductDef` stay as they are.
- Every method that leads from a handle to any number of other handles
  returns an iterator. These used to return a `Vec`: `MeshGroup::meshes`,
  `Product::{contributors, approvals, documents, security_classifications,
  versions}`, `ProductDef::{contributors, approvals, documents,
  security_classifications}`, `assigned_targets` on `Contributor`,
  `Approval`, `Document` and `SecurityClassification`, `Approval::approvers`,
  `MappedInstance::{solids, mapped_children}`, `Tolerance::datums`,
  `Dimension::features` and `Feature::geometry`. The order is the same; add
  `.collect::<Vec<_>>()` where a list is needed.
- `ProductDef::version()` returns the `Version` handle instead of its id
  string; call `.id()` on it for the label.
- `Scene::warnings()` returns `SceneWarning`s instead of strings. Each names
  the entity it concerns — the handle whose method ran into it — beside the
  message, and prints as the old string did, so `{w}` formatting is
  unchanged; read `.message` for the text. Asking the same handle again no
  longer repeats a warning.
- `StepBuilder::new()`, `part()`, `part_with()`, `person_and_org()`,
  `document()`, `security()` and `finish()` return their value instead of a
  `Result`: nothing they are given can break a rule, so they could fail only
  through a bug in step-io, and panic then. Drop the `?` or `.unwrap()`, as
  in `let part = b.part("wheel");`. `StepBuilder` implements `Default`.
- `StepBuilder::face` takes an `Orientation` instead of the `same_sense`
  boolean, and `FaceBoundInput` holds an `Orientation` in each edge pair and
  in its `orientation` field instead of a boolean. The file written is
  unchanged. Replace `true` with `Orientation::Forward` and `false` with
  `Orientation::Reversed`.
- `StepBuilder` holds the file header as a `FileHeader` from the start,
  stamped with the time and `step-io <version>`; read it with `header()` and
  fill it through `header_mut()`. `HeaderInput` and `StepBuilder::header(&…)`
  are removed: write `b.header_mut().file_name = "wheel.step".into();` for
  `b.header(&HeaderInput { file_name: Some("wheel.step".into()), .. })`, and
  `time_stamp = String::new()` for a blank stamp. A header read from another
  file can be passed in whole.
- `FileHeader::descriptions` keeps each `FILE_DESCRIPTION` entry, as
  `authors` and `organizations` do; `description` used to join them into
  one string.
- `StepBuilder::hide_layer()` refuses a layer id this builder does not hold
  with `AuthorError::DanglingRef`, and so returns a `Result`; `finish()` used
  to catch it.
- `StepBuilder::place` refuses a placement that would make an assembly
  contain itself, the part placed being the assembly or already containing
  it, with `AuthorError::CyclicAssembly`. The standard forbids such cycles,
  and a reader following one may never stop.
- `StepBuilder` refuses a NURBS curve or surface that breaks the standard's
  B-spline rules (degree at least 1, more control points than the degree,
  knot count, knots that never decrease, knot repeats, one positive weight
  per control point) and a mesh without triangles, with an index past its
  points, or with per-vertex normals that do not match the points in number,
  with the new `AuthorError::InvalidInput`.
- `StepBuilder` writes `APPLICATION_PROTOCOL_DEFINITION` with year 2020 (AP242
  edition 2) instead of 2011. Readers ignore the value; only the output text
  changes.
- The `Ap242Author` constructors take their strings as `&str`, an optional
  one as `Option<&str>`, and `add_person`'s name lists as `Option<&[&str]>`,
  instead of `String`s: write `a.add_product("p1", "part", None, …)` rather
  than `"p1".to_owned()`.
- The `Ap242Author` constructors check that a complex instance given for a
  reference is of a fitting type: a unit is refused where a curve belongs,
  with `AuthorError::InvalidComplex`. They used to check only that it
  existed.
- `StepBuilder::finish()`, `Ap242Author::finish()` and
  `Ap242Author::finish_with_header()` return the finished `StepModel`
  instead of its text. Write it with
  `step_io::write(&b.finish(), File::create(path)?)?`, or into a
  `Vec<u8>` to get the text.
- `write()` takes the output — a `File`, a `&mut Vec<u8>`, … — and writes
  the text to it 64 KB at a time instead of returning it as a `String`, so
  the text is never held in memory: writing a 90 MB file of one million
  entities takes about 30 MB on top of the model instead of 310 MB, in the
  same time. The output is flushed at the end, so a `File` needs no
  `BufWriter`, and a failing output is returned as an `std::io::Error`.
- Strings are written in plain ASCII, as Part 21 requires: every character
  other than printable ASCII becomes a `\X2\` or `\X4\` escape, and `\`
  becomes `\\`. A string of printable ASCII with no backslash is written as
  before.
- Documentation and the crate description present step-io as STEP I/O for
  3D CAD applications rather than for CAD kernels.

### Removed

- The `step_io::Result` alias; write `Result<T, step_io::Error>`.
- `SchemaId::synthetic()`, the `SchemaId::raw()` method (read the `raw`
  field) and `NonEmptyStringList`.
- From `parser`: `parse_bytes` (use `parse`), `Parser`, `Lexer`, `tokenize`,
  `Token`, `TokenKind`, `Span::slice`, `schema::identify_schema` (a file's
  schema is `model.header().schema`, or `Graph::schema` from `parse`), and
  the `entity`, `lexer`, `p21` and `schema` submodules.
- The generated enums' `parse` and `token` and the reference enums'
  `from_any`. `resolve` and `entity_key` stay.

### Fixed

- step-io builds on Rust 1.85, the minimum supported Rust version it declares.
  Versions 0.1.0 through 0.2.5 used a Rust 1.88 feature and did not build on
  1.85.
- `Occurrence::transform()` places components correctly in assemblies whose
  placement relationship lists the assembly's representation first, as
  SolidWorks writes them. It used to return the inverse placement there.
  `from` is now always the frame in the component and `to` the frame in the
  assembly. A placement whose order cannot be decided is read component-first
  and reported in `Scene::warnings`.
- `StepBuilder` writes assembly placements the way the standard requires.
  Each part placed in an assembly gets a geometric context of its own, the
  frame a component is placed at is an item of the assembly's
  representation, and the transformation starts from the component's
  origin. Placements land where they did before, and a file without
  placements is written unchanged.
- `to_nurbs()` no longer panics or runs out of memory on broken or hostile
  values in a file. It returns `None`, with a warning in
  `Scene::warnings`, for a B-spline of degree 0 or above 25, one with fewer
  control points than its degree needs, one whose knot multiplicities do not
  add up to its control points, a circle or ellipse trimmed at angles too
  large to subtract, and a B-spline trimmed outside its parameter range.
- A circle or ellipse trimmed more than a full turn apart converts to the
  arc within one turn, as Open CASCADE reads it. It used to wind round as
  many times as the angles said, and a trim like `1.E17` never finished.
- `Scene::root_definitions()` no longer lists a
  `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS` that an assembly uses as a
  component; it only checked plain `PRODUCT_DEFINITION` components, so such a
  part showed both under its assembly and as a root of its own.
- `Product::contributors()`, `approvals()`, `documents()` and
  `security_classifications()` reach metadata on a
  `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS` definition, and
  `documents()` the documents such a definition lists itself, as the same
  methods on a `ProductDef` already did; they used to look only at plain
  `PRODUCT_DEFINITION`s.
- `Feature::geometry()` returns a `FACE_SURFACE` as a `Face` and a
  `BREP_WITH_VOIDS` as a `Solid`, and `Mesh::face()` reaches a linked
  `FACE_SURFACE`. They used to give `Other` and `None`.
- `StepBuilder` and `Ap242Author` refuse a NaN or infinite real with the new
  `AuthorError::NonFiniteReal`. They used to write it as `NaN` or `inf`,
  which Part 21 has no spelling for, so the file could not be read back.
- The `Ap242Author` `add_*_cyclic` constructors check aggregate sizes, as
  the other constructors do; an empty `SET [1:?]` used to pass.
- `Face::to_nurbs()` reads a cone's half-angle in the file's plane-angle unit.
  It used to take it as radians, so a cone in a file that measures angles in
  degrees came out with the wrong radius.
- `Face::to_nurbs()` converts a cone face that closes at its apex, bounded by
  a circle and a `VERTEX_LOOP` at the apex or by the circle alone. It used to
  return `None` there for a degenerate extent.
- `read()` reports a Part 21 edition 3 `ANCHOR` or `REFERENCE` section in
  `Report::parse_warnings` (`ParseWarning::Ed3SectionDiscarded`), as it
  already did `SIGNATURE`; the model has no place for them, and they used to
  be dropped without a word. `parser::parse()` keeps them in its `Graph` as
  before.
- A face boundary that is a `POLY_LOOP`, a bare `LOOP` or a complex instance
  is reported in `Scene::warnings`; `Bound::edges()` used to come back empty
  without a word.
- `MeshGroup::solid()` finds the solid of a tessellated shell linked to a
  closed shell that the solid reaches through an `ORIENTED_CLOSED_SHELL`, a
  cavity included, as `Face::solid()` does. It used to find only an outer
  shell referenced directly.

## [0.2.5] - 2026-10-03

### Added

- `CurveInput::LineAlong(direction)` authors a straight edge along a unit
  direction the caller already holds, written bit for bit, so a kernel that
  knows the exact direction keeps it instead of the one two rounded vertex
  positions imply. A direction pointing from the end vertex back to the start
  is negated so the line runs with the edge.

### Changed

- A straight edge (`CurveInput::Line`) is written with a `VECTOR` magnitude of
  1 instead of the edge's length. The magnitude is only the line's parameter
  scale — the edge is bounded by its vertices — so readers see the same shape,
  and the value no longer carries the rounding of a length computed from two
  `f64` points. `ProfileInput::Line` already wrote 1.

## [0.2.4] - 2026-07-09

### Changed

- `StepBuilder::solid_with_voids()` now takes a `VoidShellNormals` argument
  declaring which way the void faces' normals point: `AwayFromMaterial` keeps
  them as authored (`ORIENTED_CLOSED_SHELL(..., .T.)`), `TowardMaterial`
  reverses them (`.F.`). This lets kernels pass cavity shells in their native
  orientation instead of the fixed reversal 0.2.3 applied.

## [0.2.3] - 2026-07-09

### Added

- `StepBuilder::solid_with_voids()` authors a solid with internal cavities
  (`BREP_WITH_VOIDS`), such as a hollow casting. Void solids read back like any
  other solid: `Scene::all_solids()`, `Face::solid()`, and `ProductDef::solids()`
  include them, `Solid::voids()` returns the faces of each cavity shell, and
  `StyleTarget::VoidSolid` colours, layers, and hides them.

### Changed

- `StepBuilder::mesh()` now takes `Option<SolidRef>` for the solid a display mesh
  tessellates (was `Option<ManifoldSolidBrepId>`), so a mesh can link to a plain
  solid or a `BREP_WITH_VOIDS`. Existing calls migrate with `Some(id.into())`.

## [0.2.2] - 2026-07-06

### Added

- `Datum::datum_feature()` — the physical `DATUM_FEATURE` a datum is established
  from, as a `Feature` (so its faces/edges are reachable via `Feature::geometry`).

### Changed

- `Report` drop reasons are now plain-language. `DropKind::SlotLocal`/`Nonstandard`
  are renamed to `NonstandardValue`/`NonstandardReference` (a nonstandard attribute
  *value* vs. a nonstandard *reference*), and the per-reason labels read as
  sentences (e.g. `required reference is missing`) instead of terse codes
  (`req-ref<-$`).

### Fixed

- The scene now surfaces `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS` (the
  `PRODUCT_DEFINITION` subtype used for a part that carries documents) as a
  `ProductDef`, so such a part appears in the assembly tree with its geometry
  placed — instead of the part vanishing and its solids orphaning — and its
  documents/approvals/security are reachable.
- A `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS` part now reports its PMI —
  `ProductDef::{features, dimensions, tolerances, datums}` resolve for the
  documented-part subtype as they do for a plain `PRODUCT_DEFINITION`, instead of
  always returning empty.

## [0.2.1] - 2026-07-06

### Fixed

- `ProductDef::solids()` now follows `SHAPE_REPRESENTATION_RELATIONSHIP`
  bridges, so a part reports its solids even when the geometry lives in a
  separate `ADVANCED_BREP_SHAPE_REPRESENTATION` — the common AP242 assembly
  layout, where those solids were previously left unattached.

## [0.2.0] - 2026-07-06

### Added

- The full Part 21 HEADER is read into the model: `model.header()` carries
  the file name, timestamp, authors, the originating CAD system, and the
  identified schema — one `FileHeader` type, shared with the write side.
- `write(&model)` — serialize a model to Part 21 text under its own header,
  losslessly.

### Changed

- The identified source schema moved from `Report.schema` to
  `model.header().schema`.

### Removed

- The write-side projection surface — `write_target`, `SchemaTarget`, and
  `LossReport`. The authoring API is AP242 by construction, so its output
  never needed projecting.
- The generated read/write plumbing (`generated::{read, write, walk,
  generic_normalize, schema}`) is crate-internal now; `generated::{model,
  resolve, author}` remain the public raw layer.

## [0.1.0] - 2026-07-04

### Added

- Read every mainstream application protocol — AP203, AP214, and AP242, legacy
  included — and write modern AP242 edition 2 (IS).
- Navigate what you read with `model.scene()` — lightweight handles over b-rep
  geometry, the assembly tree, display meshes, PMI, product metadata,
  presentation, and units.
- Author what you write with `StepBuilder`, which builds files the way kernels
  hold data; `Ap242Author` is the strict low-level layer beneath it.
- Heal non-standard input — what can be fixed is normalized, what cannot is
  dropped with a reason in a provenance `Report`, never silently.
- Run on one lean universal schema model instead of per-schema code, with
  entities that never appear in real STEP files pruned out. The generated
  coverage matrix ([`docs/entities.md`](docs/entities.md)) lists exactly what is
  read and written.

[Unreleased]: https://github.com/elgar328/step-io/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/elgar328/step-io/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/elgar328/step-io/compare/v0.2.5...v0.3.0
[0.2.5]: https://github.com/elgar328/step-io/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/elgar328/step-io/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/elgar328/step-io/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/elgar328/step-io/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/elgar328/step-io/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/elgar328/step-io/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/elgar328/step-io/compare/v0.1.0-alpha.1...v0.1.0
