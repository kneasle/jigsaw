//! A heap-allocated row of [`Bell`]s.  This is also used as a permutation.

use crate::{Bell, Stage};
use wasm_bindgen::prelude::*;

/// All the possible ways that a [`Row`] could be invalid.
///
/// Note that by the Pigeon Hole Principle, we do not need a third entry
/// (`MissingBell(`[`Bell`]`)`) because in order for a [`Bell`] to be missing, another [`Bell`]
/// must either be duplicated or out of range.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InvalidRowErr {
    /// A [`Bell`] would appear twice in the new [`Row`] (for example in `113456` or `4152357`)
    DuplicateBell(Bell),
    /// A [`Bell`] is not within the range of the [`Stage`] of the new [`Row`] (for example `7` in
    /// `12745` or `5` in `5432`).
    BellOutOfStage(Bell, Stage),
}

impl std::fmt::Display for InvalidRowErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidRowErr::DuplicateBell(bell) => {
                write!(f, "Bell {} would appear twice.", bell)
            }
            InvalidRowErr::BellOutOfStage(bell, stage) => {
                write!(f, "Bell {} is not within the stage {}", bell, stage)
            }
        }
    }
}

pub type RowResult = Result<Row, InvalidRowErr>;

/// An error created when a [`Row`] was used to permute something with the wrong length
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct IncompatibleStages {
    /// The [`Stage`] of the [`Row`] that was being permuted
    lhs_stage: Stage,
    /// The [`Stage`] of the [`Row`] that was doing the permuting
    rhs_stage: Stage,
}

impl IncompatibleStages {
    /// Compares two [`Stage`]s, returning `Ok(())` if they are equal and returning the appropriate
    /// `IncompatibleStages` error if not.
    pub(crate) fn test_err(lhs_stage: Stage, rhs_stage: Stage) -> Result<(), Self> {
        if lhs_stage == rhs_stage {
            Ok(())
        } else {
            Err(IncompatibleStages {
                lhs_stage,
                rhs_stage,
            })
        }
    }
}

impl std::fmt::Display for IncompatibleStages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Incompatible stages: {} (lhs), {} (rhs)",
            self.lhs_stage, self.rhs_stage
        )
    }
}

impl std::error::Error for IncompatibleStages {}

/// A single `Row` of [`Bell`]s.
///
/// This can be viewed as a permutation of [rounds](Row::rounds) on a given [`Stage`].
///
/// A `Row` must always be valid according to
/// [the Framework](https://cccbr.github.io/method_ringing_framework/fundamentals.html) - i.e., it
/// must contain every [`Bell`] up to its [`Stage`] once and precisely once.  This is only checked
/// in the constructors and then used as assumed knowledge to avoid further checks.  This is
/// similar to how [`&str`](str) and [`String`] are required to be valid UTF-8.
///
/// # Example
/// ```
/// use proj_core::{Bell, Row, Stage, InvalidRowErr};
///
/// // Create rounds on 8 bells.  Rounds is always valid on any `Stage`
/// let rounds_on_8 = Row::rounds(Stage::MAJOR);
/// assert_eq!(rounds_on_8.stage(), Stage::MAJOR);
/// assert_eq!(rounds_on_8.to_string(), "12345678");
///
/// // Parse a generic (valid) change from a string.  Note how invalid
/// // `char`s are skipped.  This could fail if the resulting `Row` is
/// // invalid, so we use ? to handle that possibility.
/// let queens = Row::parse("13579 | 24680")?;
/// assert_eq!(queens.stage(), Stage::ROYAL);
/// assert_eq!(queens.to_string(), "1357924680");
///
/// // If we try to parse an invalid `Row`, we get an error.  This means
/// // that we can assume that all `Row`s satisfy the Framework's definition
/// assert_eq!(
///     Row::parse("112345"),
///     Err(InvalidRowErr::DuplicateBell(Bell::from_name('1').unwrap()))
/// );
/// #
/// # Ok::<(), InvalidRowErr>(())
/// ```
#[wasm_bindgen]
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Row {
    /// The underlying [`Vec`] of [`Bell`]s
    bells: Vec<Bell>,
}

