//! Topological dispatch: convert every DATA-section instance exactly once in
//! reference-dependency order. Builds the dependency order (Kahn + `#N`
//! tie-break), then runs each instance through its matching handlers (simple
//! by name, complex by exact part-set case) with the 2D/3D pcurve-subtree skip.

use std::collections::{BTreeSet, HashMap};

use super::ReaderContext;
use crate::entities::{ENTITY_HANDLERS, EntityHandlerEntry, ReadKind};
use crate::parser::entity::{Attribute, EntityGraph, RawEntity, RawEntityPart};

/// Name → handler index for topological dispatch (avoids scanning all
/// `ENTITY_HANDLERS` per entity). Simple handlers key on entity name; complex
/// handlers are matched by exact case (`matches_any_case`) so they share one bucket.
struct TopoIndex {
    simple: HashMap<&'static str, Vec<&'static EntityHandlerEntry>>,
    complex: Vec<&'static EntityHandlerEntry>,
}

fn build_topo_index() -> TopoIndex {
    let mut simple: HashMap<&'static str, Vec<&'static EntityHandlerEntry>> = HashMap::new();
    let mut complex: Vec<&'static EntityHandlerEntry> = Vec::new();
    for entry in ENTITY_HANDLERS {
        match entry.kind {
            ReadKind::Simple { .. } => simple.entry(entry.name).or_default().push(entry),
            ReadKind::Complex { .. } => complex.push(entry),
        }
    }
    TopoIndex { simple, complex }
}

/// Collect the **direct** `#N` references an entity makes in its own
/// attributes (recursing lists / typed params, NOT following the targets).
/// These are the dependency edges for the topological order.
fn direct_refs(ent: &RawEntity, out: &mut Vec<u64>) {
    match ent {
        RawEntity::Simple { attributes, .. } => {
            for a in attributes {
                direct_refs_attr(a, out);
            }
        }
        RawEntity::Complex { parts, .. } => {
            for p in parts {
                for a in &p.attributes {
                    direct_refs_attr(a, out);
                }
            }
        }
    }
}

fn direct_refs_attr(a: &Attribute, out: &mut Vec<u64>) {
    match a {
        Attribute::EntityRef(n) => out.push(*n),
        Attribute::List(items) => {
            for i in items {
                direct_refs_attr(i, out);
            }
        }
        Attribute::Typed { value, .. } => direct_refs_attr(value, out),
        _ => {}
    }
}

/// Topological order of the DATA-section instances (dependencies first) via
/// Kahn's algorithm with a `#N` tie-break (deterministic). Self-edges are
/// skipped (a schema-mandated self-reference is resolved by the handler, not
/// by ordering), so a self-referential instance is ordered normally. Nodes in
/// a genuine *multi-node* cycle never reach in-degree 0 and are **excluded**
/// (the caller flags them — see the error-on-cycle policy).
fn topo_order(graph: &EntityGraph) -> Vec<u64> {
    let n = graph.entities.len();
    let mut in_degree: HashMap<u64, usize> = HashMap::with_capacity(n);
    let mut dependents: HashMap<u64, Vec<u64>> = HashMap::new();
    let mut refs = Vec::new();
    for (&id, ent) in &graph.entities {
        refs.clear();
        direct_refs(ent, &mut refs);
        refs.sort_unstable();
        refs.dedup();
        let mut deg = 0usize;
        for &t in &refs {
            // A self-reference (`t == id`) cannot be an ordering dependency —
            // an instance can't be built before itself. Schema-mandated
            // self-refs (EXPRESS `dimensional_size_with_datum_feature.WR1:
            // applies_to :=: SELF`) are resolved by the handler, not by topo
            // ordering, so skip the self-edge. Multi-node cycles still leave
            // their nodes excluded (the caller flags them).
            if t == id {
                continue;
            }
            // Edge t -> id (t must precede id). Refs to absent instances are
            // ignored (a dangling ref doesn't constrain ordering).
            if graph.entities.contains_key(&t) {
                dependents.entry(t).or_default().push(id);
                deg += 1;
            }
        }
        in_degree.insert(id, deg);
    }
    let mut ready: BTreeSet<u64> = in_degree
        .iter()
        .filter(|&(_, &d)| d == 0)
        .map(|(&k, _)| k)
        .collect();
    let mut order = Vec::with_capacity(n);
    while let Some(&id) = ready.iter().next() {
        ready.remove(&id);
        order.push(id);
        if let Some(deps) = dependents.get(&id) {
            for &d in deps {
                if let Some(e) = in_degree.get_mut(&d) {
                    *e -= 1;
                    if *e == 0 {
                        ready.insert(d);
                    }
                }
            }
        }
    }
    order
}

