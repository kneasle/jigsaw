//! The fully annotated state of a composition used for querying and rendering.

use std::{ops::Deref, rc::Rc};

use bellframe::{SameStageVec, Stage};
use emath::Pos2;

use itertools::Itertools;
use jigsaw_utils::types::{
    FragVec, MethodVec, PartIdx, PartVec, RowIdx, RowLocation, RowSource, RowVec,
};

use crate::{
    music,
    spec::{self, part_heads::PartHeads, CompSpec},
};

// Imports only used for doc comments
#[allow(unused_imports)]
use bellframe::Row;

mod from_expanded_frags; // Code to build a [`FullState`] from [`ExpandedFrag`]s and other data

/// The fully specified state of a composition.  This is designed to be efficient to query and easy
/// to render from, unlike [`CompSpec`] which is designed to be compact and easy to modify or store
/// to disk.
///
/// There will only be one copy of [`FullState`] instantiated at a time, and it is up to the
/// [`State`](super::State) instance to make sure that it always represents the data that the user
/// expects to see.  Every time the [`CompSpec`] being viewed changes (either through the user's
/// changes or through undo/redo), the [`FullState`] is recomputed for the new [`CompSpec`].
#[derive(Debug)]
pub struct FullState {
    pub part_heads: Rc<PartHeads>,
    pub fragments: FragVec<Fragment>,
    pub methods: MethodVec<Rc<Method>>,
    pub music: Music,
    /// Misc statistics about the composition (e.g. part length)
    pub stats: Stats,
    pub stage: Stage,
}

impl FullState {
    /// Creates a new [`FullState`] representing the same composition as a given [`CompSpec`].
    pub fn new(spec: &CompSpec, music: &[music::Music]) -> Self {
        let expanded_frags = spec.expand_fragments();
        from_expanded_frags::from_expanded_frags(
            expanded_frags,
            &spec.methods(),
            spec.part_heads().clone(),
            music,
            spec.stage(),
        )
    }

    /// Updates `self` to represent the same composition as a given [`CompSpec`]
    pub fn update(&mut self, spec: &CompSpec, music: &[music::Music]) {
        // For now, just overwrite `self` without reusing any allocations
        *self = Self::new(spec, music);
    }
}

///////////////
// FRAGMENTS //
///////////////

#[derive(Debug, Clone)]
pub struct Fragment {
    /// The position of the top-left corner of the first [`Row`] in this `Fragment`
    pub position: Pos2,
    /// For each part, which [`Row`]s make up this `Fragment`
    rows_per_part: PartVec<SameStageVec>,
    /// For each part, how many leaf music groups match each place in the [`Row`]s from that part.
    /// I find it extremely unlikely that we'll overflow `u8`s here (since we'd need at least 256
    /// music groups to apply to the same position in a row).  Even then, the code saturates
    /// instead of overflowing and prints a warning to stderr.
    music_highlights_per_part: PartVec<Vec<u8>>,
    /// Extra non-part-specific data about each row to help the rendering
    row_data: RowVec<RowData>,
}

impl Fragment {
    pub fn num_rows(&self) -> usize {
        self.row_data.len()
    }

    pub fn rows_in_part(&self, part: PartIdx) -> impl Iterator<Item = (RowIdx, FullRowData)> {
        let row_vec = &self.rows_per_part[part];
        let stage = row_vec.stage();
        row_vec
            .iter()
            .zip_eq(&self.row_data)
            .zip_eq(self.music_highlights_per_part[part].chunks(stage.num_bells()))
            .enumerate()
            .map(|(idx, ((row, data), music_counts))| {
                (RowIdx::new(idx), FullRowData::new(row, music_counts, data))
            })
    }
}

/// All the data required to render a row to the screen
#[derive(Debug, Clone)]
pub struct FullRowData<'frag> {
    pub row: &'frag Row,
    pub music_counts: &'frag [u8],
    data: &'frag RowData,
}

impl<'frag> FullRowData<'frag> {
    pub fn new(row: &'frag Row, music_counts: &'frag [u8], data: &'frag RowData) -> Self {
        Self {
            row,
            music_counts,
            data,
        }
    }
}

impl<'frag> Deref for FullRowData<'frag> {
    type Target = &'frag RowData;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

/// A single place where a [`Row`] can be displayed on the screen.  This corresponds to multiple
/// [`Row`]s (one per part) but these are connected inasmuch as they can only be added or removed
/// together.
#[derive(Debug, Clone)]
pub struct RowData {
    /// If `true` then this [`Row`] is considered 'part' of the composition.
    pub is_proved: bool,
    /// If `true` then this [`Row`] should have a line drawn **above** it
    pub ruleoff_above: bool,
    /// What method name should be placed here
    pub method_annotation: Option<Rc<Method>>,
    /*
    /// Do any of these [`Row`]s appear elsewhere in the composition?
    pub is_false: bool,
    */
}

/////////////
// METHODS //
/////////////

#[derive(Debug, Clone)]
pub struct Method {
    pub(crate) source: Rc<spec::Method>,
    /// Total number of [`Row`]s assigned to this [`Method`]
    pub num_rows: usize,
    /// Number of proved [`Row`]s assigned to this [`Method`]
    pub num_proved_rows: usize,
}

impl Method {
    #[inline]
    pub fn name(&self) -> String {
        self.source.name().to_owned()
    }

    #[inline]
    pub fn shorthand(&self) -> String {
        self.source.shorthand().to_owned()
    }
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
pub struct Stats {
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