#[wasm_bindgen]
impl Row {
    /// Creates rounds on a given [`Stage`].
    ///
    /// # Example
    /// ```
    /// use proj_core::{Row, Stage};
    ///
    /// assert_eq!(Row::rounds(Stage::MINIMUS).to_string(), "1234");
    /// assert_eq!(Row::rounds(Stage::CATERS).to_string(), "123456789");
    /// ```
    pub fn rounds(stage: Stage) -> Row {
        // We skip the validity check, because it is trivially satisfied by rounds
        Row::from_vec_unchecked((0..stage.as_usize()).map(Bell::from_index).collect())
    }

    /// Creates backrounds on a given [`Stage`].
    ///
    /// # Example
    /// ```
    /// use proj_core::{Row, Stage};
    ///
    /// assert_eq!(Row::backrounds(Stage::MINIMUS).to_string(), "4321");
    /// assert_eq!(Row::backrounds(Stage::CATERS).to_string(), "987654321");
    /// ```
    pub fn backrounds(stage: Stage) -> Row {
        // We skip the validity check, because it is trivially satisfied by backrounds
        Row::from_vec_unchecked((0..stage.as_usize()).rev().map(Bell::from_index).collect())
    }

    /// Creates Queens on a given [`Stage`].
    ///
    /// # Example
    /// ```
    /// use proj_core::{Row, Stage};
    ///
    /// assert_eq!(Row::queens(Stage::MINIMUS).to_string(), "1324");
    /// assert_eq!(Row::queens(Stage::CATERS).to_string(), "135792468");
    /// ```
    pub fn queens(stage: Stage) -> Row {
        // We skip the validity check, because it is trivially satisfied by backrounds
        Row::from_vec_unchecked(
            (0..stage.as_usize())
                .step_by(2)
                .chain((1..stage.as_usize()).step_by(2))
                .map(Bell::from_index)
                .collect(),
        )
    }

    /// Returns the [`Stage`] of this `Row`.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Row, Stage};
    ///
    /// // Rounds on a given `Stage` should have that `Stage`
    /// assert_eq!(Row::rounds(Stage::MINIMUS).stage(), Stage::MINIMUS);
    /// assert_eq!(Row::rounds(Stage::SEPTUPLES).stage(), Stage::SEPTUPLES);
    ///
    /// assert_eq!(Row::parse("41325")?.stage(), Stage::DOUBLES);
    /// assert_eq!(Row::parse("321 654 987 0")?.stage(), Stage::ROYAL);
    /// # Ok::<(), proj_core::InvalidRowErr>(())
    /// ```
    #[inline]
    pub fn stage(&self) -> Stage {
        self.bells.len().into()
    }

    /// Returns a [`String`] representing this `Row`.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Row, Stage};
    ///
    /// assert_eq!(Row::rounds(Stage::MAJOR).to_string(), "12345678");
    /// assert_eq!(Row::parse("146235")?.to_string(), "146235");
    /// # Ok::<(), proj_core::InvalidRowErr>(())
    /// ```
    pub fn to_string(&self) -> String {
        let mut s = String::with_capacity(self.stage().as_usize());
        self.push_to_string(&mut s);
        s
    }

    /// A very collision-resistant hash function.  It is guarunteed to be perfectly
    /// collision-resistant on the following [`Stage`]s:
    /// - 16-bit machines: Up to 6 bells
    /// - 32-bit machines: Up to 9 bells
    /// - 64-bit machines: Up to 16 bells
    ///
    /// This hashing algorithm works by reading the row as a number using the stage as a base, thus
    /// guarunteeing that (ignoring overflow), two [`Row`]s will only be hashed to the same value
    /// if they are in fact the same.  This is ludicrously inefficient in terms of hash density,
    /// but it is fast and perfect and in most cases will suffice.
    pub fn fast_hash(&self) -> usize {
        let mut accum = 0;
        let mut multiplier = 1;
        for b in self.slice() {
            accum *= b.index() * multiplier;
            multiplier *= self.stage().as_usize();
        }
        accum
    }
}

