//! Read-only product / assembly navigation handles over a [`Scene`].
//!
//! A [`Product`] is a part — its [`versions`](Product::versions),
//! [`definitions`](Product::definitions), and PLM metadata (contributors,
//! approvals, documents) hang off it. A [`ProductDef`] descends the assembly
//! through its [`occurrences`](ProductDef::occurrences), each a placed
//! instance of a child definition, and bridges to the geometry handles via
//! [`solids`](ProductDef::solids). Enter from [`Scene::all_products`] or the
//! assembly roots [`Scene::root_definitions`].

// Every public method here is a pure read accessor (see geometry.rs).
#![allow(clippy::must_use_candidate)]

use std::collections::HashSet;

use crate::generated::model as m;
use crate::scene::geometry::Solid;
use crate::scene::pmi;
use crate::scene::{Ctx, Scene};

impl Scene<'_> {
    /// Every product (`PRODUCT`) in the model.
    pub fn all_products(&self) -> impl Iterator<Item = Product<'_>> + '_ {
        let cx = self.ctx();
        (0..cx.model.product_arena.items.len()).map(move |i| Product {
            cx,
            id: m::ProductId(i),
        })
    }

    /// Every product definition (`PRODUCT_DEFINITION`) in the model.
    pub fn all_product_definitions(&self) -> impl Iterator<Item = ProductDef<'_>> + '_ {
        let cx = self.ctx();
        let plain = (0..cx.model.product_definition_arena.items.len()).map(move |i| ProductDef {
            cx,
            which: PdImpl::Plain(m::ProductDefinitionId(i)),
        });
        let with_docs = (0..cx
            .model
            .product_definition_with_associated_documents_arena
            .items
            .len())
            .map(move |i| ProductDef {
                cx,
                which: PdImpl::WithDocs(m::ProductDefinitionWithAssociatedDocumentsId(i)),
            });
        plain.chain(with_docs)
    }

    /// The assembly roots: definitions no `NEXT_ASSEMBLY_USAGE_OCCURRENCE`
    /// uses as a child (`related`). Start an assembly walk here — a single
    /// part file simply yields all of its definitions.
    pub fn root_definitions(&self) -> impl Iterator<Item = ProductDef<'_>> + '_ {
        let cx = self.ctx();
        let rg = cx.ref_graph();
        self.all_product_definitions().filter(move |def| {
            !rg.referrers(def.key()).iter().any(|r| {
                let m::EntityKey::NextAssemblyUsageOccurrence(nid) = r else {
                    return false;
                };
                let nauo = cx.model.next_assembly_usage_occurrence_arena.get(nid.0);
                matches!(
                    &nauo.related_product_definition,
                    m::ProductDefinitionOrReferenceRef::ProductDefinition(d) if m::EntityKey::ProductDefinition(*d) == def.key()
                )
            })
        })
    }
}

// ---------------------------------------------------------------------------
// Product
// ---------------------------------------------------------------------------

/// A product (`PRODUCT`) — the identity of a part (id + name).
#[derive(Clone, Copy)]
pub struct Product<'m> {
    cx: Ctx<'m>,
    id: m::ProductId,
}

impl<'m> Product<'m> {
    fn raw(&self) -> &'m m::Product {
        self.cx.model.product_arena.get(self.id.0)
    }

    /// This product's global identity (a `Copy` key for maps / deduplication;
    /// distinct from [`Product::id`], which is the STEP `id` string attribute).
    pub fn key(&self) -> m::EntityKey {
        m::EntityKey::Product(self.id)
    }

    pub fn id(&self) -> &'m str {
        &self.raw().id
    }

    pub fn name(&self) -> &'m str {
        &self.raw().name
    }

    pub fn description(&self) -> Option<&'m str> {
        self.raw().description.as_deref()
    }

    /// The definitions of this product (reverse: a `PRODUCT_DEFINITION_FORMATION`
    /// whose `of_product` is this product → a `PRODUCT_DEFINITION` whose `formation`
    /// is that formation).
    ///
    /// No field re-check is needed: a formation's only product reference is
    /// `of_product` and a definition's only formation reference is `formation`, so
    /// `referrers` already pins both edges to this product.
    pub fn definitions(&self) -> impl Iterator<Item = ProductDef<'m>> + 'm {
        let cx = self.cx;
        let rg = cx.ref_graph();
        let mut out: Vec<ProductDef<'m>> = Vec::new();
        for &fr in rg.referrers(m::EntityKey::Product(self.id)) {
            if !matches!(
                fr,
                m::EntityKey::ProductDefinitionFormation(_)
                    | m::EntityKey::ProductDefinitionFormationWithSpecifiedSource(_)
            ) {
                continue;
            }
            for &pr in rg.referrers(fr) {
                if let Some(d) = ProductDef::from_key(cx, pr) {
                    out.push(d);
                }
            }
        }
        out.into_iter()
    }

    /// Every key metadata may attach to for *this whole product*: the product
    /// itself plus all of its formations and definitions. Metadata in STEP is
    /// mostly version-level (assigned to a formation or definition), so a
    /// product-wide view must reach down the tree, not just the product entity.
    /// Mirrors the `product → formation → definition` traversal of
    /// [`Product::definitions`].
    fn metadata_targets(&self) -> Vec<m::EntityKey> {
        let rg = self.cx.ref_graph();
        let mut targets = vec![self.key()];
        for &fr in rg.referrers(self.key()) {
            if !matches!(
                fr,
                m::EntityKey::ProductDefinitionFormation(_)
                    | m::EntityKey::ProductDefinitionFormationWithSpecifiedSource(_)
            ) {
                continue;
            }
            targets.push(fr);
            for &pr in rg.referrers(fr) {
                if matches!(pr, m::EntityKey::ProductDefinition(_)) {
                    targets.push(pr);
                }
            }
        }
        targets
    }

    /// The people/organizations associated with this product across all its
    /// versions and definitions, each with the role they play.
    pub fn contributors(&self) -> Vec<Contributor<'m>> {
        contributors_of(self.cx, &self.metadata_targets())
    }

    /// The approvals anywhere in this product's tree (product, any formation, or
    /// any definition).
    pub fn approvals(&self) -> Vec<Approval<'m>> {
        approvals_of(self.cx, &self.metadata_targets())
    }

    /// The documents referenced anywhere in this product's tree.
    pub fn documents(&self) -> Vec<Document<'m>> {
        documents_of(self.cx, &self.metadata_targets())
    }

    /// The security classifications assigned anywhere in this product's tree.
    pub fn security_classifications(&self) -> Vec<SecurityClassification<'m>> {
        security_classifications_of(self.cx, &self.metadata_targets())
    }

    /// This product's versions — its `PRODUCT_DEFINITION_FORMATION`s (reverse:
    /// their `of_product` is this product). Usually one; a file may carry several
    /// revisions. Navigate `version.definitions()` to group definitions by version.
    pub fn versions(&self) -> Vec<Version<'m>> {
        let cx = self.cx;
        let rg = cx.ref_graph();
        let mut out: Vec<Version<'m>> = Vec::new();
        for r in rg.referrers(self.key()) {
            let which = match r {
                m::EntityKey::ProductDefinitionFormation(id) => FormImpl::Plain(*id),
                m::EntityKey::ProductDefinitionFormationWithSpecifiedSource(id) => {
                    FormImpl::WithSource(*id)
                }
                _ => continue,
            };
            out.push(Version { cx, which });
        }
        out
    }
}

/// A version (revision) of a product — a `PRODUCT_DEFINITION_FORMATION`.
///
/// Unifies the plain formation and the `WITH_SPECIFIED_SOURCE` variant, which
/// share id / description (its make-or-buy source is out of scope for now).
#[derive(Clone, Copy)]
pub struct Version<'m> {
    cx: Ctx<'m>,
    which: FormImpl,
}

#[derive(Clone, Copy)]
enum FormImpl {
    Plain(m::ProductDefinitionFormationId),
    WithSource(m::ProductDefinitionFormationWithSpecifiedSourceId),
}

impl<'m> Version<'m> {
    /// The shared id + description fields, resolved once so the accessors don't
    /// each repeat the backing-type match.
    fn fields(&self) -> (&'m str, Option<&'m str>) {
        let model = self.cx.model;
        match self.which {
            FormImpl::Plain(i) => {
                let f = model.product_definition_formation_arena.get(i.0);
                (&f.id, f.description.as_deref())
            }
            FormImpl::WithSource(i) => {
                let f = model
                    .product_definition_formation_with_specified_source_arena
                    .get(i.0);
                (&f.id, f.description.as_deref())
            }
        }
    }