impl ReaderContext {
    /// For each CBU outer recorded in `cbu_outer_to_mwu`, look up its
    /// conversion-factor MWU in `graph`, extract `unit_component` to get
    /// the base SI's `entity_id`, then mutate the outer's flavor entry in
    /// `named_units_arena` to set `cbu_base = Some(base_NamedUnitId)`.
    fn backfill_cbu_base(&mut self, graph: &EntityGraph) {
        use crate::ir::units::NamedUnit;
        let pairs: Vec<(u64, u64)> = self
            .cbu_outer_to_mwu
            .iter()
            .map(|(&o, &m)| (o, m))
            .collect();
        for (outer_id, mwu_id) in pairs {
            let base_entity = match graph.entities.get(&mwu_id) {
                Some(RawEntity::Simple { attributes, .. }) => {
                    attributes.iter().find_map(|a| match a {
                        // Typed wrapper: MEASURE_TYPE(real). Skip — that's the value.
                        crate::parser::entity::Attribute::EntityRef(e) => Some(*e),
                        _ => None,
                    })
                }
                _ => None,
            };
            let Some(base_entity_id) = base_entity else {
                continue;
            };
            let Some(base_nuid) = self
                .id_cache
                .get::<crate::ir::id::NamedUnitId>(base_entity_id)
            else {
                continue;
            };
            let Some(outer_nuid) = self.id_cache.get::<crate::ir::id::NamedUnitId>(outer_id) else {
                continue;
            };
            match &mut self.named_units_arena.items[outer_nuid.0 as usize] {
                NamedUnit::Length(f) => f.cbu_base = Some(base_nuid),
                NamedUnit::PlaneAngle(f) => f.cbu_base = Some(base_nuid),
                NamedUnit::Mass(f) => f.cbu_base = Some(base_nuid),
                // SolidAngle / Ratio / bare Itself have no CBU variant.
                NamedUnit::SolidAngle(_) | NamedUnit::Ratio(_) | NamedUnit::Itself(_) => {}
            }
        }
    }

    /// Re-resolve `SHAPE_DIMENSION_REPRESENTATION` items deferred from the
    /// SDR handler (phase measure-arena-1). Runs once the complex
    /// `MEASURE_REPRESENTATION_ITEM` arena entries (and their
    /// `repr_item_id_map` ids) exist. Refs are resolved in their original
    /// order; unresolved ones drop, matching the prior inline behaviour.
    fn resolve_deferred_sdr_items(&mut self) {
        use crate::entities::visualization::styled_item::resolve_representation_item_ref;
        use crate::ir::shape_rep::DimensionItem;
        use crate::ir::shape_rep::Representation;
        let raw = std::mem::take(&mut self.sdr_raw_items);
        for (repr_id, refs) in raw {
            // Try the descriptive map first (mirrors the CRI handler), then the
            // generic representation-item resolver; a ref is in exactly one.
            let items: Vec<_> = refs
                .into_iter()
                .filter_map(|r| {
                    self.descriptive_item_map
                        .get(&r)
                        .cloned()
                        .map(DimensionItem::Descriptive)
                        .or_else(|| {
                            resolve_representation_item_ref(self, r).map(DimensionItem::Item)
                        })
                })
                .collect();
            if let Representation::ShapeDimensionRepresentation(sdr) =
                &mut self.representations[repr_id]
            {
                sdr.items = items;
            }
        }
    }