impl Row {
    /// Parse a string into a `Row`, skipping any [`char`]s that aren't valid bell names.  This
    /// returns `Err(`[`InvalidRowErr`]`)` if the `Row` would be invalid.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Bell, Row, Stage, InvalidRowErr};
    ///
    /// // Parsing a valid Row gives back `Ok(Row)`
    /// assert_eq!(Row::parse("12543")?.to_string(), "12543");
    /// // Parsing valid rows with invalid characters gives back `Ok(Row)`
    /// assert_eq!(Row::parse("4321\t[65 78]")?.to_string(), "43216578");
    /// assert_eq!(Row::parse("3|2|1  6|5|4  9|8|7")?.to_string(), "321654987");
    /// // Parsing an invalid `Row` causes an error describing the problem
    /// assert_eq!(
    ///     Row::parse("112345"),
    ///     Err(InvalidRowErr::DuplicateBell(Bell::from_number(1).unwrap()))
    /// );
    /// assert_eq!(
    ///     Row::parse("12745"),
    ///     Err(InvalidRowErr::BellOutOfStage(
    ///         Bell::from_number(7).unwrap(),
    ///         Stage::DOUBLES
    ///     ))
    /// );
    /// # Ok::<(), InvalidRowErr>(())
    /// ```
    pub fn parse(s: &str) -> RowResult {
        Row::from_iter(s.chars().filter_map(Bell::from_name))
    }

    /// Utility function that creates a `Row` from an iterator of [`Bell`]s, performing the
    /// validity check.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Bell, Row, Stage, InvalidRowErr};
    ///
    /// // Create a valid row from an iterator over `Bell`s
    /// let iter = [0, 3, 4, 2, 1].iter().copied().map(Bell::from_index);
    /// let row = Row::from_iter(iter)?;
    /// assert_eq!(row.to_string(), "14532");
    /// // Attempt to create an invalid row from an iterator over `Bell`s
    /// // (we get an error)
    /// let iter = [0, 3, 7, 2, 1].iter().copied().map(Bell::from_index);
    /// assert_eq!(
    ///     Row::from_iter(iter),
    ///     Err(InvalidRowErr::BellOutOfStage(
    ///         Bell::from_name('8').unwrap(),
    ///         Stage::DOUBLES,
    ///     ))
    /// );
    ///
    /// # Ok::<(), InvalidRowErr>(())
    /// ```
    pub fn from_iter<I>(iter: I) -> RowResult
    where
        I: Iterator<Item = Bell>,
    {
        Row {
            bells: iter.collect(),
        }
        .check_validity()
    }

    /// Creates a `Row` from a [`Vec`] of [`Bell`]s, checking that the the resulting `Row` is valid.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Bell, InvalidRowErr, Row};
    ///
    /// // Converting a `Row` from a valid `Vec` of `Bell`s is fine
    /// assert_eq!(
    ///     Row::from_vec(vec![
    ///         Bell::from_name('4').unwrap(),
    ///         Bell::from_name('2').unwrap(),
    ///         Bell::from_name('1').unwrap(),
    ///         Bell::from_name('3').unwrap(),
    ///     ])?.to_string(),
    ///     "4213"
    /// );
    /// // Converting a `Row` from an invalid `Vec` of `Bell`s is not so fine
    /// assert_eq!(
    ///     Row::from_vec(vec![
    ///         Bell::from_name('4').unwrap(),
    ///         Bell::from_name('2').unwrap(),
    ///         Bell::from_name('1').unwrap(),
    ///         Bell::from_name('4').unwrap(),
    ///     ]),
    ///     Err(InvalidRowErr::DuplicateBell(Bell::from_name('4').unwrap()))
    /// );
    /// # Ok::<(), InvalidRowErr>(())
    /// ```
    pub fn from_vec(bells: Vec<Bell>) -> RowResult {
        Row { bells }.check_validity()
    }