    /// This version's global identity (a `Copy` key for maps / dedup).
    pub fn key(&self) -> m::EntityKey {
        match self.which {
            FormImpl::Plain(i) => m::EntityKey::ProductDefinitionFormation(i),
            FormImpl::WithSource(i) => {
                m::EntityKey::ProductDefinitionFormationWithSpecifiedSource(i)
            }
        }
    }

    /// The version label (`RevB`, `1`, `A`, …).
    pub fn id(&self) -> &'m str {
        self.fields().0
    }

    /// The version description, if any.
    pub fn description(&self) -> Option<&'m str> {
        self.fields().1
    }

    /// The definitions belonging to this version (reverse: a `PRODUCT_DEFINITION`
    /// whose `formation` is this version).
    pub fn definitions(&self) -> impl Iterator<Item = ProductDef<'m>> + 'm {
        let cx = self.cx;
        let rg = cx.ref_graph();
        let mut out: Vec<ProductDef<'m>> = Vec::new();
        for &r in rg.referrers(self.key()) {
            if let Some(d) = ProductDef::from_key(cx, r) {
                out.push(d);
            }
        }
        out.into_iter()
    }
}

// ---------------------------------------------------------------------------
// ProductDef
// ---------------------------------------------------------------------------

/// A product definition (`PRODUCT_DEFINITION`) — a view of a product that owns
/// shape and participates in the assembly tree.
#[derive(Clone, Copy)]
pub struct ProductDef<'m> {
    cx: Ctx<'m>,
    which: PdImpl,
}

/// A `PRODUCT_DEFINITION` or its `PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS`
/// subtype — both carry the same core fields (a documented part uses the
/// subtype) and are surfaced identically as assembly-tree nodes.
#[derive(Clone, Copy)]
enum PdImpl {
    Plain(m::ProductDefinitionId),
    WithDocs(m::ProductDefinitionWithAssociatedDocumentsId),
}

/// The `product_definition` core fields shared by both variants.
struct PdCore<'m> {
    id: &'m str,
    description: Option<&'m str>,
    formation: &'m m::ProductDefinitionFormationRef,
}

impl<'m> ProductDef<'m> {
    /// Wrap a `PRODUCT_DEFINITION`/`..._WITH_ASSOCIATED_DOCUMENTS` key as a
    /// definition handle; `None` for any other entity.
    fn from_key(cx: Ctx<'m>, key: m::EntityKey) -> Option<Self> {
        match key {
            m::EntityKey::ProductDefinition(i) => Some(Self {
                cx,
                which: PdImpl::Plain(i),
            }),
            m::EntityKey::ProductDefinitionWithAssociatedDocuments(i) => Some(Self {
                cx,
                which: PdImpl::WithDocs(i),
            }),
            _ => None,
        }
    }