    /// Convert every instance once in reference-dependency order. Validates the
    /// handler registry, seeds the CBU suppression set, runs the topo loop
    /// (folding in `SURFACE_CURVE` pcurve collection at each instance's
    /// position), then the post-passes (`backfill_cbu_base`, deferred SDR).
    pub(super) fn run_topo(&mut self, graph: &EntityGraph) {
        validate_registry_no_ambiguity();
        let order = topo_order(graph);
        if order.len() < graph.entities.len() {
            // Multi-node reference cycle: the unprocessed nodes can't be built
            // by the resolve-then-construct reader (chicken-and-egg). Flag as
            // malformed (error-on-cycle policy). Self-edges are skipped by
            // `topo_order`, so this only triggers for genuine multi-node cycles.
            let unresolved = graph.entities.len() - order.len();
            self.warnings
                .push(crate::ir::error::ConvertError::UnexpectedEntityForm {
                    entity_id: 0,
                    detail: format!(
                        "cyclic reference: {unresolved} instance(s) unprocessable (malformed file)"
                    ),
                });
        }
        // Order-independent seeding of the CBU `conversion_factor` suppression
        // set. Under topo the embedded MWU (a dependency) is processed before
        // its CONVERSION_BASED_UNIT, so the set must be seeded up front or the
        // MWU duplicates the inline conversion factor the writer re-emits.
        self.prescan_cbu_internal_mwu_refs(graph);
        let index = build_topo_index();
        for id in order {
            let Some(ent) = graph.get(id) else { continue };
            self.dispatch_one_topo(graph, id, ent, &index);
            // Fold the SURFACE_CURVE / SEAM_CURVE pcurve collection in at the
            // entity's topo position (its surfaces / 2D curves are already done).
            if let RawEntity::Simple {
                name, attributes, ..
            } = ent
                && (name == "SURFACE_CURVE" || name == "SEAM_CURVE")
            {
                crate::entities::geometry::surface_curve::collect_surface_curve(
                    self,
                    id,
                    attributes,
                    graph,
                    name == "SEAM_CURVE",
                );
            }
        }
        // Inline post-passes that `run_*_passes` ran mid-sequence — now after
        // the single loop (all producers done; equivalent timing).
        self.backfill_cbu_base(graph);
        self.resolve_deferred_sdr_items();
    }

    /// Seed `cbu_internal_mwu_refs` from every `CONVERSION_BASED_UNIT`'s
    /// `conversion_factor` ref (attr index 1) so the MWU handlers suppress the
    /// embedded duplicate regardless of dispatch order. Mirrors the insert in
    /// `read_conversion_based_unit_body` (which fires for any CBU name,
    /// recognised or not), just hoisted ahead of the topo loop.
    fn prescan_cbu_internal_mwu_refs(&mut self, graph: &EntityGraph) {
        for ent in graph.entities.values() {
            let RawEntity::Complex { parts, .. } = ent else {
                continue;
            };
            for part in parts {
                if part.name == "CONVERSION_BASED_UNIT"
                    && let Some(Attribute::EntityRef(r)) = part.attributes.get(1)
                {
                    self.cbu_internal_mwu_refs.insert(*r);
                }
            }
        }
    }

    /// Dispatch all matching handlers for one instance, applying the
    /// pcurve-subtree skip to 3D handlers (2D handlers self-discriminate).
    /// Simple handlers match by name; complex handlers by exact case.
    fn dispatch_one_topo(
        &mut self,
        graph: &EntityGraph,
        id: u64,
        ent: &RawEntity,
        index: &TopoIndex,
    ) {
        let is_pcurve = self.pcurve_subtree_ids.contains(&id);
        let candidates: &[&EntityHandlerEntry] = match ent {
            RawEntity::Simple { name, .. } => {
                index.simple.get(name.as_str()).map_or(&[], Vec::as_slice)
            }
            RawEntity::Complex { .. } => &index.complex,
        };
        for &entry in candidates {
            if !entry.is_2d {
                // 3D handler: honour the pcurve-subtree partition (POINT /
                // DIRECTION self-discriminate by coord count, so exempt).
                let respect_pcurve = !matches!(entry.name, "CARTESIAN_POINT" | "DIRECTION");
                if respect_pcurve && is_pcurve {
                    continue;
                }
            }
            self.dispatch_one(graph, entry, id, ent);
        }
        // Exact-case matching: a complex instance whose part-set matches no
        // handler case at all is dropped — surface it. The check is against the
        // registry (not whether dispatch *ran* a handler) so a pcurve-skipped
        // instance, which still has a matching handler, is not flagged.
        if let RawEntity::Complex { parts, .. } = ent {
            self.warn_unhandled_complex(id, parts);
        }
    }

