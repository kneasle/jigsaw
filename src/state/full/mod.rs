//! The fully annotated state of a composition used for querying and rendering.

use std::{ops::Deref, rc::Rc};

use bellframe::{SameStageVec, Stage};
use eframe::egui::Vec2;

use crate::utils::{RowLocation, RowSource};

use super::{
    music,
    spec::{self, part_heads::PartHeads, CompSpec},
};

/// The fully specified state of a composition.  This is designed to be efficient to query and easy
/// to render from, unlike [`CompSpec`] which is designed to be compact and easy to modify or store
/// to disk.
///
/// There will only be one copy of [`FullComp`] instantiated at a time, and it is up to the
/// [`Comp`] instance to make sure that it always represents the data that the user expects to see.
/// Every time the [`CompSpec`] being viewed changes (either through the user's changes or through
/// undo/redo), the [`FullComp`] is recomputed for the new [`CompSpec`].
#[derive(Debug, Clone)]
pub(crate) struct FullState {
    pub part_heads: Rc<PartHeads>,
    pub fragments: Vec<Fragment>,
    pub methods: Vec<Method>,
    pub music: Music,
    /// Misc statistics about the composition (e.g. part length)
    pub stats: Stats,
    pub stage: Stage,
}

impl FullState {
    /// Creates a new [`FullState`] representing the same composition as a given [`CompSpec`].
    pub fn new(spec: &CompSpec, music: &[music::Music]) -> Self {
        spec::expand(spec, music) // Delegate to the `expand` module
    }

    /// Updates `self` to represent the same composition as a given [`CompSpec`]
    pub fn update(&mut self, spec: &CompSpec, music: &[music::Music]) {
        // Just overwrite `self`, without reusing any allocations
        *self = Self::new(spec, music);
    }
}

///////////////
// FRAGMENTS //
///////////////

#[derive(Debug, Clone)]
pub(crate) struct Fragment {
    // These fields need to be `pub(super)` so that they can be populated during expansion by
    // `super::spec::expand::expand(...)`
    /// The position of the top-left corner of the first [`Row`] in this `Fragment`
    pub position: Vec2,
    /// The index of the link group which the top of this `Fragment` is connected to
    pub link_group_top: Option<usize>,
    /// The index of the link group which the bottom of this `Fragment` is connected to
    pub link_group_bottom: Option<usize>,
    /// The `ExpandedRow`s from this `Fragment`.  Each of these contains one [`Row`] per part.
    pub expanded_rows: Vec<ExpandedRow>,
}

/////////////
// METHODS //
/////////////

#[derive(Debug, Clone)]
pub(crate) struct Method {
    pub(super) source: Rc<spec::Method>, // Accessed through `Deref` coercion
    /// Total number of [`Row`]s assigned to this [`Method`]
    pub num_rows: usize,
    /// Number of proved [`Row`]s assigned to this [`Method`]
    pub num_proved_rows: usize,
}

// Deref-coerce to `spec::Method`.  This will make `full::Method` appear to 'inherit' all the
// properties of the contained `spec::Method`
impl Deref for Method {
    type Target = spec::Method;

    fn deref(&self) -> &Self::Target {
        &self.source
    }
}

/////////////////////
// (EXPANDED) ROWS //
/////////////////////

/// A single place where a [`Row`] can be displayed on the screen.  This corresponds to multiple
/// [`Row`]s (one per part) but these are connected inasmuch as they can only be added or removed
/// together.
#[derive(Debug, Clone)]
pub(crate) struct ExpandedRow {
    /// This `ExpandedRow` expands to one [`Row`] per part.
    pub rows: SameStageVec,
    /// If `true` then this [`Row`] is considered 'part' of the composition.
    pub is_proved: bool,
    /// Do any of these [`Row`]s appear elsewhere in the composition?
    pub is_false: bool,
    /// For each part, for each place, how many leaf music groups match at this location
    pub music_highlights: Vec<Vec<usize>>,
}

///////////
// MUSIC //
///////////

/// Top-level representation of music
#[derive(Debug, Clone)]
pub struct Music {
    pub(super) groups: Vec<Rc<MusicGroup>>,
    pub(super) total_count: usize,
    pub(super) max_count: usize,
}

impl Music {
    pub fn groups(&self) -> &[Rc<MusicGroup>] {
        self.groups.as_slice()
    }

    pub fn total_count(&self) -> usize {
        self.total_count
    }

    /// Get a reference to the music's max count.
    pub fn max_count(&self) -> &usize {
        &self.max_count
    }
}

/// A group of musical rows, potentially subdivided into more groups.  This strongly follows the
/// shape of [`super::music::Music`].
#[derive(Debug, Clone)]
pub struct MusicGroup {
    pub name: String,
    pub max_count: usize,
    // If empty, then this [`MusicGroup`] is a 'leaf' of the tree
    pub inner: MusicGroupInner,
}

impl MusicGroup {
    /// Add the [`RowSource`] of every [`Row`] matched by `self` or any of its descendants.
    /// [`RowSource`]s may be added multiple times.
    pub fn add_row_sources(&self, out: &mut impl Extend<RowSource>) {
        match &self.inner {
            MusicGroupInner::Leaf { rows_matched } => {
                out.extend(rows_matched.iter().map(|loc| loc.as_source()))
            }
            MusicGroupInner::Group { sub_groups, .. } => {
                for g in sub_groups {
                    g.add_row_sources(out);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum MusicGroupInner {
    Leaf {
        rows_matched: Vec<RowLocation>,
    },
    Group {
        sub_groups: Vec<Rc<MusicGroup>>,
        count: usize,
    },
}

impl MusicGroupInner {
    /// Returns the number of times that this [`MusicGroup`] was matched in the composition
    pub fn count(&self) -> usize {
        match self {
            MusicGroupInner::Leaf { rows_matched } => rows_matched.len(),
            MusicGroupInner::Group { count, .. } => *count,
        }
    }
}

/////////////////////
// MISC STATISTICS //
/////////////////////

#[derive(Debug, Clone)]
pub(crate) struct Stats {
    /// The number of [`Row`]s in each part of the composition
    pub part_len: usize,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            part_len: Default::default(),
        }
    }
}