    /// The shared core fields, read from whichever arena backs this definition.
    fn core(&self) -> PdCore<'m> {
        match self.which {
            PdImpl::Plain(i) => {
                let r = self.cx.model.product_definition_arena.get(i.0);
                PdCore {
                    id: &r.id,
                    description: r.description.as_deref(),
                    formation: &r.formation,
                }
            }
            PdImpl::WithDocs(i) => {
                let r = self
                    .cx
                    .model
                    .product_definition_with_associated_documents_arena
                    .get(i.0);
                PdCore {
                    id: &r.id,
                    description: r.description.as_deref(),
                    formation: &r.formation,
                }
            }
        }
    }

    /// This definition's global identity (a `Copy` key for maps / deduplication;
    /// distinct from [`ProductDef::id`], the STEP `id` string attribute).
    pub fn key(&self) -> m::EntityKey {
        match self.which {
            PdImpl::Plain(i) => m::EntityKey::ProductDefinition(i),
            PdImpl::WithDocs(i) => m::EntityKey::ProductDefinitionWithAssociatedDocuments(i),
        }
    }

    pub fn id(&self) -> &'m str {
        self.core().id
    }

    pub fn description(&self) -> Option<&'m str> {
        self.core().description
    }

    /// The product this definition belongs to (forward: `formation → of_product`).
    pub fn product(&self) -> Option<Product<'m>> {
        let model = self.cx.model;
        let of_product: &m::ProductRef = match self.core().formation {
            m::ProductDefinitionFormationRef::ProductDefinitionFormation(i) => {
                &model.product_definition_formation_arena.get(i.0).of_product
            }
            m::ProductDefinitionFormationRef::ProductDefinitionFormationWithSpecifiedSource(i) => {
                &model
                    .product_definition_formation_with_specified_source_arena
                    .get(i.0)
                    .of_product
            }
            m::ProductDefinitionFormationRef::Complex(_) => return None,
        };
        match of_product {
            m::ProductRef::Product(pid) => Some(Product {
                cx: self.cx,
                id: *pid,
            }),
            m::ProductRef::Complex(_) => None,
        }
    }

    /// The component occurrences directly under this definition — each a usage
    /// (`NAUO`) carrying the child definition *and* its placement transform
    /// (reverse: a `NAUO` whose `relating` is this definition).
    pub fn occurrences(&self) -> impl Iterator<Item = Occurrence<'m>> + 'm {
        let cx = self.cx;
        let rg = cx.ref_graph();
        let me = self.key();
        let mut out: Vec<Occurrence<'m>> = Vec::new();
        for r in rg.referrers(me) {
            if let m::EntityKey::NextAssemblyUsageOccurrence(nid) = r {
                let nauo = cx.model.next_assembly_usage_occurrence_arena.get(nid.0);
                if pdor_is(&nauo.relating_product_definition, me) {
                    out.push(Occurrence { cx, id: *nid });
                }
            }
        }
        out.into_iter()
    }

    /// Component definitions directly under this one in the assembly tree — the
    /// occurrences resolved to their child definitions (drops the placement).
    pub fn children(&self) -> impl Iterator<Item = ProductDef<'m>> + 'm {
        self.occurrences().filter_map(|o| o.definition())
    }

    /// Assembly definitions this one is a component of (reverse: a `NAUO` whose
    /// `related` is this definition → its `relating`).
    pub fn parents(&self) -> impl Iterator<Item = ProductDef<'m>> + 'm {
        let cx = self.cx;
        let rg = cx.ref_graph();
        let me = self.key();
        let mut out: Vec<ProductDef<'m>> = Vec::new();
        for r in rg.referrers(me) {
            if let m::EntityKey::NextAssemblyUsageOccurrence(nid) = r {
                let nauo = cx.model.next_assembly_usage_occurrence_arena.get(nid.0);
                if pdor_is(&nauo.related_product_definition, me) {
                    if let Some(d) = pdor_to_def(cx, &nauo.relating_product_definition) {
                        out.push(d);
                    }
                }
            }
        }
        out.into_iter()
    }

    /// The b-rep solids of this definition's shape — the bridge into the geometry
    /// handles. Reverse: a `PRODUCT_DEFINITION_SHAPE` referencing this definition →
    /// a `SHAPE_DEFINITION_REPRESENTATION` referencing that shape → forward through
    /// its representation's items to each `MANIFOLD_SOLID_BREP`. When the geometry
    /// lives in a separate representation bridged by a plain
    /// `SHAPE_REPRESENTATION_RELATIONSHIP` (the common AP242 assembly layout), the
    /// equivalence bridge is followed to collect those solids too.
    pub fn solids(&self) -> impl Iterator<Item = Solid<'m>> + 'm {
        let cx = self.cx;
        let rg = cx.ref_graph();
        let me = self.key();
        let mut out: Vec<Solid<'m>> = Vec::new();
        // Shared across every representation reached: guards relationship cycles
        // and keeps a shape-equivalence bridge from re-collecting a rep.
        let mut visited: HashSet<m::EntityKey> = HashSet::new();
        for r in rg.referrers(me) {
            let m::EntityKey::ProductDefinitionShape(pds_id) = r else {
                continue;
            };
            let pds = cx.model.product_definition_shape_arena.get(pds_id.0);
            if !cdef_is(&pds.definition, me) {
                continue;
            }
            for s in rg.referrers(m::EntityKey::ProductDefinitionShape(*pds_id)) {
                let m::EntityKey::ShapeDefinitionRepresentation(sdr_id) = s else {
                    continue;
                };
                let sdr = cx.model.shape_definition_representation_arena.get(sdr_id.0);
                if matches!(&sdr.definition, m::RepresentedDefinitionRef::ProductDefinitionShape(p) if *p == *pds_id)
                {
                    collect_solids_from_repr(cx, &sdr.used_representation, &mut out, &mut visited);
                }
            }
        }
        // A brep shared by two representations could be collected twice.
        let mut seen: HashSet<m::EntityKey> = HashSet::new();
        out.retain(|s| seen.insert(s.key()));
        out.into_iter()
    }

    /// The shape features of this part (`SHAPE_ASPECT`s whose `of_shape`
    /// resolves to this definition). The model-wide [`Scene::features`] filtered
    /// to this part.
    pub fn features(&self) -> impl Iterator<Item = pmi::Feature<'m>> + 'm {
        pmi::features_of(self.cx, self.key()).into_iter()
    }

    /// The dimensions of this part (those whose targeted feature is on it).
    pub fn dimensions(&self) -> impl Iterator<Item = pmi::Dimension<'m>> + 'm {
        pmi::dimensions_of(self.cx, self.key()).into_iter()
    }

    /// The geometric tolerances of this part (those whose target — a feature or
    /// the whole part — is on it).
    pub fn tolerances(&self) -> impl Iterator<Item = pmi::Tolerance<'m>> + 'm {
        pmi::tolerances_of(self.cx, self.key()).into_iter()
    }

    /// The datums of this part (`DATUM` / `COMMON_DATUM` whose `of_shape` is on it).
    pub fn datums(&self) -> impl Iterator<Item = pmi::Datum<'m>> + 'm {
        pmi::datums_of(self.cx, self.key()).into_iter()
    }

    /// The version identifier of this part — its `PRODUCT_DEFINITION_FORMATION.id`.
    pub fn version(&self) -> Option<&'m str> {
        let model = self.cx.model;
        match self.core().formation {
            m::ProductDefinitionFormationRef::ProductDefinitionFormation(i) => {
                Some(&model.product_definition_formation_arena.get(i.0).id)
            }
            m::ProductDefinitionFormationRef::ProductDefinitionFormationWithSpecifiedSource(i) => {
                Some(
                    &model
                        .product_definition_formation_with_specified_source_arena
                        .get(i.0)
                        .id,
                )
            }
            m::ProductDefinitionFormationRef::Complex(_) => None,
        }
    }

    /// The people / organizations involved with this part and their roles
    /// (creator, design owner, supplier, …), from the person-and-organization
    /// assignments targeting this definition, its product, or its formation.
    /// The identities a management assignment (person/org, approval, document)
    /// may target for this part: the definition, its product, and its formation.
    /// Ordered broadest-first (product, then formation, then definition) so the
    /// walkers' first-hit de-dup labels each result with its broadest scope.
    fn metadata_targets(&self) -> Vec<m::EntityKey> {
        let mut targets = Vec::new();
        if let Some(p) = self.product() {
            targets.push(p.key());
        }
        match self.core().formation {
            m::ProductDefinitionFormationRef::ProductDefinitionFormation(i) => {
                targets.push(m::EntityKey::ProductDefinitionFormation(*i));
            }
            m::ProductDefinitionFormationRef::ProductDefinitionFormationWithSpecifiedSource(i) => {
                targets.push(m::EntityKey::ProductDefinitionFormationWithSpecifiedSource(
                    *i,
                ));
            }
            m::ProductDefinitionFormationRef::Complex(_) => {}
        }
        targets.push(self.key());
        targets
    }

    /// The people/organizations assigned to this part's definition, product, or
    /// formation, each with the role they play.
    pub fn contributors(&self) -> Vec<Contributor<'m>> {
        contributors_of(self.cx, &self.metadata_targets())
    }

    /// The approvals on this part — the `APPROVAL`s assigned to its definition,
    /// product, or formation (each with a status, level, and approvers).
    pub fn approvals(&self) -> Vec<Approval<'m>> {
        approvals_of(self.cx, &self.metadata_targets())
    }

    /// The documents referenced by this part — drawings, specs, standards
    /// (reverse: an `APPLIED_DOCUMENT_REFERENCE` whose `items` include this
    /// definition, its product, or its formation), plus, for a
    /// `..._WITH_ASSOCIATED_DOCUMENTS` definition, the documents it carries
    /// directly in `documentation_ids`.
    pub fn documents(&self) -> Vec<Document<'m>> {
        let mut out = documents_of(self.cx, &self.metadata_targets());
        if let PdImpl::WithDocs(i) = self.which {
            let raw = self
                .cx
                .model
                .product_definition_with_associated_documents_arena
                .get(i.0);
            let scope = scope_of(self.key());
            for dref in &raw.documentation_ids {
                let which = match dref {
                    m::DocumentRef::Document(i) => DocImpl::Document(*i),
                    m::DocumentRef::DocumentFile(i) => DocImpl::DocumentFile(*i),
                    m::DocumentRef::Complex(_) => continue,
                };
                let doc = Document {
                    cx: self.cx,
                    which,
                    scope,
                };
                if !out.iter().any(|d| d.key() == doc.key()) {
                    out.push(doc);
                }
            }
        }
        out
    }

    /// The security classifications assigned to this part — its confidentiality
    /// level and why (assigned to its definition, product, or formation).
    pub fn security_classifications(&self) -> Vec<SecurityClassification<'m>> {
        security_classifications_of(self.cx, &self.metadata_targets())
    }
}

// ---------------------------------------------------------------------------
// Shared metadata walkers
//
// Metadata (contributors, approvals, documents, classifications) attaches to a
// part by *reverse* assignment: an assignment entity lists the definition,
// product, or formation in its `items`. Each family gathers its handles by
// scanning the referrers of a set of target keys; only the target set differs
// between a `ProductDef` (its own def + product + formation) and a `Product`
// (the whole product tree), so the walks are factored out here and parameterised
// by `targets`. Per-family de-dup/extraction stays distinct (a single generic
// walker doesn't fit — the de-dup keys differ).
//
// Each result is labelled with the `Scope` of the target it was reached through.
// Because `metadata_targets` is ordered broadest-first and the walkers keep the
// first hit, a multi-target assignment is labelled with its broadest scope.
// ---------------------------------------------------------------------------

/// Which level of the product structure a piece of metadata is attached to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Attached to the product itself — applies across every version.
    Product,
    /// Attached to a specific version (`PRODUCT_DEFINITION_FORMATION`).
    Version,
    /// Attached to a specific definition (`PRODUCT_DEFINITION`).
    Definition,
}

/// The scope of a metadata target key. Targets only ever come from
/// `metadata_targets`, which yields product / formation / definition keys.
fn scope_of(k: m::EntityKey) -> Scope {
    match k {
        m::EntityKey::Product(_) => Scope::Product,
        m::EntityKey::ProductDefinitionFormation(_)
        | m::EntityKey::ProductDefinitionFormationWithSpecifiedSource(_) => Scope::Version,
        _ => Scope::Definition,
    }
}

/// A single product-structure entity a piece of metadata is attached to — the
/// full detail behind [`Scope`]. Unlike [`Contributor::assigned_scope`] (which
/// answers *which level*), this names *which entity*, so multiple targets at the
/// same level stay distinct.
#[derive(Clone, Copy)]
pub enum Target<'m> {
    Product(Product<'m>),
    Version(Version<'m>),
    Definition(ProductDef<'m>),
}

impl Target<'_> {
    /// The structure level of this target.
    pub fn scope(&self) -> Scope {
        match self {
            Target::Product(_) => Scope::Product,
            Target::Version(_) => Scope::Version,
            Target::Definition(_) => Scope::Definition,
        }
    }
}

/// Map an assignment-item key to a [`Target`] handle. Items that are not
/// product-structure entities (product / formation / definition) yield `None`.
fn target_of(cx: Ctx<'_>, k: m::EntityKey) -> Option<Target<'_>> {
    Some(match k {
        m::EntityKey::Product(id) => Target::Product(Product { cx, id }),
        m::EntityKey::ProductDefinitionFormation(id) => Target::Version(Version {
            cx,
            which: FormImpl::Plain(id),
        }),
        m::EntityKey::ProductDefinitionFormationWithSpecifiedSource(id) => {
            Target::Version(Version {
                cx,
                which: FormImpl::WithSource(id),
            })
        }
        other => return ProductDef::from_key(cx, other).map(Target::Definition),
    })
}

/// Collect the distinct product-structure targets from a stream of item keys.
fn targets_from_keys<'m>(cx: Ctx<'m>, keys: impl Iterator<Item = m::EntityKey>) -> Vec<Target<'m>> {
    let mut out: Vec<Target<'m>> = Vec::new();
    let mut seen: Vec<m::EntityKey> = Vec::new();
    for k in keys {
        if seen.contains(&k) {
            continue;
        }
        seen.push(k);
        if let Some(t) = target_of(cx, k) {
            out.push(t);
        }
    }
    out
}

