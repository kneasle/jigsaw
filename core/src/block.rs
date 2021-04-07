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

/// An `AnnotBlock` with no annotations.
pub type Block = AnnotBlock<()>;

impl Block {
    /// Creates a new unannotated `Block` from a [`Vec`] of [`Row`]s, without performing any safety
    /// checks.  This also performs a transmutation from `X` to `(X, ())`, which should be safe but
    /// if you prefer to avoid unsafety like this then you can use
    /// [`AnnotBlock::from_annot_rows_unchecked`].
    ///
    /// # Safety
    ///
    /// This is safe when the following properties hold:
    /// - `rows` has length at least 2.  This is so that there is at least one [`Row`] in the
    ///   block, plus one leftover [`Row`].
    /// - All the `rows` have the same [`Stage`].
    pub unsafe fn from_rows_unchecked(mut rows: Vec<Row>) -> Self {
        // This unsafety is OK, because we are not transmuting the `Vec` directly, and `Row` and
        // `(Row, ())` must share the same memory layout.
        let ptr = rows.as_mut_ptr() as *mut (Row, ());
        let len = rows.len();
        let cap = rows.capacity();
        std::mem::forget(rows);
        AnnotBlock {
            rows: Vec::from_raw_parts(ptr, len, cap),
        }
    }
}

/// An `AnnotBlock` is in essence a multi-permutation: it describes the transposition of a single
/// start [`Row`] into many [`Row`]s, the first of which is always the one supplied.  The last
/// [`Row`] of an `AnnotBlock` is considered 'left-over', and represents the first [`Row`] that
/// should be rung after this `AnnotBlock`.
///
/// A few things to note about `Block`s:
/// - All `Block`s must have non-zero length.  Zero-length blocks cannot be created with `safe`
///   code, and will cause undefined behaviour or `panic!`s.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AnnotBlock<A> {
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
    /// 3. The first [`Row`] should always equal `rounds`
    rows: Vec<(Row, A)>,
}

// We don't need `is_empty`, because the length is guaruteed to be at least 1
#[allow(clippy::len_without_is_empty)]
impl<A> AnnotBlock<A> {
    /// Parse a multi-line [`str`]ing into an unannotated `Block`.  The last [`Row`] parsed will be
    /// considered 'left over' - i.e. it isn't directly part of this `Block` but rather will be the
    /// first [`Row`] of any `Block` which gets appended to this one.  Each [`Row`] has the default
    /// annotation (thus requiring that `A` implements [`Default`]).
    pub fn parse(s: &str) -> Result<Self, ParseError>
    where
        A: Default,
    {
        // We store the _inverse_ of the first Row, because for each row R we are solving the
        // equation `FX = R` where F is the first Row.  The solution to this is `X = F^-1 * R`, so
        // it makes sense to invert F once and then use that in all subsequent calculations.
        let mut inv_first_row: Option<Row> = None;
        let mut annot_rows: Vec<(Row, A)> = Vec::new();
        for (i, l) in s.lines().enumerate() {
            // Parse the line into a Row, and fail if its either invalid or doesn't match the stage
            let parsed_row =
                Row::parse(l).map_err(|err| ParseError::InvalidRow { line: i, err })?;
            if let Some(inv_first_row) = &inv_first_row {
                if inv_first_row.stage() != parsed_row.stage() {
                    return Err(ParseError::IncompatibleStages {
                        line: i,
                        first_stage: inv_first_row.stage(),
                        different_stage: parsed_row.stage(),
                    });
                }
                // If all the checks passed, push the row
                annot_rows.push((
                    unsafe { inv_first_row.mul_unchecked(&parsed_row) },
                    A::default(),
                ));
            } else {
                // If this is the first Row, then push rounds and set the inverse first row
                inv_first_row = Some(!&parsed_row);
                annot_rows.push((Row::rounds(parsed_row.stage()), A::default()));
            }
        }
        // Return an error if the rows would form a zero-length block
        if annot_rows.len() <= 1 {
            return Err(ParseError::ZeroLengthBlock);
        }
        // Create a block from the newly parsed [`Row`]s.  This unsafety is OK, because we have
        // verified all the invariants
        Ok(unsafe { Self::from_annot_rows_unchecked(annot_rows) })
    }

    /// Creates a new `AnnotBlock` from a [`Vec`] of annotated [`Row`]s, checking that the result
    /// is valid.
    pub fn from_annot_rows(annot_rows: Vec<(Row, A)>) -> Result<Self, ParseError> {
        assert!(annot_rows[0].0.is_rounds());
        if annot_rows.len() <= 1 {
            return Err(ParseError::ZeroLengthBlock);
        }
        let first_stage = annot_rows[0].0.stage();
        for (i, (r, _annot)) in annot_rows.iter().enumerate().skip(1) {
            if r.stage() != first_stage {
                return Err(ParseError::IncompatibleStages {
                    line: i,
                    first_stage,
                    different_stage: r.stage(),
                });
            }
        }
        // This unsafety is OK because we've checked all the required invariants
        Ok(unsafe { Self::from_annot_rows_unchecked(annot_rows) })
    }

    /// Creates a new `AnnotBlock` from a [`Vec`] of annotated [`Row`]s, without performing any
    /// safety checks.
    ///
    /// # Safety
    ///
    /// This is safe when the following properties hold:
    /// - `rows` has length at least 2.  This is so that there is at least one [`Row`] in the
    ///   `AnnotBlock`, plus one leftover [`Row`].
    /// - All the `rows` have the same [`Stage`].
    pub unsafe fn from_annot_rows_unchecked(rows: Vec<(Row, A)>) -> Self {
        AnnotBlock { rows }
    }

