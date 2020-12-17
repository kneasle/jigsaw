//! A representation of a [`Block`] of ringing; i.e. a sort of 'multi-permutation' which takes a
//! starting [`Row`] and yields a sequence of permuted [`Row`]s.

use crate::{Bell, IncompatibleStages, Perm, Row, Stage};

/// A `Block` is a generalisation of [`Perm`], where instead of taking a [`Row`] and mapping that
/// to a single [`Row`], we map that [`Row`] to **multiple** [`Row`]s.
///
/// A few things to note about `Block`s:
/// - All `Block`s must have non-zero length.  Zero-length blocks cannot be created with `safe`
///   code, and will cause undefined behaviour, usually `panic!`s.
/// - A [`Perm`] is just a special case of a [`Block`] of length `1`.
/// - Like [`Perm`], a `Block` can be used to permute slices of any [`Clone`] type, provided that
///   the length of the input slice the same as the [`Stage`] of the `Block`.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Block {
    /// The [`Perm`]s making up this `Block`.
    ///
    /// A few important implementation details to note:
    /// 1. All [`Perm`]s in `Block::perms` are permuting _the starting [`Row`]_, not each other.
    /// 2. There is an implicit [identity `Perm`](Perm::id) at the start of every `Block`, but this
    ///    is not stored in `Block::perms`.
    /// 3. The last [`Perm`] in `Block::perms` is 'left-over' - i.e. it shouldn't be used for truth
    ///    checking, and is used to generate the starting [`Row`] for the next `Block` that would
    ///    be rung after this one.
    ///
    /// We also enforce the following invariants:
    /// 1. `Block::perms` contains at least one [`Perm`].  Zero-length `Block`s cannot be created
    ///    using `safe` code.
    /// 2. All the [`Perm`]s in `Block::perms` must have the same [`Stage`].
    ///
    /// As an example, let's take the `Block` representing one lead of
    /// [Bastow Little Bob Minor](https://rsw.me.uk/blueline/methods/view/Bastow_Little_Bob_Minor).
    /// In order to be as unambiguous as possible, I'm going to be permuting `abcdef`.  The rows we
    /// would want to truth check are
    /// ```text
    /// abcdef
    /// badcfe
    /// bacdef
    /// abdcfe
    /// ```
    /// and the 'left-over' [`Perm`] should be `adbfce`.  However, the `abcdef` at the start is an
    /// arbitrary choice of the input list, so we don't include it in the representation (following
    /// point 2.), and instead we store the 'left-over' [`Perm`] on the end of the [`Vec`]
    /// (following point 3.).  Therefore, this `Block` would be stored in memory as the following
    /// slice (note that [`Perm`]s are 0-indexed):
    /// ```ignore
    /// [
    ///     Perm::from(&[1, 0, 3, 2, 5, 4]),
    ///     Perm::from(&[1, 0, 2, 3, 4, 5]),
    ///     Perm::from(&[0, 1, 3, 2, 5, 4]),
    ///     Perm::from(&[0, 3, 1, 5, 2, 4]),
    /// ]
    /// ```
    perms: Vec<Perm>,
}

impl Block {
    /// Gets the [`Stage`] of this `Block`.
    #[inline]
    pub fn stage(&self) -> Stage {
        self.perms[0].stage()
    }

    /// Gets the length of this `Block`.  This is the number of [`Row`]s that would be generated
    /// when this `Block` is used to permute an arbitrary [`Row`].  This is guarunteed to be at
    /// least 1.
    #[inline]
    pub fn len(&self) -> usize {
        self.perms.len()
    }

    /// Returns an [`Iterator`] which returns permutions of the contents of a given `slice`.
    #[inline]
    pub fn perm_iter<'b, 's, T>(
        &'b self,
        slice: &'s [T],
    ) -> Result<PermIter<'b, 's, T>, IncompatibleStages>
    where
        T: Clone,
    {
        PermIter::from_block(self, slice)
    }

