//! read (`EntityGraph` map → `SpikeUnits`) and write (`SpikeUnits` → STEP text).
//!
//! Annotated `[GEN]` / `[HAND]` like generated.rs for the coverage estimate.
//! read is two-pass: pass 1 assigns a typed id to every unit `#N` (drop-hole
//! free — id is allocated at push time), pass 2 resolves refs.

use std::collections::BTreeMap;

use step_io::{Attribute, RawEntity, RawEntityPart};

use crate::generated::*;

/// Set of entity type names that belong to the units subset this spike handles.
/// [GEN] — derivable as the schema closure of the target entities.
const SIMPLE_UNIT_NAMES: &[&str] = &[
    "DIMENSIONAL_EXPONENTS",
    "MEASURE_WITH_UNIT",
    "LENGTH_MEASURE_WITH_UNIT",
    "UNCERTAINTY_MEASURE_WITH_UNIT",
    "DERIVED_UNIT",
    "DERIVED_UNIT_ELEMENT",
];

const COMPLEX_PART_NAMES: &[&str] = &[
    "NAMED_UNIT",
    "SI_UNIT",
    "CONVERSION_BASED_UNIT",
    "CONTEXT_DEPENDENT_UNIT",
    "LENGTH_UNIT",
    "MASS_UNIT",
    "PLANE_ANGLE_UNIT",
    "SOLID_ANGLE_UNIT",
    "TIME_UNIT",
];

fn is_complex_unit(parts: &[RawEntityPart]) -> bool {
    parts
        .iter()
        .any(|p| COMPLEX_PART_NAMES.contains(&p.name.as_str()))
}

/// What kind of typed id a given `#N` maps to. [GEN]
#[derive(Debug, Clone, Copy)]
pub enum UnitId {
    Complex(ComplexUnitId),
    DimExp(DimExpId),
    Measure(MeasureWithUnitId),
    Derived(DerivedUnitId),
    DerivedElem(DerivedUnitElementId),
}