fn contributors_of<'m>(cx: Ctx<'m>, targets: &[m::EntityKey]) -> Vec<Contributor<'m>> {
    let rg = cx.ref_graph();
    let mut out: Vec<Contributor<'m>> = Vec::new();
    let mut seen: Vec<m::EntityKey> = Vec::new();
    for &t in targets {
        let scope = scope_of(t);
        for r in rg.referrers(t) {
            let which = match r {
                m::EntityKey::CcDesignPersonAndOrganizationAssignment(id) => {
                    ContributorImpl::CcDesign(*id)
                }
                m::EntityKey::AppliedPersonAndOrganizationAssignment(id) => {
                    ContributorImpl::Applied(*id)
                }
                _ => continue,
            };
            if seen.contains(r) {
                continue;
            }
            seen.push(*r);
            out.push(Contributor { cx, which, scope });
        }
    }
    out
}

fn approvals_of<'m>(cx: Ctx<'m>, targets: &[m::EntityKey]) -> Vec<Approval<'m>> {
    let rg = cx.ref_graph();
    let mut out: Vec<Approval<'m>> = Vec::new();
    let mut seen: Vec<m::ApprovalId> = Vec::new();
    for &t in targets {
        let scope = scope_of(t);
        for r in rg.referrers(t) {
            let m::ApprovalRef::Approval(aid) = match r {
                m::EntityKey::CcDesignApproval(id) => {
                    &cx.model
                        .cc_design_approval_arena
                        .get(id.0)
                        .assigned_approval
                }
                m::EntityKey::AppliedApprovalAssignment(id) => {
                    &cx.model
                        .applied_approval_assignment_arena
                        .get(id.0)
                        .assigned_approval
                }
                _ => continue,
            };
            if seen.contains(aid) {
                continue;
            }
            seen.push(*aid);
            out.push(Approval {
                cx,
                id: *aid,
                scope,
            });
        }
    }
    out
}

/// A complex `assigned_document` cannot be resolved to a document handle — it is
/// skipped and surfaced through [`Scene::warnings`](crate::scene::Scene::warnings).
fn documents_of<'m>(cx: Ctx<'m>, targets: &[m::EntityKey]) -> Vec<Document<'m>> {
    let rg = cx.ref_graph();
    let mut out: Vec<Document<'m>> = Vec::new();
    let mut seen: Vec<m::EntityKey> = Vec::new();
    for &t in targets {
        let scope = scope_of(t);
        for r in rg.referrers(t) {
            let m::EntityKey::AppliedDocumentReference(id) = r else {
                continue;
            };
            let which = match &cx
                .model
                .applied_document_reference_arena
                .get(id.0)
                .assigned_document
            {
                m::DocumentRef::Document(i) => DocImpl::Document(*i),
                m::DocumentRef::DocumentFile(i) => DocImpl::DocumentFile(*i),
                m::DocumentRef::Complex(_) => {
                    cx.warn(
                        "APPLIED_DOCUMENT_REFERENCE.assigned_document is a complex instance; \
                         document left unread"
                            .to_owned(),
                    );
                    continue;
                }
            };
            let doc = Document { cx, which, scope };
            let k = doc.key();
            if seen.contains(&k) {
                continue;
            }
            seen.push(k);
            out.push(doc);
        }
    }
    out
}

fn security_classifications_of<'m>(
    cx: Ctx<'m>,
    targets: &[m::EntityKey],
) -> Vec<SecurityClassification<'m>> {
    let rg = cx.ref_graph();
    let mut out: Vec<SecurityClassification<'m>> = Vec::new();
    let mut seen: Vec<m::SecurityClassificationId> = Vec::new();
    for &t in targets {
        let scope = scope_of(t);
        for r in rg.referrers(t) {
            let m::SecurityClassificationRef::SecurityClassification(sid) = match r {
                m::EntityKey::CcDesignSecurityClassification(id) => {
                    &cx.model
                        .cc_design_security_classification_arena
                        .get(id.0)
                        .assigned_security_classification
                }
                m::EntityKey::AppliedSecurityClassificationAssignment(id) => {
                    &cx.model
                        .applied_security_classification_assignment_arena
                        .get(id.0)
                        .assigned_security_classification
                }
                _ => continue,
            };
            if seen.contains(sid) {
                continue;
            }
            seen.push(*sid);
            out.push(SecurityClassification {
                cx,
                id: *sid,
                scope,
            });
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Contributor / Person (management metadata: who is involved with a part)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum ContributorImpl {
    CcDesign(m::CcDesignPersonAndOrganizationAssignmentId),
    Applied(m::AppliedPersonAndOrganizationAssignmentId),
}

/// A person + organization assigned to a part, with the role they play
/// (`creator`, `design_owner`, `part_supplier`, …).
#[derive(Clone, Copy)]
pub struct Contributor<'m> {
    cx: Ctx<'m>,
    which: ContributorImpl,
    scope: Scope,
}

impl<'m> Contributor<'m> {
    /// Which level of the product structure this assignment attaches to.
    pub fn assigned_scope(&self) -> Scope {
        self.scope
    }

    /// The exact product-structure entities this assignment attaches to (its
    /// `items`) — versions and definitions stay distinct even at the same level.
    pub fn assigned_targets(&self) -> Vec<Target<'m>> {
        let cx = self.cx;
        let keys: Vec<m::EntityKey> = match self.which {
            ContributorImpl::CcDesign(i) => cx
                .model
                .cc_design_person_and_organization_assignment_arena
                .get(i.0)
                .items
                .iter()
                .map(m::CcPersonOrganizationItemRef::entity_key)
                .collect(),
            ContributorImpl::Applied(i) => cx
                .model
                .applied_person_and_organization_assignment_arena
                .get(i.0)
                .items
                .iter()
                .map(m::PersonAndOrganizationItemRef::entity_key)
                .collect(),
        };
        targets_from_keys(cx, keys.into_iter())
    }

    /// The assignment's person-and-organization and role ids (both refs are
    /// single-variant, so they always resolve).
    fn ids(&self) -> (m::PersonAndOrganizationId, m::PersonAndOrganizationRoleId) {
        let model = self.cx.model;
        let (pao, role) = match self.which {
            ContributorImpl::CcDesign(i) => {
                let a = model
                    .cc_design_person_and_organization_assignment_arena
                    .get(i.0);
                (&a.assigned_person_and_organization, &a.role)
            }
            ContributorImpl::Applied(i) => {
                let a = model
                    .applied_person_and_organization_assignment_arena
                    .get(i.0);
                (&a.assigned_person_and_organization, &a.role)
            }
        };
        let m::PersonAndOrganizationRef::PersonAndOrganization(pid) = pao;
        let m::PersonAndOrganizationRoleRef::PersonAndOrganizationRole(rid) = role;
        (*pid, *rid)
    }

    /// The role this contributor plays (`creator`, `design_owner`, …).
    pub fn role(&self) -> &'m str {
        let (_, rid) = self.ids();
        &self
            .cx
            .model
            .person_and_organization_role_arena
            .get(rid.0)
            .name
    }

    /// The person.
    pub fn person(&self) -> Person<'m> {
        let (pao, _) = self.ids();
        let m::PersonRef::Person(pid) = &self
            .cx
            .model
            .person_and_organization_arena
            .get(pao.0)
            .the_person;
        Person {
            cx: self.cx,
            id: *pid,
        }
    }

    /// The organization's name.
    pub fn organization(&self) -> &'m str {
        let (pao, _) = self.ids();
        let m::OrganizationRef::Organization(oid) = &self
            .cx
            .model
            .person_and_organization_arena
            .get(pao.0)
            .the_organization;
        &self.cx.model.organization_arena.get(oid.0).name
    }
}