    /// Creates a `Row` from a [`Vec`] of [`Bell`]s, **without** checking that the the resulting
    /// `Row` is valid.  Only use this if you're certain that the input is valid, since performing
    /// invalid operations on `Row`s is undefined behaviour.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Bell, InvalidRowErr, Row};
    ///
    /// // Converting a `Row` from a valid `Vec` of `Bell`s is fine
    /// assert_eq!(
    ///     Row::from_vec_unchecked(vec![
    ///         Bell::from_name('4').unwrap(),
    ///         Bell::from_name('2').unwrap(),
    ///         Bell::from_name('1').unwrap(),
    ///         Bell::from_name('3').unwrap(),
    ///     ]).to_string(),
    ///     "4213"
    /// );
    /// // Converting a `Row` from an invalid `Vec` of `Bell`s **works**,
    /// // but creates an invalid `Row`
    /// assert_eq!(
    ///     Row::from_vec_unchecked(vec![
    ///         Bell::from_name('4').unwrap(),
    ///         Bell::from_name('2').unwrap(),
    ///         Bell::from_name('1').unwrap(),
    ///         Bell::from_name('4').unwrap(),
    ///     ]).to_string(),
    ///     "4214"
    /// );
    /// ```
    #[inline]
    pub fn from_vec_unchecked(bells: Vec<Bell>) -> Row {
        Row { bells }
    }

    /// Checks the validity of a potential `Row`, returning it if valid and returning an
    /// [`InvalidRowErr`] otherwise (consuming the potential `Row`).
    fn check_validity(self) -> RowResult {
        // We check validity by keeping a checklist of which `Bell`s we've seen, and checking off
        // each bell as we go.
        let mut checklist = vec![false; self.stage().as_usize()];
        // Loop over all the bells to check them off in the checklist
        for b in &self.bells {
            match checklist.get_mut(b.index()) {
                // If the `Bell` is out of range of the checklist, it can't belong within the `Stage`
                // of this `Row`
                None => return Err(InvalidRowErr::BellOutOfStage(*b, self.stage())),
                // If the `Bell` has already been seen before, then it must be a duplicate
                Some(&mut true) => return Err(InvalidRowErr::DuplicateBell(*b)),
                // If the `Bell` has not been seen before, check off the checklist entry and continue
                Some(x) => *x = true,
            }
        }
        // If none of the `Bell`s caused errors, the row must be valid
        Ok(self)
    }

    /// Returns an immutable reference to the underlying slice of [`Bell`]s that makes up this
    /// `Row`.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Bell, Row};
    ///
    /// let tittums = Row::parse("15263748")?;
    /// assert_eq!(tittums.slice()[3], Bell::from_name('6').unwrap());
    /// # Ok::<(), proj_core::InvalidRowErr>(())
    /// ```
    #[inline]
    pub fn slice(&self) -> &[Bell] {
        self.bells.as_slice()
    }