/// Read the units subset out of a parsed entity map into `SpikeUnits`.
/// Returns the model plus the `#N -> UnitId` map (for the driver / tests).
pub fn read(map: &BTreeMap<u64, RawEntity>) -> (SpikeUnits, BTreeMap<u64, UnitId>) {
    let mut model = SpikeUnits::default();
    let mut idmap: BTreeMap<u64, UnitId> = BTreeMap::new();

    // ---- pass 1: allocate ids + bind non-ref content -------------------
    // Refs are filled with a placeholder and patched in pass 2. We store the
    // raw RawEntity index alongside so pass 2 can re-read attrs. [GEN]
    let mut pending_measures: Vec<(MeasureWithUnitId, u64)> = Vec::new();
    let mut pending_complex: Vec<(ComplexUnitId, u64)> = Vec::new();
    let mut pending_derived: Vec<(DerivedUnitId, u64)> = Vec::new();
    let mut pending_derived_elem: Vec<(DerivedUnitElementId, u64)> = Vec::new();

    for (&id, ent) in map {
        match ent {
            RawEntity::Simple {
                name, attributes, ..
            } if name == "DIMENSIONAL_EXPONENTS" => {
                let de = read_dim_exp(attributes);
                let aid = DimExpId(model.dim_exps.push(de));
                idmap.insert(id, UnitId::DimExp(aid));
            }
            RawEntity::Simple { name, .. }
                if name.ends_with("MEASURE_WITH_UNIT") || name == "MEASURE_WITH_UNIT" =>
            {
                // placeholder; resolved in pass 2
                let placeholder = MeasureWithUnit {
                    subtype_name: name.clone(),
                    value_component: MeasureValue {
                        type_name: None,
                        value: 0.0,
                    },
                    unit_component: UnitRef::NamedUnit(ComplexUnitId(usize::MAX)),
                    extra: Vec::new(),
                };
                let aid = MeasureWithUnitId(model.measures.push(placeholder));
                idmap.insert(id, UnitId::Measure(aid));
                pending_measures.push((aid, id));
            }
            RawEntity::Simple { name, .. } if name == "DERIVED_UNIT" => {
                let aid = DerivedUnitId(model.derived_units.push(DerivedUnit {
                    elements: Vec::new(),
                }));
                idmap.insert(id, UnitId::Derived(aid));
                pending_derived.push((aid, id));
            }
            RawEntity::Simple { name, .. } if name == "DERIVED_UNIT_ELEMENT" => {
                let aid =
                    DerivedUnitElementId(model.derived_unit_elements.push(DerivedUnitElement {
                        unit: NamedUnitRef::Complex(ComplexUnitId(usize::MAX)),
                        exponent: 0.0,
                    }));
                idmap.insert(id, UnitId::DerivedElem(aid));
                pending_derived_elem.push((aid, id));
            }
            RawEntity::Complex { parts, .. } if is_complex_unit(parts) => {
                // parts with non-ref attrs bound now; ref-bearing parts
                // (NAMED_UNIT(#dim), CONVERSION_BASED_UNIT) patched in pass 2.
                let bag = read_complex_parts_norefs(parts);
                let aid = ComplexUnitId(model.complex_units.push(ComplexUnit { parts: bag }));
                idmap.insert(id, UnitId::Complex(aid));
                pending_complex.push((aid, id));
            }
            _ => {}
        }
        let _ = SIMPLE_UNIT_NAMES; // referenced for documentation/coverage
    }

    // ---- pass 2: resolve refs ------------------------------------------
    for (aid, raw_id) in pending_measures {
        if let Some(RawEntity::Simple { attributes, .. }) = map.get(&raw_id) {
            resolve_measure(&mut model, aid, attributes, &idmap);
        }
    }
    for (aid, raw_id) in pending_complex {
        if let Some(RawEntity::Complex { parts, .. }) = map.get(&raw_id) {
            resolve_complex(&mut model, aid, parts, &idmap);
        }
    }
    for (aid, raw_id) in pending_derived_elem {
        if let Some(RawEntity::Simple { attributes, .. }) = map.get(&raw_id) {
            resolve_derived_elem(&mut model, aid, attributes, &idmap);
        }
    }
    for (aid, raw_id) in pending_derived {
        if let Some(RawEntity::Simple { attributes, .. }) = map.get(&raw_id) {
            resolve_derived(&mut model, aid, attributes, &idmap);
        }
    }

    (model, idmap)
}

// ---- read helpers ------------------------------------------------------

fn as_real(a: &Attribute) -> f64 {
    match a {
        Attribute::Real(r) => *r,
        Attribute::Integer(i) => *i as f64,
        _ => panic!("expected real, got {a:?}"),
    }
}

/// [GEN] flat struct read: 7 reals in order.
fn read_dim_exp(attrs: &[Attribute]) -> DimensionalExponents {
    DimensionalExponents {
        length_exponent: as_real(&attrs[0]),
        mass_exponent: as_real(&attrs[1]),
        time_exponent: as_real(&attrs[2]),
        electric_current_exponent: as_real(&attrs[3]),
        thermodynamic_temperature_exponent: as_real(&attrs[4]),
        amount_of_substance_exponent: as_real(&attrs[5]),
        luminous_intensity_exponent: as_real(&attrs[6]),
    }
}

/// [GEN] select-of-scalars read: `LENGTH_MEASURE(1.0)` → {type_name, value}.
fn read_measure_value(a: &Attribute) -> MeasureValue {
    match a {
        // Generic select member: TYPE(scalar).
        Attribute::Typed { type_name, value } => MeasureValue {
            type_name: Some(type_name.clone()),
            value: as_real(value),
        },
        // Bare real: a subtype redeclared value_component to a concrete REAL.
        Attribute::Real(_) | Attribute::Integer(_) => MeasureValue {
            type_name: None,
            value: as_real(a),
        },
        _ => panic!("expected measure_value, got {a:?}"),
    }
}