/// A person (`PERSON`).
#[derive(Clone, Copy)]
pub struct Person<'m> {
    cx: Ctx<'m>,
    id: m::PersonId,
}

impl<'m> Person<'m> {
    fn raw(&self) -> &'m m::Person {
        self.cx.model.person_arena.get(self.id.0)
    }

    /// This person's global identity (a `Copy` key for maps / dedup).
    pub fn key(&self) -> m::EntityKey {
        m::EntityKey::Person(self.id)
    }

    /// The person's STEP `id`.
    pub fn id(&self) -> &'m str {
        &self.raw().id
    }

    /// The person's given name, if recorded.
    pub fn first_name(&self) -> Option<&'m str> {
        self.raw().first_name.as_deref()
    }

    /// The person's family name, if recorded.
    pub fn last_name(&self) -> Option<&'m str> {
        self.raw().last_name.as_deref()
    }
}

// ---------------------------------------------------------------------------
// Occurrence (assembly component usage) + placement transform
// ---------------------------------------------------------------------------

/// A coordinate frame (`AXIS2_PLACEMENT_3D`): an origin plus the local Z axis
/// and X reference direction (Y is `Z × X`). Omitted axes use the STEP defaults.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub origin: [f64; 3],
    pub axis: [f64; 3],
    pub ref_direction: [f64; 3],
}

/// An item-defined transformation as two frames: the relative transform maps
/// `from` (`transform_item_1`) to `to` (`transform_item_2`) — the faithful
/// two-frame form; [`Transform::matrix`] composes them on demand.
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub from: Placement,
    pub to: Placement,
}

// Small vector helpers for the frame math (local on purpose: the nurbs module
// keeps its own private set, and a shared math module would be more coupling
// than these one-liners are worth).
fn vdot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn vsub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn vscale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn vcross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn vnorm(a: [f64; 3]) -> Option<[f64; 3]> {
    let n = vdot(a, a).sqrt();
    (n >= 1e-12).then(|| vscale(a, 1.0 / n))
}

impl Placement {
    /// The orthonormal frame `(x, y, z)` this placement defines: z is the
    /// axis, x is the reference direction projected orthogonal to z (the STEP
    /// derivation), y completes the right-handed set. `None` for a degenerate
    /// placement (a zero axis, or a reference direction parallel to the axis).
    fn frame(&self) -> Option<([f64; 3], [f64; 3], [f64; 3])> {
        let z = vnorm(self.axis)?;
        let x = vnorm(vsub(
            self.ref_direction,
            vscale(z, vdot(self.ref_direction, z)),
        ))?;
        Some((x, vcross(z, x), z))
    }
}

impl Transform {
    /// The relative placement composed into one homogeneous 4×4 matrix — an
    /// opt-in derived view of the two stored frames (`M = F_to · F_from⁻¹`:
    /// geometry defined in the `from` frame lands in the `to` frame).
    ///
    /// Convention: `m[row][col]`, points map as column vectors `p' = M · p`,
    /// and the last row is `[0, 0, 0, 1]` (STEP placements carry rotation and
    /// translation only). `None` if either placement is degenerate (a zero
    /// axis, or a reference direction parallel to the axis) — the matrix is
    /// not fabricated from bad frames.
    // Matrix index arithmetic reads clearest with explicit indices.
    #[allow(clippy::needless_range_loop)]
    pub fn matrix(&self) -> Option<[[f64; 4]; 4]> {
        let rf = self.from.frame()?; // columns of R_from
        let rt = self.to.frame()?; // columns of R_to
        let (rf, rt) = ([rf.0, rf.1, rf.2], [rt.0, rt.1, rt.2]);
        let mut m = [[0.0_f64; 4]; 4];
        // R = R_to · R_fromᵀ (an orthonormal frame inverts as its transpose).
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] = (0..3).map(|k| rt[k][i] * rf[k][j]).sum();
            }
        }
        // t = o_to − R · o_from
        for i in 0..3 {
            m[i][3] =
                self.to.origin[i] - (0..3).map(|j| m[i][j] * self.from.origin[j]).sum::<f64>();
        }
        m[3][3] = 1.0;
        Some(m)
    }
}

/// A component usage in an assembly (`NEXT_ASSEMBLY_USAGE_OCCURRENCE`): the
/// child definition plus where it is placed.
#[derive(Clone, Copy)]
pub struct Occurrence<'m> {
    cx: Ctx<'m>,
    id: m::NextAssemblyUsageOccurrenceId,
}

impl<'m> Occurrence<'m> {
    fn raw(&self) -> &'m m::NextAssemblyUsageOccurrence {
        self.cx
            .model
            .next_assembly_usage_occurrence_arena
            .get(self.id.0)
    }

    /// This occurrence's global identity (a `Copy` key for maps / deduplication).
    pub fn key(&self) -> m::EntityKey {
        m::EntityKey::NextAssemblyUsageOccurrence(self.id)
    }

    /// The component definition this occurrence places (forward: `related`).
    pub fn definition(&self) -> Option<ProductDef<'m>> {
        pdor_to_def(self.cx, &self.raw().related_product_definition)
    }

    /// The component's placement transform in the assembly, if present (reverse:
    /// a `PRODUCT_DEFINITION_SHAPE` defined by this NAUO → a
    /// `CONTEXT_DEPENDENT_SHAPE_REPRESENTATION` → its transformation operator's
    /// two `AXIS2_PLACEMENT_3D` frames). A transform item that is not an
    /// `AXIS2_PLACEMENT_3D` is reported via [`Scene::warnings`](crate::scene::Scene)
    /// and yields `None` rather than a silently wrong frame.
    pub fn transform(&self) -> Option<Transform> {
        let cx = self.cx;
        let rg = cx.ref_graph();
        let me = self.id;
        for r in rg.referrers(m::EntityKey::NextAssemblyUsageOccurrence(me)) {
            let m::EntityKey::ProductDefinitionShape(pds_id) = r else {
                continue;
            };
            let pds = cx.model.product_definition_shape_arena.get(pds_id.0);
            if !matches!(&pds.definition, m::CharacterizedDefinitionRef::NextAssemblyUsageOccurrence(n) if *n == me)
            {
                continue;
            }
            for c in rg.referrers(m::EntityKey::ProductDefinitionShape(*pds_id)) {
                let m::EntityKey::ContextDependentShapeRepresentation(cdsr_id) = c else {
                    continue;
                };
                let cdsr = cx
                    .model
                    .context_dependent_shape_representation_arena
                    .get(cdsr_id.0);
                if !matches!(&cdsr.represented_product_relation, m::ProductDefinitionShapeRef::ProductDefinitionShape(p) if *p == *pds_id)
                {
                    continue;
                }
                if let Some(idt) = idt_of_cdsr(cx, &cdsr.representation_relation) {
                    let from = resolve_placement(cx, &idt.transform_item_1)?;
                    let to = resolve_placement(cx, &idt.transform_item_2)?;
                    return Some(Transform { from, to });
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// MappedInstance (MAPPED_ITEM placement)
// ---------------------------------------------------------------------------

/// Model-wide mapped-item enumeration.
impl Scene<'_> {
    /// Every `MAPPED_ITEM` in the model — the second placement mechanism
    /// besides [`Occurrence`]: a representation's geometry stamped from its
    /// origin frame onto a target frame (assembly component reuse, and
    /// drawing/PMI view instancing).
    pub fn all_mapped_instances(&self) -> impl Iterator<Item = MappedInstance<'_>> + '_ {
        let cx = self.ctx();
        (0..cx.model.mapped_item_arena.items.len()).map(move |i| MappedInstance {
            cx,
            id: m::MappedItemId(i),
        })
    }
}

/// One `MAPPED_ITEM`: an instance of a source representation's geometry,
/// placed by a pair of frames (the map's origin and this item's target).
#[derive(Clone, Copy)]
pub struct MappedInstance<'m> {
    cx: Ctx<'m>,
    id: m::MappedItemId,
}

impl<'m> MappedInstance<'m> {
    fn raw(&self) -> &'m m::MappedItem {
        self.cx.model.mapped_item_arena.get(self.id.0)
    }

    pub fn name(&self) -> &'m str {
        &self.raw().name
    }

    /// This instance's global identity (a `Copy` key for maps / deduplication).
    pub fn key(&self) -> m::EntityKey {
        m::EntityKey::MappedItem(self.id)
    }

    fn map(&self) -> Option<&'m m::RepresentationMap> {
        let m::RepresentationMapRef::RepresentationMap(id) = &self.raw().mapping_source else {
            self.cx
                .warn("MAPPED_ITEM.mapping_source is not a REPRESENTATION_MAP".to_owned());
            return None;
        };
        Some(self.cx.model.representation_map_arena.get(id.0))
    }

    /// The placement as two frames, like [`Occurrence::transform`]: the source
    /// geometry is defined relative to `from` (the map's origin) and placed at
    /// `to` (this item's target). A frame that is not an `AXIS2_PLACEMENT_3D`
    /// (a camera origin, a 2D drawing target) is reported via
    /// [`Scene::warnings`](crate::scene::Scene) and yields `None`.
    pub fn transform(&self) -> Option<Transform> {
        let cx = self.cx;
        let map = self.map()?;
        let from = resolve_placement(cx, &map.mapping_origin)?;
        let to = resolve_placement(cx, &self.raw().mapping_target)?;
        Some(Transform { from, to })
    }

    /// The b-rep solids of the source representation — the geometry this
    /// instance stamps. A non-shape source (a presentation view) has none.
    pub fn solids(&self) -> Vec<Solid<'m>> {
        let cx = self.cx;
        let Some(map) = self.map() else {
            return Vec::new();
        };
        rep_items(cx, &map.mapped_representation)
            .into_iter()
            .flatten()
            .filter_map(|it| match it {
                m::RepresentationItemRef::ManifoldSolidBrep(id) => Some(Solid::from_id(cx, *id)),
                m::RepresentationItemRef::BrepWithVoids(id) => Some(Solid::from_void_id(cx, *id)),
                _ => None,
            })
            .collect()
    }

    /// Nested mapped items inside the source representation — one level of an
    /// assembly chain; walk them to traverse the instance tree.
    pub fn mapped_children(&self) -> Vec<MappedInstance<'m>> {
        let cx = self.cx;
        let Some(map) = self.map() else {
            return Vec::new();
        };
        rep_items(cx, &map.mapped_representation)
            .into_iter()
            .flatten()
            .filter_map(|it| match it {
                m::RepresentationItemRef::MappedItem(id) => Some(MappedInstance { cx, id: *id }),
                _ => None,
            })
            .collect()
    }
}