    /// Returns an iterator over the [`Bell`]s in this `Row`
    #[inline]
    pub fn iter<'a>(&'a self) -> std::iter::Copied<std::slice::Iter<'a, Bell>> {
        self.slice().iter().copied()
    }

    /// Perform an in-place check that this `Row` is equal to rounds.  `x.is_rounds()` is an
    /// optimised version of `x == Row::rounds(x.stage())`.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Row, Stage};
    ///
    /// // Rounds is ... rounds (DOH)
    /// assert!(Row::rounds(Stage::MAXIMUS).is_rounds());
    /// // This is not rounds
    /// assert!(!Row::parse("18423756")?.is_rounds());
    /// # Ok::<(), proj_core::InvalidRowErr>(())
    /// ```
    pub fn is_rounds(&self) -> bool {
        self.iter().enumerate().all(|(i, b)| b.index() == i)
    }

    /// All the `Row`s formed by repeatedly permuting a given `Row`.  The first item returned will
    /// always be the input `Row`, and the last will always be `rounds`.
    ///
    /// # Example
    /// ```
    /// use proj_core::Row;
    ///
    /// // The closure of "18234567" are all the fixed-treble cyclic part heads.
    /// assert_eq!(
    ///     Row::parse("18234567")?.closure(),
    ///     vec![
    ///         Row::parse("18234567")?,
    ///         Row::parse("17823456")?,
    ///         Row::parse("16782345")?,
    ///         Row::parse("15678234")?,
    ///         Row::parse("14567823")?,
    ///         Row::parse("13456782")?,
    ///         Row::parse("12345678")?,
    ///     ]
    /// );
    /// # Ok::<(), proj_core::InvalidRowErr>(())
    /// ```
    pub fn closure(&self) -> Vec<Row> {
        let mut closure = Vec::new();
        let mut row = self.clone();
        loop {
            closure.push(row.clone());
            if row.is_rounds() {
                return closure;
            }
            row = &row * self;
        }
    }

    /// Concatenates the names of the [`Bell`]s in this `Row` to the end of a [`String`].  See also
    /// [`to_string`](Row::to_string), which returns a new [`String`] rather than concatenating to
    /// an existing one.
    ///
    /// # Example
    /// ```
    /// use proj_core::Row;
    ///
    /// let waterfall = Row::parse("6543217890")?;
    /// let mut string = "Waterfall is: ".to_string();
    /// waterfall.push_to_string(&mut string);
    /// assert_eq!(string, "Waterfall is: 6543217890");
    /// # Ok::<(), proj_core::InvalidRowErr>(())
    /// ```
    pub fn push_to_string(&self, string: &mut String) {
        for b in &self.bells {
            string.push_str(&b.name());
        }
    }
}

impl std::fmt::Debug for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Row({})", self.to_string())
    }
}

impl std::fmt::Display for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl std::ops::Index<usize> for Row {
    type Output = Bell;

    fn index(&self, index: usize) -> &Bell {
        &self.slice()[index]
    }
}

impl std::ops::Mul for Row {
    type Output = Row;

    /// Uses the RHS to permute the LHS, consuming both arguments.
    ///
    /// # Panics
    ///
    /// This panics if the [`Row`]s have different [`Stages`].
    ///
    /// # Example
    ///
    /// ```
    /// use proj_core::Row;
    ///
    /// assert_eq!(
    ///     Row::parse("13425678")? * Row::parse("43217568")?,
    ///     Row::parse("24317568")?
    /// );
    ///
    /// # Ok::<(), proj_core::InvalidRowErr>(())
    /// ```
    fn mul(self, rhs: Row) -> Row {
        &self * &rhs
    }
}

impl std::ops::Mul for &Row {
    type Output = Row;

    /// Uses the RHS to permute the LHS without consuming either argument.
    ///
    /// # Panics
    ///
    /// This panics if the [`Row`]s have different [`Stages`].
    ///
    /// # Example
    ///
    /// ```
    /// use proj_core::Row;
    ///
    /// assert_eq!(
    ///     &Row::parse("13425678")? * &Row::parse("43217568")?,
    ///     Row::parse("24317568")?
    /// );
    ///
    /// # Ok::<(), proj_core::InvalidRowErr>(())
    /// ```
    fn mul(self, rhs: &Row) -> Row {
        IncompatibleStages::test_err(self.stage(), rhs.stage()).unwrap();
        Row::from_vec_unchecked(rhs.iter().map(|b| self[b.index()]).collect())
    }
}
