//! The fully annotated state of a composition used for querying and rendering.

use bellframe::SameStageVec;

use crate::V2;

mod expand;

/// The fully specified state of a composition.  This is designed to be efficient to query and easy
/// to render from, unlike [`CompSpec`] which is designed to be compact and easy to modify or store
/// to disk.
///
/// There will only be one copy of [`FullComp`] instantiated at a time, and it is up to the
/// [`Comp`] instance to make sure that it always represents the data that the user expects to see.
/// Every time the [`CompSpec`] being viewed changes (either through the user's changes or through
/// undo/redo), the [`FullComp`] is recomputed for the new [`CompSpec`].
#[derive(Debug, Clone)]
pub struct FullComp {
    fragments: Vec<Fragment>,
}

#[derive(Debug, Clone)]
struct Fragment {
    position: V2,
    /// The index of the link group which the top of this `Fragment` is connected to
    link_group_top: Option<usize>,
    /// The index of the link group which the bottom of this `Fragment` is connected to
    link_group_bottom: Option<usize>,
    /// The `ExpandedRow`s from this `Fragment`.  Each of these contains one [`Row`] per part.
    rows: Vec<ExpandedRow>,
}

/// A single place where a [`Row`] can be displayed on the screen.  This corresponds to multiple
/// [`Row`]s (one per part) but these are connected inasmuch as they can only be added or removed
/// together.
#[derive(Debug, Clone)]
struct ExpandedRow {
    /// This `ExpandedRow` expands to one [`Row`] per part.
    rows: SameStageVec,
    /// If `true` then this [`Row`] is considered 'part' of the composition.
    is_proved: bool,
    /// Do any of these [`Row`]s appear elsewhere in the composition?
    is_false: bool,
}