/// The `items` of a shape-carrying representation (the kinds mapped items
/// point at in practice); other representation kinds carry no b-rep items.
fn rep_items<'m>(cx: Ctx<'m>, r: &m::RepresentationRef) -> Option<&'m [m::RepresentationItemRef]> {
    match r {
        m::RepresentationRef::ShapeRepresentation(id) => {
            Some(&cx.model.shape_representation_arena.get(id.0).items)
        }
        m::RepresentationRef::AdvancedBrepShapeRepresentation(id) => Some(
            &cx.model
                .advanced_brep_shape_representation_arena
                .get(id.0)
                .items,
        ),
        m::RepresentationRef::GeometricallyBoundedWireframeShapeRepresentation(id) => Some(
            &cx.model
                .geometrically_bounded_wireframe_shape_representation_arena
                .get(id.0)
                .items,
        ),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The item-defined transformation behind a context-dependent shape
/// representation: its `representation_relation` is the complex
/// `(…, REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION, …)` whose operator is an
/// `ITEM_DEFINED_TRANSFORMATION`.
fn idt_of_cdsr<'m>(
    cx: Ctx<'m>,
    rr: &m::ShapeRepresentationRelationshipRef,
) -> Option<&'m m::ItemDefinedTransformation> {
    let m::ShapeRepresentationRelationshipRef::Complex(cuid) = rr else {
        return None;
    };
    let op = cx
        .model
        .complex_unit_arena
        .get(cuid.0)
        .parts
        .iter()
        .find_map(|p| match p {
            m::UnitPart::RepresentationRelationshipWithTransformation {
                transformation_operator,
            } => Some(transformation_operator),
            _ => None,
        })?;
    let m::TransformationRef::ItemDefinedTransformation(id) = op else {
        return None;
    };
    Some(cx.model.item_defined_transformation_arena.get(id.0))
}

/// Resolve a transform item to a [`Placement`]. Only `AXIS2_PLACEMENT_3D` is a
/// full 3D frame; anything else is recorded as a warning and yields `None`
/// (the other placement kinds store values in their own coordinate system, so
/// padding them with default axes would be wrong).
fn resolve_placement(cx: Ctx<'_>, item: &m::RepresentationItemRef) -> Option<Placement> {
    let m::RepresentationItemRef::Axis2Placement3d(id) = item else {
        cx.warn("transform item is not an AXIS2_PLACEMENT_3D".to_owned());
        return None;
    };
    let a = cx.model.axis2_placement3d_arena.get(id.0);
    Some(Placement {
        origin: point3(cx, &a.location),
        axis: a.axis.as_ref().map_or([0.0, 0.0, 1.0], |d| dir3(cx, d)),
        ref_direction: a
            .ref_direction
            .as_ref()
            .map_or([1.0, 0.0, 0.0], |d| dir3(cx, d)),
    })
}

/// A cartesian point's coordinates as a 3-vector (missing components are 0).
fn point3(cx: Ctx<'_>, r: &m::CartesianPointRef) -> [f64; 3] {
    if let m::CartesianPointRef::CartesianPoint(id) = r {
        let c = &cx.model.cartesian_point_arena.get(id.0).coordinates;
        [
            c.first().copied().unwrap_or(0.0),
            c.get(1).copied().unwrap_or(0.0),
            c.get(2).copied().unwrap_or(0.0),
        ]
    } else {
        [0.0; 3]
    }
}

/// A direction's ratios as a 3-vector (missing components are 0).
fn dir3(cx: Ctx<'_>, r: &m::DirectionRef) -> [f64; 3] {
    if let m::DirectionRef::Direction(id) = r {
        let c = &cx.model.direction_arena.get(id.0).direction_ratios;
        [
            c.first().copied().unwrap_or(0.0),
            c.get(1).copied().unwrap_or(0.0),
            c.get(2).copied().unwrap_or(0.0),
        ]
    } else {
        [0.0; 3]
    }
}

/// Whether a `ProductDefinitionOrReferenceRef` points at exactly this definition
/// (plain or `..._WITH_ASSOCIATED_DOCUMENTS`).
fn pdor_is(r: &m::ProductDefinitionOrReferenceRef, key: m::EntityKey) -> bool {
    match r {
        m::ProductDefinitionOrReferenceRef::ProductDefinition(i) => {
            m::EntityKey::ProductDefinition(*i) == key
        }
        m::ProductDefinitionOrReferenceRef::ProductDefinitionWithAssociatedDocuments(i) => {
            m::EntityKey::ProductDefinitionWithAssociatedDocuments(*i) == key
        }
        _ => false,
    }
}

/// Resolve a `ProductDefinitionOrReferenceRef` to a definition handle (plain or
/// `..._WITH_ASSOCIATED_DOCUMENTS`; occurrence / generic-reference / complex out
/// of scope).
fn pdor_to_def<'m>(cx: Ctx<'m>, r: &m::ProductDefinitionOrReferenceRef) -> Option<ProductDef<'m>> {
    match r {
        m::ProductDefinitionOrReferenceRef::ProductDefinition(i) => {
            ProductDef::from_key(cx, m::EntityKey::ProductDefinition(*i))
        }
        m::ProductDefinitionOrReferenceRef::ProductDefinitionWithAssociatedDocuments(i) => {
            ProductDef::from_key(
                cx,
                m::EntityKey::ProductDefinitionWithAssociatedDocuments(*i),
            )
        }
        _ => None,
    }
}

/// Whether a `CharacterizedDefinitionRef` (a `PRODUCT_DEFINITION_SHAPE.definition`)
/// points at exactly this definition (plain or `..._WITH_ASSOCIATED_DOCUMENTS`).
fn cdef_is(cd: &m::CharacterizedDefinitionRef, key: m::EntityKey) -> bool {
    match cd {
        m::CharacterizedDefinitionRef::ProductDefinition(i) => {
            m::EntityKey::ProductDefinition(*i) == key
        }
        m::CharacterizedDefinitionRef::ProductDefinitionWithAssociatedDocuments(i) => {
            m::EntityKey::ProductDefinitionWithAssociatedDocuments(*i) == key
        }
        _ => false,
    }
}