/// Read complex parts, binding only NON-ref own attrs (refs filled in pass 2).
/// [GEN] — dispatch on part name; per-part own-attr binding; NO combo matching.
fn read_complex_parts_norefs(parts: &[RawEntityPart]) -> Vec<UnitPart> {
    parts
        .iter()
        .map(|p| match p.name.as_str() {
            "NAMED_UNIT" => UnitPart::NamedUnit {
                // `*` -> Derived; a ref is patched in pass 2.
                dimensions: match &p.attributes[0] {
                    Attribute::Derived => DimRef::Derived,
                    _ => DimRef::Ref(DimExpId(usize::MAX)),
                },
            },
            "SI_UNIT" => UnitPart::SiUnit {
                prefix: match &p.attributes[0] {
                    Attribute::Unset => None,
                    Attribute::Enum(s) => Some(SiPrefix::parse(s).expect("si_prefix")),
                    other => panic!("si_unit prefix: {other:?}"),
                },
                name: match &p.attributes[1] {
                    Attribute::Enum(s) => SiUnitName::parse(s).expect("si_unit_name"),
                    other => panic!("si_unit name: {other:?}"),
                },
            },
            "CONVERSION_BASED_UNIT" => UnitPart::ConversionBasedUnit {
                name: match &p.attributes[0] {
                    Attribute::String(s) => s.clone(),
                    other => panic!("cbu name: {other:?}"),
                },
                conversion_factor: MeasureWithUnitId(usize::MAX),
            },
            "CONTEXT_DEPENDENT_UNIT" => UnitPart::ContextDependentUnit {
                name: match &p.attributes[0] {
                    Attribute::String(s) => s.clone(),
                    other => panic!("cdu name: {other:?}"),
                },
            },
            "LENGTH_UNIT" => UnitPart::LengthUnit,
            "MASS_UNIT" => UnitPart::MassUnit,
            "PLANE_ANGLE_UNIT" => UnitPart::PlaneAngleUnit,
            "SOLID_ANGLE_UNIT" => UnitPart::SolidAngleUnit,
            "TIME_UNIT" => UnitPart::TimeUnit,
            other => panic!("unknown complex unit part: {other}"),
        })
        .collect()
}

fn lookup_unit_ref(idmap: &BTreeMap<u64, UnitId>, n: u64) -> UnitRef {
    match idmap.get(&n) {
        Some(UnitId::Complex(c)) => UnitRef::NamedUnit(*c),
        Some(UnitId::Derived(d)) => UnitRef::DerivedUnit(*d),
        other => panic!("unit_component ref #{n} -> {other:?}"),
    }
}

fn lookup_named_unit_ref(idmap: &BTreeMap<u64, UnitId>, n: u64) -> NamedUnitRef {
    match idmap.get(&n) {
        Some(UnitId::Complex(c)) => NamedUnitRef::Complex(*c),
        other => panic!("named_unit ref #{n} -> {other:?}"),
    }
}

fn lookup_dim_ref(idmap: &BTreeMap<u64, UnitId>, n: u64) -> DimExpId {
    match idmap.get(&n) {
        Some(UnitId::DimExp(d)) => *d,
        other => panic!("dimensions ref #{n} -> {other:?}"),
    }
}

fn lookup_measure_id(idmap: &BTreeMap<u64, UnitId>, n: u64) -> MeasureWithUnitId {
    match idmap.get(&n) {
        Some(UnitId::Measure(m)) => *m,
        other => panic!("conversion_factor ref #{n} -> {other:?}"),
    }
}