    /// Takes a [`Row`] of the same [`Stage`] as this `Block` and returns an [`Iterator`] that
    /// generates the sequence of [`Row`]s that make up this `Block` starting at that [`Row`].
    #[inline]
    pub fn row_iter<'b, 'r>(&'b self, row: &'r Row) -> Result<RowIter<'b, 'r>, IncompatibleStages> {
        RowIter::from_block(self, row)
    }

    /// Returns the 'left-over' [`Perm`] of this `Block`.  This [`Perm`] represents the overall
    /// effect of the `Block`, and should not be used when generating rows for truth checking.
    #[inline]
    pub fn leftover_perm(&self) -> &Perm {
        // We can safely unwrap here, because we enforce an invariant that `self.perms.len() > 0`
        self.perms.last().unwrap()
    }
}

/// A small enum to track the state of [`PermIter`].
#[derive(Debug, Clone)]
enum IterState {
    /// The [`PermIter`] hasn't returned anything yet, so should just return the original slice
    Identity,
    /// The [`PermIter`] is actually reading from the [`Block`]
    PermFromBlock,
}

/// An [`Iterator`] that takes a [`Block`] and a slice with the same length as the [`Block`]'s
/// [`Stage`], and generates the sequence of permutations of that slice that correspond to the
/// [`Block`].  The elements of the slices will be [`Clone`]s of the original items.
#[derive(Debug, Clone)]
pub struct PermIter<'block, 'slice, T> {
    block_iter: std::slice::Iter<'block, Perm>,
    state: IterState,
    slice: &'slice [T],
}

impl<'block, 'slice, T> PermIter<'block, 'slice, T> {
    /// Creates a new `PermIter` given a [`Block`] and a slice of items which implement [`Clone`]
    fn from_block(block: &'block Block, slice: &'slice [T]) -> Result<Self, IncompatibleStages>
    where
        T: Clone,
    {
        if block.stage().as_usize() != slice.len() {
            return Err(IncompatibleStages::new(slice.len(), block.stage()));
        }
        Ok(PermIter {
            // We can unwrap here, because `block.perms.len() > 0`
            block_iter: block.perms.split_last().unwrap().1.iter(),
            state: IterState::Identity,
            slice,
        })
    }
}

impl<'block, 'slice, T> Iterator for PermIter<'block, 'slice, T>
where
    T: Clone,
{
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            IterState::Identity => {
                self.state = IterState::PermFromBlock;
                Some(self.slice.iter().cloned().collect())
            }
            // Invariant 2. of [`Block`] means that `perm_iter` must return a series of [`Perm`]s
            // of the same stage.  The constructor [`PermIter::from_block`] also guaruntees that
            // the [`Block`]'s stage is compatible with the length of the slice.
            // => The unwrap is safe
            IterState::PermFromBlock => Some(self.block_iter.next()?.permute(self.slice).unwrap()),
        }
    }
}

/// An [`Iterator`] that takes a [`Block`] and a [`Row`] with the same [`Stage`] and generates the
/// sequence of [`Row`]s that make up the [`Block`] starting at that [`Row`].
#[derive(Debug, Clone)]
pub struct RowIter<'block, 'row> {
    perm_iter: PermIter<'block, 'row, Bell>,
}

impl<'block, 'row> RowIter<'block, 'row> {
    /// Creates a new `RowIter` given a [`Block`] and a [`Row`].
    fn from_block(block: &'block Block, start_row: &'row Row) -> Result<Self, IncompatibleStages> {
        Ok(RowIter {
            perm_iter: PermIter::from_block(block, start_row.slice())?,
        })
    }
}

impl<'block, 'row> Iterator for RowIter<'block, 'row> {
    type Item = Row;

    fn next(&mut self) -> Option<Row> {
        // We can use `from_vec_unchecked`, because the permuted `Row` must be valid if the
        // `Perm` and input `Row` both satisfy their invariants
        self.perm_iter.next().map(Row::from_vec_unchecked)
    }
}