    /// Try to dispatch a single `(entry, entity)` pair. The handler runs
    /// when name (simple) or required-parts (complex) match; otherwise
    /// nothing happens. Multiple handlers may match the same entity by
    /// design (self-discriminating sister handlers).
    fn dispatch_one(
        &mut self,
        graph: &EntityGraph,
        entry: &EntityHandlerEntry,
        id: u64,
        ent: &RawEntity,
    ) {
        match (&entry.kind, ent) {
            (
                ReadKind::Simple { read },
                RawEntity::Simple {
                    name, attributes, ..
                },
            ) if name == entry.name => {
                if let Err(e) = read(self, id, attributes, graph) {
                    self.record_drop_or_warn(entry, id, e, graph);
                }
            }
            (ReadKind::Complex { cases, read }, RawEntity::Complex { parts, .. })
                if crate::reader::matches_any_case(parts, cases) =>
            {
                if let Err(e) = read(self, id, parts, graph) {
                    self.record_drop_or_warn(entry, id, e, graph);
                }
            }
            _ => {}
        }
    }

    /// Classify a handler's read error: a `MissingReference` to an id the file
    /// never defines (dangling), or to an entity already dropped as a dangling
    /// cascade, is malformed *input* — not a step-io coverage gap. Record it as
    /// a `NonStandardInput` normalization (LOSS-exempt) and seed
    /// `nonstandard_dropped_refs` so dependents cascade the same way. Every
    /// other error stays a defect. Gated to Simple handlers: `entry.name` is the
    /// exact STEP type name there, whereas a complex handler's registered name
    /// may not match the part-name a round-trip checker keys on.
    /// See `reader::nonstandard` (`NS-dangling-reference-drop`).
    fn record_drop_or_warn(
        &mut self,
        entry: &EntityHandlerEntry,
        id: u64,
        e: crate::ir::error::ConvertError,
        graph: &EntityGraph,
    ) {
        if matches!(entry.kind, ReadKind::Simple { .. })
            && let crate::ir::error::ConvertError::MissingReference { to, .. } = &e
            && (graph.get(*to).is_none() || self.nonstandard_dropped_refs.contains(to))
        {
            self.ns_record(
                super::NsCase::DanglingReferenceDrop,
                entry.name.to_string(),
                "dropped (dangling/cascade reference)",
            );
            self.nonstandard_dropped_refs.insert(id);
            return;
        }
        // A strict ENUM bind rejected a non-standard token. Rejecting a
        // non-standard value is correct behaviour, so classify the drop as a
        // NORM normalization (not a defect/LOSS); seed the id so references
        // cascade the same way.
        if let crate::ir::error::ConvertError::NonStandardEnumValue { .. } = &e {
            self.ns_record(
                super::NsCase::NonStandardEnumValue,
                entry.name.to_string(),
                "dropped (non-standard enum value)",
            );
            self.nonstandard_dropped_refs.insert(id);
            return;
        }
        self.warnings.push(e);
    }

    /// Warn (once per distinct part-set per file) that a complex instance's
    /// part-set matches no complex handler case and was dropped. Skipped when
    /// some handler case *does* match (e.g. a pcurve-skipped instance) or the
    /// shape is allow-listed as read indirectly by another handler.
    fn warn_unhandled_complex(&mut self, id: u64, parts: &[RawEntityPart]) {
        let handled = ENTITY_HANDLERS.iter().any(|e| {
            matches!(&e.kind, ReadKind::Complex { cases, .. }
                if crate::reader::matches_any_case(parts, cases))
        });
        if handled {
            return;
        }
        let mut names: Vec<String> = parts.iter().map(|p| p.name.clone()).collect();
        names.sort();
        names.dedup();
        let actual: BTreeSet<&str> = names.iter().map(String::as_str).collect();
        if INDIRECTLY_READ_COMPLEX_CASES
            .iter()
            .any(|c| c.iter().copied().collect::<BTreeSet<_>>() == actual)
        {
            return;
        }
        if self.unhandled_complex_seen.insert(names.join("+")) {
            self.warnings
                .push(crate::ir::error::ConvertError::UnhandledComplex {
                    entity_id: id,
                    parts: names,
                });
        }
    }
}