    /// Gets the [`Stage`] of this `Block`.
    #[inline]
    pub fn stage(&self) -> Stage {
        self.rows[0].0.stage()
    }

    /// Gets the [`Row`] at a given index, along with its annotation.
    #[inline]
    pub fn get_row(&self, index: usize) -> Option<&Row> {
        self.get_annot_row(index).map(|(r, _annot)| r)
    }

    /// Gets an immutable reference to the annotation of the [`Row`] at a given index, if it
    /// exists.
    #[inline]
    pub fn get_annot(&self, index: usize) -> Option<&A> {
        self.get_annot_row(index).map(|(_row, annot)| annot)
    }

    /// Gets an mutable reference to the annotation of the [`Row`] at a given index, if it
    /// exists.
    #[inline]
    pub fn get_annot_mut(&mut self, index: usize) -> Option<&mut A> {
        self.rows.get_mut(index).map(|(_row, annot)| annot)
    }

    /// Gets the [`Row`] at a given index, along with its annotation.
    #[inline]
    pub fn get_annot_row(&self, index: usize) -> Option<&(Row, A)> {
        self.rows.get(index)
    }

    /// Gets the first [`Row`] of this `AnnotBlock`, along with its annotation.
    #[inline]
    pub fn first_annot_row(&self) -> &(Row, A) {
        // This can't panic, because of the invariant disallowing zero-sized `AnnotBlock`s
        &self.rows[0]
    }

    /// Gets the length of this `Block` (excluding the left-over [`Row`]).  This is guarunteed to
    /// be at least 1.
    #[inline]
    pub fn len(&self) -> usize {
        self.rows.len() - 1
    }

    /// Returns an [`Iterator`] over all the [`Row`]s in this `AnnotBlock`, along with their
    /// annotations.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, (Row, A)> {
        self.rows.iter()
    }

    /// Returns an immutable reference to the slice of annotated [`Row`]s making up this [`Block`]
    #[inline]
    pub fn annot_rows(&self) -> &[(Row, A)] {
        self.rows.as_slice()
    }

    /// Returns an [`Iterator`] over all the [`Row`]s in this `Block`, without their annotations.
    #[inline]
    pub fn rows(&self) -> impl Iterator<Item = &Row> + '_ {
        self.iter().map(|(r, _annot)| r)
    }

    /// Pre-multiplies every [`Row`] in this `Block` by another [`Row`].  The resulting `Block` is
    /// equivalent to `self` (inasmuch as the relations between the [`Row`]s are identical), but it
    /// will start from a different [`Row`].
    pub fn pre_mul(&mut self, perm_row: &Row) -> Result<(), IncompatibleStages> {
        IncompatibleStages::test_err(perm_row.stage(), self.stage())?;
        let mut row_buf = Row::empty();
        self.rows.iter_mut().for_each(|(r, _annot)| {
            // Do in-place pre-multiplication using `row_buf` as a temporary buffer
            row_buf.clone_from(r);
            *r = unsafe { perm_row.mul_unchecked(&row_buf) };
        });
        Ok(())
    }

    /// Returns the 'left-over' [`Row`] of this `Block`.  This [`Row`] represents the overall
    /// effect of the `Block`, and should not be used when generating rows for truth checking.
    #[inline]
    pub fn leftover_row(&self) -> &(Row, A) {
        // We can safely unwrap here, because we enforce an invariant that `self.rows.len() > 0`
        self.rows.last().unwrap()
    }

    /// Returns a mutable reference to the annotation of the 'left-over' [`Row`] of this `Block`.
    #[inline]
    pub fn leftover_annot_mut(&mut self) -> &mut A {
        // We can safely unwrap here, because we enforce an invariant that `self.rows.len() > 0`
        &mut self.rows.last_mut().unwrap().1
    }

    /// Convert this `AnnotBlock` into another `AnnotBlock` with identical [`Row`]s, but where each
    /// annotation is passed through the given function.
    pub fn map_annots<B>(self, f: impl Fn(A) -> B) -> AnnotBlock<B> {
        unsafe {
            AnnotBlock::from_annot_rows_unchecked(
                self.rows
                    .into_iter()
                    .map(|(r, annot)| (r, f(annot)))
                    .collect(),
            )
        }
    }

    /// Extend this `AnnotBlock` with the contents of another `AnnotBlock`.  This modifies `self`
    /// to have the effect of ringing `self` then `other`.  Note that this overwrites the
    /// leftover [`Row`] of `self`, replacing its annotation with that of `other`'s first [`Row`].
    pub fn extend_with(&mut self, other: Self) -> Result<(), IncompatibleStages> {
        IncompatibleStages::test_err(self.stage(), other.stage())?;
        // Remove the leftover row
        let leftover_row = self.rows.pop().unwrap().0;
        self.rows.extend(
            other
                .rows
                .into_iter()
                .map(|(r, annot)| (unsafe { leftover_row.mul_unchecked(&r) }, annot)),
        );
        Ok(())
    }

    /// Extend this `AnnotBlock` with the contents of another `AnnotBlock`, cloning the
    /// annotations.  This modifies `self` to have the effect of ringing `self` then `other`.  Note
    /// that this overwrites the leftover [`Row`] of `self`, replacing its annotation with that of
    /// `other`'s first [`Row`].
    pub fn extend_with_cloned(&mut self, other: &Self) -> Result<(), IncompatibleStages>
    where
        A: Clone,
    {
        IncompatibleStages::test_err(self.stage(), other.stage())?;
        // Remove the leftover row
        let leftover_row = self.rows.pop().unwrap().0;
        self.rows.extend(
            other
                .rows
                .iter()
                .map(|(r, annot)| (unsafe { leftover_row.mul_unchecked(r) }, annot.clone())),
        );
        Ok(())
    }
}
