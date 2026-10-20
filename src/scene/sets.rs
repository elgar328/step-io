//! Geometric sets: [`GeometricSet`], a `GEOMETRIC_SET` or `GEOMETRIC_CURVE_SET`
//! — curves and points kept together outside any b-rep, as a wireframe or a
//! sketch.

// Every public method here is a pure read accessor; `#[must_use]` on each would be
// pure noise, so allow the pedantic lint module-wide.
#![allow(clippy::must_use_candidate)]

use crate::generated::model as m;
use crate::scene::geometry::{Curve, Point};
use crate::scene::{Ctx, Scene};

impl Scene<'_> {
    /// Every geometric set in the model (`GEOMETRIC_SET` and
    /// `GEOMETRIC_CURVE_SET`).
    pub fn all_geometric_sets(&self) -> impl Iterator<Item = GeometricSet<'_>> + '_ {
        let cx = self.ctx();
        let sets = cx
            .model
            .geometric_set_arena
            .indices()
            .map(move |i| GeometricSet {
                cx,
                which: SetImpl::Set(m::GeometricSetId(i)),
            });
        let curve_sets = cx
            .model
            .geometric_curve_set_arena
            .indices()
            .map(move |i| GeometricSet {
                cx,
                which: SetImpl::Curves(m::GeometricCurveSetId(i)),
            });
        sets.chain(curve_sets)
    }
}

/// A set of geometry kept outside any b-rep (`GEOMETRIC_SET`, or its subtype
/// `GEOMETRIC_CURVE_SET` of curves and points): a wireframe, a sketch, a cloud
/// of points.
#[derive(Clone, Copy)]
pub struct GeometricSet<'m> {
    cx: Ctx<'m>,
    which: SetImpl,
}

#[derive(Clone, Copy)]
enum SetImpl {
    Set(m::GeometricSetId),
    Curves(m::GeometricCurveSetId),
}

/// One element of a [`GeometricSet`]. Curves and points are typed; anything
/// else (a placement, a planar box, a surface, a complex instance) is `Other`,
/// by key.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum SetElement<'m> {
    Curve(Curve<'m>),
    /// A `CARTESIAN_POINT`.
    Point(Point<'m>),
    Other(m::EntityKey),
}

impl<'m> GeometricSet<'m> {
    /// A set from a representation item, if it is one.
    pub(crate) fn from_item(cx: Ctx<'m>, it: &m::RepresentationItemRef) -> Option<Self> {
        let which = match it {
            m::RepresentationItemRef::GeometricSet(i) => SetImpl::Set(*i),
            m::RepresentationItemRef::GeometricCurveSet(i) => SetImpl::Curves(*i),
            _ => return None,
        };
        Some(GeometricSet { cx, which })
    }

    /// A set reached as a `GEOMETRIC_ITEM_SPECIFIC_USAGE` identified item (used
    /// by `pmi` feature geometry); `None` for an item that is not a set.
    pub(crate) fn from_model_item(cx: Ctx<'m>, r: &m::GeometricModelItemRef) -> Option<Self> {
        let which = match r {
            m::GeometricModelItemRef::GeometricSet(i) => SetImpl::Set(*i),
            m::GeometricModelItemRef::GeometricCurveSet(i) => SetImpl::Curves(*i),
            _ => return None,
        };
        Some(GeometricSet { cx, which })
    }

    /// This set's global identity (a `Copy` key for maps / deduplication).
    pub fn key(&self) -> m::EntityKey {
        match self.which {
            SetImpl::Set(i) => m::EntityKey::GeometricSet(i),
            SetImpl::Curves(i) => m::EntityKey::GeometricCurveSet(i),
        }
    }

    /// (`name`, `elements`) — the two set types share this layout.
    fn parts(&self) -> (&'m str, &'m [m::GeometricSetSelectRef]) {
        let model = self.cx.model;
        match self.which {
            SetImpl::Set(i) => {
                let s = model.geometric_set_arena.get(i.0);
                (model.text(s.name), &s.elements)
            }
            SetImpl::Curves(i) => {
                let s = model.geometric_curve_set_arena.get(i.0);
                (model.text(s.name), &s.elements)
            }
        }
    }

    pub fn name(&self) -> &'m str {
        self.parts().0
    }

    /// The elements of this set, in order.
    pub fn elements(&self) -> impl Iterator<Item = SetElement<'m>> + 'm {
        let cx = self.cx;
        self.parts().1.iter().map(move |e| {
            if let Some(curve) = Curve::from_set_select(cx, e) {
                return SetElement::Curve(curve);
            }
            match e {
                m::GeometricSetSelectRef::CartesianPoint(i) => {
                    SetElement::Point(Point::from_id(cx, *i))
                }
                other => SetElement::Other(other.entity_key()),
            }
        })
    }

    /// The colour styled onto this set: its curve style's colour, else its
    /// point style's marker colour. The elements may carry styles of their own.
    pub fn color(&self) -> Option<crate::scene::presentation::Rgb> {
        let this = self.about();
        crate::scene::presentation::curve_colour_of(this.cx, this.key())
            .or_else(|| crate::scene::presentation::point_colour_of(this.cx, this.key()))
    }

    /// The name of the presentation layer this set belongs to, if any.
    pub fn layer(&self) -> Option<&'m str> {
        crate::scene::presentation::layer_of(self.cx, self.key())
    }

    /// Whether this set is visible — `false` if an `INVISIBILITY` hides it (via
    /// its styled item or its layer). Defaults to visible.
    pub fn is_visible(&self) -> bool {
        crate::scene::presentation::is_visible(self.cx, self.key())
    }
}

about_by_key!(GeometricSet);