/// Complex part-sets that are *not* dispatched by a registered handler but are
/// read indirectly by another handler walking the graph — so an "unhandled
/// complex" warning would be a false positive. (The RR complex is resolved by
/// `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION`.)
const INDIRECTLY_READ_COMPLEX_CASES: &[&[&str]] = &[&[
    "REPRESENTATION_RELATIONSHIP",
    "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
    "SHAPE_REPRESENTATION_RELATIONSHIP",
]];

/// One-shot validator: under exact-case matching, asserts no two
/// **distinct-name** complex handlers (across *all* passes) declare a shared
/// exact case — such a pair would both claim the same instance, dropping the
/// loser's parts. Same-name handlers are exempt: they are 2D/3D sisters
/// (e.g. `RATIONAL_B_SPLINE_CURVE`) that deliberately share cases and are
/// disambiguated at dispatch by the pcurve-subtree skip / 2D-vs-3D pass split.
fn validate_registry_no_ambiguity() {
    use std::sync::OnceLock;
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let case_eq =
            |a: &[&str], b: &[&str]| a.len() == b.len() && a.iter().all(|p| b.contains(p));
        for (i, a) in ENTITY_HANDLERS.iter().enumerate() {
            for b in ENTITY_HANDLERS.iter().skip(i + 1) {
                if a.name == b.name {
                    continue; // 2D/3D sisters share cases by design.
                }
                if let (ReadKind::Complex { cases: ca, .. }, ReadKind::Complex { cases: cb, .. }) =
                    (&a.kind, &b.kind)
                {
                    for x in *ca {
                        for y in *cb {
                            assert!(
                                !case_eq(x, y),
                                "ambiguous complex handlers '{}' vs '{}' share exact case {x:?}",
                                a.name,
                                b.name,
                            );
                        }
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod topo_tests {
    use super::topo_order;

    fn graph(data: &str) -> crate::parser::EntityGraph {
        let src = format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
             FILE_NAME('','',(''),(''),'','','');\n\
             FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\nDATA;\n{data}\nENDSEC;\n\
             END-ISO-10303-21;\n"
        );
        crate::parse(&src).expect("parse")
    }

    fn pos(order: &[u64], id: u64) -> usize {
        order.iter().position(|&x| x == id).expect("in order")
    }

    #[test]
    fn dependencies_precede_dependents_and_full_coverage() {
        // #1 -> #2,#3 ; #2 -> #4 ; #3 -> #4 ; #4 -> (none)
        let g = graph("#1=A('',#2,#3);\n#2=B('',#4);\n#3=C('',#4);\n#4=D('');");
        let order = topo_order(&g);
        assert_eq!(order.len(), 4, "all acyclic nodes covered");
        assert!(pos(&order, 4) < pos(&order, 2));
        assert!(pos(&order, 4) < pos(&order, 3));
        assert!(pos(&order, 2) < pos(&order, 1));
        assert!(pos(&order, 3) < pos(&order, 1));
        // deterministic #N tie-break: #4 first, then #2 before #3.
        assert_eq!(order, vec![4, 2, 3, 1]);
    }

    #[test]
    fn cycle_nodes_excluded() {
        // #1 <-> #2 cycle, #3 acyclic leaf referenced by #1.
        let g = graph("#1=A('',#2,#3);\n#2=B('',#1);\n#3=C('');");
        let order = topo_order(&g);
        // #3 is acyclic and processable; #1/#2 (cycle) are excluded.
        assert_eq!(order, vec![3], "only the acyclic leaf is ordered");
    }

    #[test]
    fn self_reference_is_ordered_not_excluded() {
        // #2 references itself (the EXPRESS WR1 `applies_to :=: SELF` shape,
        // e.g. DIMENSIONAL_SIZE_WITH_DATUM_FEATURE). The self-edge must be
        // skipped so #2 is ordered after its real dependency #1, and the
        // dependent #3 follows — not dropped as a cycle.
        let g = graph("#1=A('');\n#2=B('',#1,#2);\n#3=C('',#2);");
        let order = topo_order(&g);
        assert_eq!(
            order.len(),
            3,
            "self-ref node and its dependent are ordered"
        );
        assert!(pos(&order, 1) < pos(&order, 2), "real dependency precedes");
        assert!(
            pos(&order, 2) < pos(&order, 3),
            "self-ref node precedes dependent"
        );
    }
}