/// Push every `MANIFOLD_SOLID_BREP` reachable from a representation's `items`,
/// following plain `SHAPE_REPRESENTATION_RELATIONSHIP` (shape equivalence) to
/// bridged representations. The common assembly export puts a component's
/// placement in one `SHAPE_REPRESENTATION` and its geometry in a separate
/// `ADVANCED_BREP_SHAPE_REPRESENTATION`, linked by such a relationship. The
/// `WITH_TRANSFORMATION` variant is a complex instance (not in this arena), so
/// occurrence placement is never followed here. `visited` guards cycles.
fn collect_solids_from_repr<'m>(
    cx: Ctx<'m>,
    r: &m::RepresentationRef,
    out: &mut Vec<Solid<'m>>,
    visited: &mut HashSet<m::EntityKey>,
) {
    let (key, items): (m::EntityKey, &[m::RepresentationItemRef]) = match r {
        m::RepresentationRef::ShapeRepresentation(i) => (
            m::EntityKey::ShapeRepresentation(*i),
            &cx.model.shape_representation_arena.get(i.0).items,
        ),
        m::RepresentationRef::AdvancedBrepShapeRepresentation(i) => (
            m::EntityKey::AdvancedBrepShapeRepresentation(*i),
            &cx.model
                .advanced_brep_shape_representation_arena
                .get(i.0)
                .items,
        ),
        _ => return,
    };
    if !visited.insert(key) {
        return;
    }
    for it in items {
        match it {
            m::RepresentationItemRef::ManifoldSolidBrep(sid) => out.push(Solid::from_id(cx, *sid)),
            m::RepresentationItemRef::BrepWithVoids(sid) => {
                out.push(Solid::from_void_id(cx, *sid));
            }
            _ => {}
        }
    }
    // Hop across plain shape-equivalence relationships to the other endpoint.
    let rg = cx.ref_graph();
    for referrer in rg.referrers(key) {
        let m::EntityKey::ShapeRepresentationRelationship(srr_id) = referrer else {
            continue;
        };
        let srr = cx
            .model
            .shape_representation_relationship_arena
            .get(srr_id.0);
        let (k1, k2) = (srr.rep_1.entity_key(), srr.rep_2.entity_key());
        let other = if k1 == key {
            k2
        } else if k2 == key {
            k1
        } else {
            continue;
        };
        if let Ok(other_ref) = m::RepresentationRef::from_any(other) {
            collect_solids_from_repr(cx, &other_ref, out, visited);
        }
    }
}

// ---------------------------------------------------------------------------
// Approval / Approver (management metadata: a part's approval state and who
// authorised it)
// ---------------------------------------------------------------------------

/// An approval on a part (`APPROVAL`) — its status, level, and approvers.
#[derive(Clone, Copy)]
pub struct Approval<'m> {
    cx: Ctx<'m>,
    id: m::ApprovalId,
    scope: Scope,
}

impl<'m> Approval<'m> {
    fn raw(&self) -> &'m m::Approval {
        self.cx.model.approval_arena.get(self.id.0)
    }

    /// This approval's global identity (a `Copy` key for maps / dedup).
    pub fn key(&self) -> m::EntityKey {
        m::EntityKey::Approval(self.id)
    }

    /// Which level of the product structure this approval attaches to (distinct
    /// from [`Approval::level`], the approval's own status level).
    pub fn assigned_scope(&self) -> Scope {
        self.scope
    }

    /// The exact product-structure entities this approval is assigned to (across
    /// every assignment that carries it).
    pub fn assigned_targets(&self) -> Vec<Target<'m>> {
        let cx = self.cx;
        let mut keys: Vec<m::EntityKey> = Vec::new();
        for r in cx.ref_graph().referrers(self.key()) {
            match r {
                m::EntityKey::CcDesignApproval(i) => keys.extend(
                    cx.model
                        .cc_design_approval_arena
                        .get(i.0)
                        .items
                        .iter()
                        .map(m::ApprovedItemRef::entity_key),
                ),
                m::EntityKey::AppliedApprovalAssignment(i) => keys.extend(
                    cx.model
                        .applied_approval_assignment_arena
                        .get(i.0)
                        .items
                        .iter()
                        .map(m::ApprovalItemRef::entity_key),
                ),
                _ => {}
            }
        }
        targets_from_keys(cx, keys.into_iter())
    }

    /// The approval status name (`approved`, `not_yet_approved`, …).
    pub fn status(&self) -> &'m str {
        let m::ApprovalStatusRef::ApprovalStatus(sid) = &self.raw().status;
        &self.cx.model.approval_status_arena.get(sid.0).name
    }

    /// The approval level.
    pub fn level(&self) -> &'m str {
        &self.raw().level
    }

    /// Who authorised this approval, with their role (reverse: an
    /// `APPROVAL_PERSON_ORGANIZATION` whose `authorized_approval` is this approval).
    pub fn approvers(&self) -> Vec<Approver<'m>> {
        let cx = self.cx;
        let mut out: Vec<Approver<'m>> = Vec::new();
        for r in cx.ref_graph().referrers(self.key()) {
            if let m::EntityKey::ApprovalPersonOrganization(id) = r {
                out.push(Approver { cx, id: *id });
            }
        }
        out
    }

    /// When this approval was made (reverse: an `APPROVAL_DATE_TIME` whose
    /// `dated_approval` is this approval), flattened into an [`ApprovalDate`].
    ///
    /// `None` when no date is recorded. A top-level complex date instance cannot
    /// be decoded into calendar/clock fields — it also yields `None`, but is
    /// surfaced through [`Scene::warnings`](crate::scene::Scene::warnings) so the
    /// "no date" and "unreadable date" cases stay distinguishable.
    pub fn date(&self) -> Option<ApprovalDate> {
        let cx = self.cx;
        for r in cx.ref_graph().referrers(self.key()) {
            if let m::EntityKey::ApprovalDateTime(id) = r {
                let dt = &cx.model.approval_date_time_arena.get(id.0).date_time;
                return resolve_datetime(cx, dt);
            }
        }
        None
    }
}

/// A calendar/clock instant flattened out of the STEP `date_time_select` union
/// (`CALENDAR_DATE` / `DATE` / `DATE_AND_TIME` / `LOCAL_TIME`). Components absent
/// from the source stay `None`; seconds and time-zone are intentionally out of
/// scope for now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ApprovalDate {
    pub year: Option<i64>,
    pub month: Option<i64>,
    pub day: Option<i64>,
    pub hour: Option<i64>,
    pub minute: Option<i64>,
}

/// Flatten a `date_time_select` into an [`ApprovalDate`]. Every non-complex arm
/// fills at least one component (calendar dates carry year/month/day, local
/// times carry the hour), so an all-`None` result only arises from a top-level
/// complex instance — which is reported as `None` (see [`Approval::date`]).
fn resolve_datetime(cx: Ctx<'_>, r: &m::DateTimeSelectRef) -> Option<ApprovalDate> {
    let mut d = ApprovalDate::default();
    match r {
        m::DateTimeSelectRef::CalendarDate(i) => set_calendar(cx, &mut d, *i),
        m::DateTimeSelectRef::Date(i) => {
            d.year = Some(cx.model.date_arena.get(i.0).year_component);
        }
        m::DateTimeSelectRef::DateAndTime(i) => {
            let dt = cx.model.date_and_time_arena.get(i.0);
            set_date(cx, &mut d, &dt.date_component);
            let m::LocalTimeRef::LocalTime(lt) = &dt.time_component;
            set_time(cx, &mut d, *lt);
        }
        m::DateTimeSelectRef::LocalTime(i) => set_time(cx, &mut d, *i),
        m::DateTimeSelectRef::Complex(_) => {
            cx.warn(
                "APPROVAL_DATE_TIME.date_time is a complex instance; date left unread".to_owned(),
            );
            return None;
        }
    }
    Some(d)
}

