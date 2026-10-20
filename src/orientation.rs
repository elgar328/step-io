//! Which way a face, a boundary loop, an edge in a loop, or a shell in a
//! solid is used.

/// Which way a face, a face's boundary loop, an edge in a loop, or a shell in
/// a solid is used relative to the geometry or element it is built on. STEP
/// stores each of these as a boolean; reading ([`Face::orientation`],
/// [`Bound::orientation`], [`Bound::oriented_edges`], [`Solid::outer`],
/// [`Solid::voids`]) and authoring ([`StepBuilder::face`],
/// [`FaceBoundInput`]) both use this type, so a value read back can be passed
/// straight to the builder.
///
/// [`Face::orientation`]: crate::scene::Face::orientation
/// [`Bound::orientation`]: crate::scene::Bound::orientation
/// [`Bound::oriented_edges`]: crate::scene::Bound::oriented_edges
/// [`Solid::outer`]: crate::scene::Solid::outer
/// [`Solid::voids`]: crate::scene::Solid::voids
/// [`StepBuilder::face`]: crate::StepBuilder::face
/// [`FaceBoundInput`]: crate::build::FaceBoundInput
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// A face: its normal is its surface's normal. A boundary loop: the loop
    /// is used as it runs. An edge in a loop: the loop runs along the edge
    /// from its start vertex to its end vertex. A shell in a solid: its faces
    /// are used as they are.
    Forward,
    /// A face: its normal is against its surface's normal. A boundary loop:
    /// the loop is used reversed. An edge in a loop: the loop runs along the
    /// edge from its end vertex back to its start. A shell in a solid: every
    /// face is used turned over.
    Reversed,
}

impl Orientation {
    /// From the STEP boolean: `.T.` is [`Forward`](Self::Forward).
    pub(crate) fn from_flag(flag: bool) -> Self {
        if flag { Self::Forward } else { Self::Reversed }
    }

    /// The STEP boolean: `.T.` for [`Forward`](Self::Forward).
    pub(crate) fn flag(self) -> bool {
        self == Self::Forward
    }
}
