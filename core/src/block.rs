//! A representation of a [`Block`] of ringing; i.e. a sort of 'multi-permutation' which takes a
//! starting [`Row`] and yields a sequence of permuted [`Row`]s.

use crate::{Row, Stage};
use wasm_bindgen::prelude::*;

/// A `Block` is an ordered sequence of [`Row`]s
///
/// A few things to note about `Block`s:
/// - All `Block`s must have non-zero length.  Zero-length blocks cannot be created with `safe`
///   code, and will cause undefined behaviour, usually `panic!`ing.
/// - A [`Row`] can be viewed as a special case of a [`Block`] of length `1`.
#[wasm_bindgen]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Block {
    /// The [`Row`]s making up this `Block`.
    ///
    /// A few important implementation details to note:
    /// 1. All [`Row`]s in `Block::rows` are permuting _the starting [`Row`]_, not each other.
    /// 2. There is an implicit [rounds](Row::rounds) at the start of every `Block`, but this
    ///    is not stored in `Block::rows`.
    /// 3. The last [`Row`] in `Block::rows` is 'left-over' - i.e. it shouldn't be used for truth
    ///    checking, and is used to generate the starting [`Row`] for the next `Block` that would
    ///    be rung after this one.
    ///
    /// We also enforce the following invariants:
    /// 1. `Block::rows` contains at least one [`Row`].  Zero-length `Block`s cannot be created
    ///    using `safe` code.
    /// 2. All the [`Row`]s in `Block::rows` must have the same [`Stage`].
    rows: Vec<Row>,
}

impl Block {
    /// Gets the [`Stage`] of this `Block`.
    #[inline]
    pub fn stage(&self) -> Stage {
        self.rows[0].stage()
    }

    /// Gets the length of this `Block`.  This is the number of [`Row`]s that would be generated
    /// when this `Block` is used to permute an arbitrary [`Row`].  This is guarunteed to be at
    /// least 1.
    #[inline]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Takes a [`Row`] of the same [`Stage`] as this `Block` and returns an [`Iterator`] that
    /// generates the sequence of [`Row`]s that make up this `Block` starting at that [`Row`].
    #[inline]
    pub fn row_iter<'b, 'r>(&'b self) -> RowIter<'b> {
        RowIter::from_block(self)
    }

    /// Returns the 'left-over' [`Row`] of this `Block`.  This [`Row`] represents the overall
    /// effect of the `Block`, and should not be used when generating rows for truth checking.
    #[inline]
    pub fn leftover_row(&self) -> &Row {
        // We can safely unwrap here, because we enforce an invariant that `self.rows.len() > 0`
        self.rows.last().unwrap()
    }
}

/// A small enum to track the state of [`RowIter`].
#[derive(Debug, Clone)]
enum IterState {
    /// The [`RowIter`] hasn't returned anything yet, so should just return the original slice
    Identity,
    /// The [`RowIter`] is actually reading from the [`Block`]
    RowFromBlock,
}

/// An [`Iterator`] that takes a [`Block`] and a slice with the same length as the [`Block`]'s
/// [`Stage`], and generates the sequence of permutations of that slice that correspond to the
/// [`Block`].  The elements of the slices will be [`Clone`]s of the original items.
#[derive(Debug, Clone)]
pub struct RowIter<'block> {
    block_iter: std::slice::Iter<'block, Row>,
    stage: Stage,
    state: IterState,
}

impl<'block> RowIter<'block> {
    /// Creates a new `RowIter` given a [`Block`] and a slice of items which implement [`Clone`]
    fn from_block(block: &'block Block) -> Self {
        RowIter {
            // We can unwrap here, because `block.rows.len() > 0`
            block_iter: block.rows.split_last().unwrap().1.iter(),
            stage: block.stage(),
            state: IterState::Identity,
        }
    }
}

impl<'block> Iterator for RowIter<'block> {
    type Item = Row;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            IterState::Identity => {
                self.state = IterState::RowFromBlock;
                Some(Row::rounds(self.stage))
            }
            // Invariant 2. of [`Block`] means that `perm_iter` must return a series of [`Row`]s
            // of the same stage.  The constructor [`RowIter::from_block`] also guaruntees that
            // the [`Block`]'s stage is compatible with the length of the slice.
            // => The unwrap is safe
            IterState::RowFromBlock => Some(self.block_iter.next()?.clone()),
        }
    }
}