/// [GEN] resolve MWU refs + capture subtype-extra string attrs.
fn resolve_measure(
    model: &mut SpikeUnits,
    aid: MeasureWithUnitId,
    attrs: &[Attribute],
    idmap: &BTreeMap<u64, UnitId>,
) {
    let value_component = read_measure_value(&attrs[0]);
    let unit_component = match &attrs[1] {
        Attribute::EntityRef(n) => lookup_unit_ref(idmap, *n),
        other => panic!("unit_component: {other:?}"),
    };
    // tail attrs (uncertainty_measure_with_unit name/description) [HAND]
    let mut extra = Vec::new();
    for a in &attrs[2..] {
        match a {
            Attribute::String(s) => extra.push(s.clone()),
            Attribute::Unset => extra.push(String::new()),
            other => panic!("mwu extra attr: {other:?}"),
        }
    }
    let m = &mut model.measures.items[aid.0];
    m.value_component = value_component;
    m.unit_component = unit_component;
    m.extra = extra;
}

/// [GEN] patch ref-bearing complex parts.
fn resolve_complex(
    model: &mut SpikeUnits,
    aid: ComplexUnitId,
    parts: &[RawEntityPart],
    idmap: &BTreeMap<u64, UnitId>,
) {
    let bag = &mut model.complex_units.items[aid.0];
    for (slot, p) in bag.parts.iter_mut().zip(parts.iter()) {
        match (slot, p.name.as_str()) {
            (UnitPart::NamedUnit { dimensions }, "NAMED_UNIT") => {
                if let Attribute::EntityRef(n) = &p.attributes[0] {
                    *dimensions = DimRef::Ref(lookup_dim_ref(idmap, *n));
                }
            }
            (
                UnitPart::ConversionBasedUnit {
                    conversion_factor, ..
                },
                "CONVERSION_BASED_UNIT",
            ) => {
                if let Attribute::EntityRef(n) = &p.attributes[1] {
                    *conversion_factor = lookup_measure_id(idmap, *n);
                }
            }
            _ => {}
        }
    }
}

fn lookup_derived_elem(idmap: &BTreeMap<u64, UnitId>, n: u64) -> DerivedUnitElementId {
    match idmap.get(&n) {
        Some(UnitId::DerivedElem(e)) => *e,
        other => panic!("derived_unit_element ref #{n} -> {other:?}"),
    }
}

/// [GEN] resolve `derived_unit.elements` = SET OF refs to standalone elements.
fn resolve_derived(
    model: &mut SpikeUnits,
    aid: DerivedUnitId,
    attrs: &[Attribute],
    idmap: &BTreeMap<u64, UnitId>,
) {
    let mut elems = Vec::new();
    if let Attribute::List(list) = &attrs[0] {
        for e in list {
            match e {
                Attribute::EntityRef(n) => elems.push(lookup_derived_elem(idmap, *n)),
                other => panic!("derived_unit element entry: {other:?}"),
            }
        }
    }
    model.derived_units.items[aid.0].elements = elems;
}

/// [GEN] resolve a standalone `DERIVED_UNIT_ELEMENT(#unit, exponent)`.
fn resolve_derived_elem(
    model: &mut SpikeUnits,
    aid: DerivedUnitElementId,
    attrs: &[Attribute],
    idmap: &BTreeMap<u64, UnitId>,
) {
    let unit = match &attrs[0] {
        Attribute::EntityRef(n) => lookup_named_unit_ref(idmap, *n),
        other => panic!("due unit: {other:?}"),
    };
    let elem = &mut model.derived_unit_elements.items[aid.0];
    elem.unit = unit;
    elem.exponent = as_real(&attrs[1]);
}

// ===========================================================================
// WRITE: SpikeUnits -> STEP DATA text. [GEN]
// ===========================================================================

/// Writer state: assigns fresh `#N` to each arena item on demand, topo-first
/// (dependencies emitted before dependents). [GEN]
pub struct Writer<'a> {
    model: &'a SpikeUnits,
    next: u64,
    out: String,
    dim_ids: Vec<Option<u64>>,
    measure_ids: Vec<Option<u64>>,
    complex_ids: Vec<Option<u64>>,
    derived_ids: Vec<Option<u64>>,
    derived_elem_ids: Vec<Option<u64>>,
}

