//! A representation of a [`Block`] of ringing; i.e. a sort of 'multi-permutation' which takes a
//! starting [`Row`] and yields a sequence of permuted [`Row`]s.

use crate::{IncompatibleStages, InvalidRowError, Row, Stage};

/// All the possible ways that parsing a [`Block`] could fail
#[derive(Debug, Clone)]
pub enum ParseError {
    ZeroLengthBlock,
    InvalidRow {
        line: usize,
        err: InvalidRowError,
    },
    IncompatibleStages {
        line: usize,
        first_stage: Stage,
        different_stage: Stage,
    },
}

/// A `Block` is an ordered sequence of [`Row`]s, which are usually meant to be rung contiguously.
///
/// A few things to note about `Block`s:
/// - All `Block`s must have non-zero length.  Zero-length blocks cannot be created with `safe`
///   code, and will cause undefined behaviour, usually `panic!`ing.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Block {
    /// The [`Row`]s making up this `Block`.
    ///
    /// A few important implementation details to note:
    /// 1. The last [`Row`] in `Block::rows` is 'left-over' - i.e. it shouldn't be used for truth
    ///    checking, and is used to generate the starting [`Row`] for the next `Block` that would
    ///    be rung after this one.
    ///
    /// We also enforce the following invariants:
    /// 1. `Block::rows` contains at least two [`Row`]s.  Zero-length `Block`s cannot be created
    ///    using `safe` code.
    /// 2. All the [`Row`]s in `Block::rows` must have the same [`Stage`].
    rows: Vec<Row>,
}

// We don't need `is_empty`, because the length is guaruteed to be at least 1
#[allow(clippy::len_without_is_empty)]
impl Block {
    /// Parse a multi-line [`str`]ing into a `Block`.  The last [`Row`] parsed will be considered
    /// 'left over' - i.e. it isn't directly part of this `Block` but rather will be the first
    /// [`Row`] of any `Block` which gets appended to this one.
    pub fn parse(s: &str) -> Result<Block, ParseError> {
        // We store the _inverse_ of the first Row, because for each row R we are solving the
        // equation `FX = R` where F is the first Row.  The solution to this is `X = F^-1 * R`, so
        // it makes sense to invert F once and then use that in all subsequent calculations.
        let mut rows = Vec::new();
        for (i, l) in s.lines().enumerate() {
            // Parse the line into a Row, and fail if its either invalid or doesn't match the stage
            let parsed_row =
                Row::parse(l).map_err(|err| ParseError::InvalidRow { line: i, err })?;
            if let Some(first_stage) = rows.first().map(Row::stage) {
                if first_stage != parsed_row.stage() {
                    return Err(ParseError::IncompatibleStages {
                        line: i,
                        first_stage,
                        different_stage: parsed_row.stage(),
                    });
                }
            }
            // If all the checks passed, push the row
            rows.push(parsed_row);
        }
        // Return an error if the rows would form a zero-length block
        if rows.len() <= 1 {
            return Err(ParseError::ZeroLengthBlock);
        }
        // Create a block from the newly parsed [`Row`]s.  This unsafety is OK, because we have
        // verified all the invariants
        Ok(unsafe { Self::from_rows_unchecked(rows) })
    }

    /// Creates a new `Block` from a [`Vec`] of [`Row`]s, without performing any safety checks.
    ///
    /// # Safety
    ///
    /// This is safe when the following properties hold:
    /// - `rows` has length at least 2.  This is so that there is at least one [`Row`] in the
    ///   block, plus one leftover [`Row`].
    /// - All the `rows` have the same [`Stage`].
    pub unsafe fn from_rows_unchecked(rows: Vec<Row>) -> Self {
        Block { rows }
    }

    /// Gets the [`Stage`] of this `Block`.
    #[inline]
    pub fn stage(&self) -> Stage {
        self.rows[0].stage()
    }

    /// Gets the length of this `Block` (excluding the left-over [`Row`]).  This is guarunteed to
    /// be at least 1.
    #[inline]
    pub fn len(&self) -> usize {
        self.rows.len() - 1
    }

    /// Returns an [`Iterator`] over all the non-left-over [`Row`]s in this `Block`
    #[inline]
    pub fn rows(&self) -> std::slice::Iter<'_, Row> {
        self.rows.iter()
    }

    /// Pre-multiplies every [`Row`] in this `Block` by another [`Row`].  The resulting `Block` is
    /// equivalent to `self` (inasmuch as the relations between the [`Row`]s are identical), but it
    /// will start from a different [`Row`].
    pub fn pre_mul(&mut self, perm_row: &Row) -> Result<(), IncompatibleStages> {
        IncompatibleStages::test_err(perm_row.stage(), self.stage())?;
        let mut row_buf = Row::empty();
        self.rows.iter_mut().for_each(|r| {
            // Do in-place pre-multiplication using `row_buf` as a temporary buffer
            row_buf.clone_from(r);
            *r = unsafe { perm_row.mul_unchecked(&row_buf) };
        });
        Ok(())
    }

    /// Returns the 'left-over' [`Row`] of this `Block`.  This [`Row`] represents the overall
    /// effect of the `Block`, and should not be used when generating rows for truth checking.
    #[inline]
    pub fn leftover_row(&self) -> &Row {
        // We can safely unwrap here, because we enforce an invariant that `self.rows.len() > 0`
        self.rows.last().unwrap()
    }
}
