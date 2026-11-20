//! The low-level model reads: `model.all::<T>()`, `model.get(id)` and
//! `model[id]`, and the forward `Ref` resolver — holding a reference (a
//! `LINE`'s `pnt` / `dir`), `resolve(&model)` returns the concrete target
//! object, the forward dual of `RefGraph`'s backward `referrers`.

use step_io::model::{CartesianPointRefView, VectorRefView};
use step_io::read;

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

const BODY: &str = "\
#1=CARTESIAN_POINT('',(2.0,0.0,0.0));\n\
#2=DIRECTION('',(1.0,0.0,0.0));\n\
#3=VECTOR('',#2,1.0);\n\
#4=LINE('',#1,#3);\n";

#[test]
fn resolves_forward_references_to_concrete_targets() {
    let src = format!("{HEADER}{BODY}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");

    let line = &model[step_io::model::LineId(0)];

    // LINE.pnt is a CartesianPointRef → its concrete CARTESIAN_POINT.
    match line.pnt.resolve(&model) {
        CartesianPointRefView::CartesianPoint(p) => assert_eq!(p.coordinates[..], [2.0, 0.0, 0.0]),
        other => panic!("expected a cartesian point, got {other:?}"),
    }

    // LINE.dir is a VectorRef → its concrete VECTOR.
    assert!(matches!(line.dir.resolve(&model), VectorRefView::Vector(_)));
}

#[test]
fn reads_entities_by_type_and_by_id() {
    let src = format!("{HEADER}{BODY}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");

    // `all` pairs each entity with the id that names it.
    let lines: Vec<_> = model.all::<step_io::model::Line>().collect();
    assert_eq!(lines.len(), 1);
    for (id, line) in lines {
        let (by_index, by_get) = (&model[id], model.get(id).expect("in range"));
        assert!(std::ptr::eq(by_index, line));
        assert!(std::ptr::eq(by_get, line));
    }
    assert!(model.all::<step_io::model::Circle>().next().is_none());
    // An id out of range is `None` for `get` (and a panic for indexing).
    assert!(model.get(step_io::model::LineId(1)).is_none());
}

/// A key names its entity type's STEP keyword; a complex instance has none.
#[test]
fn entity_keys_name_their_keyword() {
    use step_io::EntityKey;
    use step_io::model::{AdvancedFaceId, ComplexEntityId, ShellBasedSurfaceModelId};
    assert_eq!(
        EntityKey::AdvancedFace(AdvancedFaceId(0)).keyword(),
        Some("ADVANCED_FACE")
    );
    assert_eq!(
        EntityKey::ShellBasedSurfaceModel(ShellBasedSurfaceModelId(3)).keyword(),
        Some("SHELL_BASED_SURFACE_MODEL")
    );
    assert_eq!(EntityKey::ComplexEntity(ComplexEntityId(0)).keyword(), None);
}

/// An aggregate member of a SELECT resolves to its elements, the model's
/// references.
#[test]
fn resolves_an_aggregate_member_to_its_elements() {
    use step_io::model::{CompoundItemDefinitionRefView, CompoundRepresentationItemId};

    let body =
        format!("{BODY}#5=COMPOUND_REPRESENTATION_ITEM('c',SET_REPRESENTATION_ITEM((#1,#4)));\n");
    let src = format!("{HEADER}{body}{FOOTER}");
    let (model, _rep) = read(src.as_bytes()).expect("read");

    let item = &model[CompoundRepresentationItemId(0)];
    let CompoundItemDefinitionRefView::SetRepresentationItem(elements) =
        item.item_element.resolve(&model)
    else {
        panic!("expected a set of representation items");
    };
    let targets: Vec<_> = elements
        .iter()
        .map(step_io::model::RepresentationItemRef::entity_key)
        .collect();
    assert_eq!(
        targets,
        [
            step_io::EntityKey::CartesianPoint(step_io::model::CartesianPointId(0)),
            step_io::EntityKey::Line(step_io::model::LineId(0)),
        ]
    );
}