impl<'a> Writer<'a> {
    pub fn new(model: &'a SpikeUnits) -> Self {
        Writer {
            model,
            next: 1,
            out: String::new(),
            dim_ids: vec![None; model.dim_exps.items.len()],
            measure_ids: vec![None; model.measures.items.len()],
            complex_ids: vec![None; model.complex_units.items.len()],
            derived_ids: vec![None; model.derived_units.items.len()],
            derived_elem_ids: vec![None; model.derived_unit_elements.items.len()],
        }
    }

    fn fresh(&mut self) -> u64 {
        let n = self.next;
        self.next += 1;
        n
    }

    fn emit_dim(&mut self, id: DimExpId) -> u64 {
        if let Some(n) = self.dim_ids[id.0] {
            return n;
        }
        let n = self.fresh();
        self.dim_ids[id.0] = Some(n);
        let d = self.model.dim_exps.get(id.0);
        self.out.push_str(&format!(
            "#{n} = DIMENSIONAL_EXPONENTS({},{},{},{},{},{},{});\n",
            real(d.length_exponent),
            real(d.mass_exponent),
            real(d.time_exponent),
            real(d.electric_current_exponent),
            real(d.thermodynamic_temperature_exponent),
            real(d.amount_of_substance_exponent),
            real(d.luminous_intensity_exponent),
        ));
        n
    }

    fn emit_measure(&mut self, id: MeasureWithUnitId) -> u64 {
        if let Some(n) = self.measure_ids[id.0] {
            return n;
        }
        let m = self.model.measures.get(id.0);
        let unit_ref = self.emit_unit_ref(m.unit_component);
        let n = self.fresh();
        self.measure_ids[id.0] = Some(n);
        let val = match &m.value_component.type_name {
            Some(t) => format!("{t}({})", real(m.value_component.value)),
            None => real(m.value_component.value),
        };
        let mut s = format!("#{n} = {}({val},#{unit_ref}", m.subtype_name);
        for e in &m.extra {
            s.push_str(&format!(",'{e}'"));
        }
        s.push_str(");\n");
        self.out.push_str(&s);
        n
    }

    fn emit_unit_ref(&mut self, r: UnitRef) -> u64 {
        match r {
            UnitRef::NamedUnit(c) => self.emit_complex(c),
            UnitRef::DerivedUnit(d) => self.emit_derived(d),
        }
    }

    fn emit_named_unit_ref(&mut self, r: NamedUnitRef) -> u64 {
        match r {
            NamedUnitRef::Complex(c) => self.emit_complex(c),
        }
    }

    fn emit_derived_elem(&mut self, id: DerivedUnitElementId) -> u64 {
        if let Some(n) = self.derived_elem_ids[id.0] {
            return n;
        }
        let e = self.model.derived_unit_elements.get(id.0);
        let unit_ref = self.emit_named_unit_ref(e.unit);
        let exp = e.exponent;
        let n = self.fresh();
        self.derived_elem_ids[id.0] = Some(n);
        self.out.push_str(&format!(
            "#{n} = DERIVED_UNIT_ELEMENT(#{unit_ref},{});\n",
            real(exp)
        ));
        n
    }

    fn emit_derived(&mut self, id: DerivedUnitId) -> u64 {
        if let Some(n) = self.derived_ids[id.0] {
            return n;
        }
        let du = self.model.derived_units.get(id.0);
        let elem_ids: Vec<DerivedUnitElementId> = du.elements.clone();
        let refs: Vec<u64> = elem_ids
            .iter()
            .map(|&e| self.emit_derived_elem(e))
            .collect();
        let n = self.fresh();
        self.derived_ids[id.0] = Some(n);
        let elems = refs
            .iter()
            .map(|u| format!("#{u}"))
            .collect::<Vec<_>>()
            .join(",");
        self.out
            .push_str(&format!("#{n} = DERIVED_UNIT(({elems}));\n"));
        n
    }