/// Set year/month/day from a `date_select` (a complex date part is left unread).
fn set_date(cx: Ctx<'_>, d: &mut ApprovalDate, r: &m::DateRef) {
    match r {
        m::DateRef::CalendarDate(i) => set_calendar(cx, d, *i),
        m::DateRef::Date(i) => d.year = Some(cx.model.date_arena.get(i.0).year_component),
        m::DateRef::Complex(_) => {}
    }
}

fn set_calendar(cx: Ctx<'_>, d: &mut ApprovalDate, i: m::CalendarDateId) {
    let c = cx.model.calendar_date_arena.get(i.0);
    d.year = Some(c.year_component);
    d.month = Some(c.month_component);
    d.day = Some(c.day_component);
}

fn set_time(cx: Ctx<'_>, d: &mut ApprovalDate, i: m::LocalTimeId) {
    let t = cx.model.local_time_arena.get(i.0);
    d.hour = Some(t.hour_component);
    d.minute = t.minute_component;
}

/// The person / organization that authorised an approval, and their role.
#[derive(Clone, Copy)]
pub struct Approver<'m> {
    cx: Ctx<'m>,
    id: m::ApprovalPersonOrganizationId,
}

impl<'m> Approver<'m> {
    fn raw(&self) -> &'m m::ApprovalPersonOrganization {
        self.cx
            .model
            .approval_person_organization_arena
            .get(self.id.0)
    }

    /// The approver's role (`approver`, `security_approval`, …).
    pub fn role(&self) -> &'m str {
        let m::ApprovalRoleRef::ApprovalRole(rid) = &self.raw().role;
        &self.cx.model.approval_role_arena.get(rid.0).role
    }

    /// The approver's person, if the approver is a person or a
    /// person-and-organization.
    pub fn person(&self) -> Option<Person<'m>> {
        let pid = match &self.raw().person_organization {
            m::PersonOrganizationSelectRef::Person(i) => *i,
            m::PersonOrganizationSelectRef::PersonAndOrganization(i) => {
                let m::PersonRef::Person(p) = &self
                    .cx
                    .model
                    .person_and_organization_arena
                    .get(i.0)
                    .the_person;
                *p
            }
            m::PersonOrganizationSelectRef::Organization(_) => return None,
        };
        Some(Person {
            cx: self.cx,
            id: pid,
        })
    }

    /// The approver's organization name, if the approver is an organization or a
    /// person-and-organization.
    pub fn organization(&self) -> Option<&'m str> {
        let oid = match &self.raw().person_organization {
            m::PersonOrganizationSelectRef::Organization(i) => *i,
            m::PersonOrganizationSelectRef::PersonAndOrganization(i) => {
                let m::OrganizationRef::Organization(o) = &self
                    .cx
                    .model
                    .person_and_organization_arena
                    .get(i.0)
                    .the_organization;
                *o
            }
            m::PersonOrganizationSelectRef::Person(_) => return None,
        };
        Some(&self.cx.model.organization_arena.get(oid.0).name)
    }
}

#[derive(Clone, Copy)]
enum DocImpl {
    Document(m::DocumentId),
    DocumentFile(m::DocumentFileId),
}

/// A document referenced by a part — a drawing, specification, or standard.
///
/// Unifies STEP's `DOCUMENT` and `DOCUMENT_FILE`, which share id / name /
/// description / kind (the file-specific fields of `DOCUMENT_FILE` are out of
/// scope for now).
#[derive(Clone, Copy)]
pub struct Document<'m> {
    cx: Ctx<'m>,
    which: DocImpl,
    scope: Scope,
}

impl<'m> Document<'m> {
    /// Which level of the product structure this document reference attaches to.
    pub fn assigned_scope(&self) -> Scope {
        self.scope
    }

    /// The exact product-structure entities this document is referenced from
    /// (across every `APPLIED_DOCUMENT_REFERENCE` that carries it).
    pub fn assigned_targets(&self) -> Vec<Target<'m>> {
        let cx = self.cx;
        let mut keys: Vec<m::EntityKey> = Vec::new();
        for r in cx.ref_graph().referrers(self.key()) {
            if let m::EntityKey::AppliedDocumentReference(i) = r {
                keys.extend(
                    cx.model
                        .applied_document_reference_arena
                        .get(i.0)
                        .items
                        .iter()
                        .map(m::DocumentReferenceItemRef::entity_key),
                );
            }
        }
        targets_from_keys(cx, keys.into_iter())
    }

    /// The four shared fields (id, name, description, kind), resolved once so the
    /// public accessors don't each repeat the backing-type match.
    fn fields(&self) -> (&'m str, &'m str, Option<&'m str>, &'m m::DocumentTypeRef) {
        let model = self.cx.model;
        match self.which {
            DocImpl::Document(i) => {
                let d = model.document_arena.get(i.0);
                (&d.id, &d.name, d.description.as_deref(), &d.kind)
            }
            DocImpl::DocumentFile(i) => {
                let d = model.document_file_arena.get(i.0);
                (&d.id, &d.name, d.description.as_deref(), &d.kind)
            }
        }
    }

    /// This document's global identity (a `Copy` key for maps / dedup).
    pub fn key(&self) -> m::EntityKey {
        match self.which {
            DocImpl::Document(i) => m::EntityKey::Document(i),
            DocImpl::DocumentFile(i) => m::EntityKey::DocumentFile(i),
        }
    }

    /// The document identifier (`DOC-1`, a part number, …).
    pub fn id(&self) -> &'m str {
        self.fields().0
    }

    /// The document name / title.
    pub fn name(&self) -> &'m str {
        self.fields().1
    }

    /// The document description, if any.
    pub fn description(&self) -> Option<&'m str> {
        self.fields().2
    }

    /// The document kind (the `product_data_type` of its `DOCUMENT_TYPE`, e.g.
    /// `drawing`, `specification`).
    pub fn kind(&self) -> &'m str {
        let m::DocumentTypeRef::DocumentType(tid) = self.fields().3;
        &self
            .cx
            .model
            .document_type_arena
            .get(tid.0)
            .product_data_type
    }
}

/// A security classification assigned to a part — its confidentiality level
/// (`confidential`, `export-controlled`, …) and the reason for it.
#[derive(Clone, Copy)]
pub struct SecurityClassification<'m> {
    cx: Ctx<'m>,
    id: m::SecurityClassificationId,
    scope: Scope,
}

impl<'m> SecurityClassification<'m> {
    fn raw(&self) -> &'m m::SecurityClassification {
        self.cx.model.security_classification_arena.get(self.id.0)
    }

    /// This classification's global identity (a `Copy` key for maps / dedup).
    pub fn key(&self) -> m::EntityKey {
        m::EntityKey::SecurityClassification(self.id)
    }

    /// Which level of the product structure this classification attaches to.
    pub fn assigned_scope(&self) -> Scope {
        self.scope
    }

    /// The exact product-structure entities this classification is assigned to
    /// (across every assignment that carries it).
    pub fn assigned_targets(&self) -> Vec<Target<'m>> {
        let cx = self.cx;
        let mut keys: Vec<m::EntityKey> = Vec::new();
        for r in cx.ref_graph().referrers(self.key()) {
            match r {
                m::EntityKey::CcDesignSecurityClassification(i) => keys.extend(
                    cx.model
                        .cc_design_security_classification_arena
                        .get(i.0)
                        .items
                        .iter()
                        .map(m::CcClassifiedItemRef::entity_key),
                ),
                m::EntityKey::AppliedSecurityClassificationAssignment(i) => keys.extend(
                    cx.model
                        .applied_security_classification_assignment_arena
                        .get(i.0)
                        .items
                        .iter()
                        .map(m::SecurityClassificationItemRef::entity_key),
                ),
                _ => {}
            }
        }
        targets_from_keys(cx, keys.into_iter())
    }

    /// The classification name / identifier.
    pub fn name(&self) -> &'m str {
        &self.raw().name
    }

    /// Why the part is classified.
    pub fn purpose(&self) -> &'m str {
        &self.raw().purpose
    }

    /// The confidentiality level (the `name` of its `SECURITY_CLASSIFICATION_LEVEL`,
    /// e.g. `confidential`, `unclassified`).
    pub fn level(&self) -> &'m str {
        let m::SecurityClassificationLevelRef::SecurityClassificationLevel(lid) =
            &self.raw().security_level;
        &self
            .cx
            .model
            .security_classification_level_arena
            .get(lid.0)
            .name
    }
}