    fn emit_complex(&mut self, id: ComplexUnitId) -> u64 {
        if let Some(n) = self.complex_ids[id.0] {
            return n;
        }
        let cu = self.model.complex_units.get(id.0);
        // emit dependency refs first (dimensions, conversion_factor)
        let mut part_txt: Vec<String> = Vec::with_capacity(cu.parts.len());
        for part in &cu.parts {
            part_txt.push(match part {
                UnitPart::NamedUnit { dimensions } => match dimensions {
                    DimRef::Derived => "NAMED_UNIT(*)".to_string(),
                    DimRef::Ref(d) => {
                        let dn = self.emit_dim(*d);
                        format!("NAMED_UNIT(#{dn})")
                    }
                },
                UnitPart::SiUnit { prefix, name } => {
                    let p = match prefix {
                        Some(p) => p.token(),
                        None => "$",
                    };
                    format!("SI_UNIT({p},{})", name.token())
                }
                UnitPart::ConversionBasedUnit {
                    name,
                    conversion_factor,
                } => {
                    let mn = self.emit_measure(*conversion_factor);
                    format!("CONVERSION_BASED_UNIT('{name}',#{mn})")
                }
                UnitPart::ContextDependentUnit { name } => {
                    format!("CONTEXT_DEPENDENT_UNIT('{name}')")
                }
                UnitPart::LengthUnit => "LENGTH_UNIT()".to_string(),
                UnitPart::MassUnit => "MASS_UNIT()".to_string(),
                UnitPart::PlaneAngleUnit => "PLANE_ANGLE_UNIT()".to_string(),
                UnitPart::SolidAngleUnit => "SOLID_ANGLE_UNIT()".to_string(),
                UnitPart::TimeUnit => "TIME_UNIT()".to_string(),
            });
        }
        let n = self.fresh();
        self.complex_ids[id.0] = Some(n);
        self.out
            .push_str(&format!("#{n} = ( {} );\n", part_txt.join(" ")));
        n
    }

    /// Emit every arena item, returning the DATA-section body. [GEN]
    pub fn emit_all(mut self) -> String {
        for i in 0..self.model.complex_units.items.len() {
            self.emit_complex(ComplexUnitId(i));
        }
        for i in 0..self.model.measures.items.len() {
            self.emit_measure(MeasureWithUnitId(i));
        }
        for i in 0..self.model.derived_units.items.len() {
            self.emit_derived(DerivedUnitId(i));
        }
        for i in 0..self.model.derived_unit_elements.items.len() {
            self.emit_derived_elem(DerivedUnitElementId(i));
        }
        for i in 0..self.model.dim_exps.items.len() {
            self.emit_dim(DimExpId(i));
        }
        self.out
    }
}

/// Format a real the way STEP does (always a decimal point). [GEN]
fn real(v: f64) -> String {
    if v == v.trunc() && v.is_finite() && v.abs() < 1e15 {
        return format!("{}.", v as i64);
    }
    // STEP real syntax requires a decimal point in the mantissa (`1.E-07`),
    // whereas Rust's `{:E}` emits `1E-7`. Insert the point and normalize.
    let s = format!("{v:E}");
    if let Some(epos) = s.find('E') {
        let (mant, exp) = s.split_at(epos);
        let mant = if mant.contains('.') {
            mant.to_string()
        } else {
            format!("{mant}.")
        };
        format!("{mant}{exp}")
    } else {
        s
    }
}

/// Wrap a DATA body into a minimal valid STEP file. [GEN/HAND boilerplate]
pub fn wrap_step(data_body: &str) -> String {
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\n\
         FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\nDATA;\n{data_body}ENDSEC;\n\
         END-ISO-10303-21;\n"
    )
}
