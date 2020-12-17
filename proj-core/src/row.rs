//! A heap-allocated row of [`Bell`]s.

use crate::{Bell, Stage};

// Imports that are only used by doc comments (meaning rustc will generate a warning if not
// suppressed)
#[allow(unused_imports)]
use crate::Perm;

/// All the possible ways that a [`Row`] could be invalid.
///
/// Note that by the Pigeon Hole Principle, we do not need a third entry
/// (`MissingBell(`[`Bell`]`)`) because in order for a [`Bell`] to be missing, another [`Bell`]
/// must either be duplicated or out of range.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InvalidRowErr {
    /// A [`Bell`] would appear twice in the new [`Row`] (for example in `113456` or `4152357`)
    DuplicateBell(Bell),
    /// A [`Bell`] is not within the range of the [`Stage`] of the new [`Row`] (for example in
    /// `12745` or `5432`).
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

/// A single `Row` of [`Bell`]s.
///
/// This can be viewed as a [`Perm`]utation of [rounds](Row::rounds) on a given [`Stage`].
///
/// A `Row` is **required** to be a valid `Row` according to
/// [the Framework](https://cccbr.github.io/method_ringing_framework/fundamentals.html) - i.e., it
/// must contain every [`Bell`] up to the [`Stage`] once and precisely once.  This is checked in
/// all the constructors and then assumed as an invariant.
///
/// This is similar to how [`&str`](str) and [`String`] are required to be valid UTF-8.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Row {
    /// The underlying [`Vec`] of [`Bell`]s
    bells: Vec<Bell>,
}

impl Row {
    /// Creates a [`Row`] representing rounds on a given [`Stage`].
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
        Row {
            bells: (0..stage.as_usize()).map(Bell::from_index).collect(),
        }
    }

    /// Parse a string into a `Row`, skipping any [`char`]s that aren't valid bell names.  This
    /// returns `Err(`[`InvalidRowErr`]`)` if the `Row` would be invalid.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Bell, Row, Stage};
    /// use proj_core::row::InvalidRowErr;
    ///
    /// // Parsing a valid Row gives back `Ok(Row)`
    /// assert_eq!(Row::parse("12543").unwrap().to_string(), "12543");
    /// // Parsing valid rows with invalid characters gives back `Ok(Row)`
    /// assert_eq!(Row::parse("4321\t[65 78]").unwrap().to_string(), "43216578");
    /// assert_eq!(Row::parse("3|2|1  6|5|4  9|8|7").unwrap().to_string(), "321654987");
    /// // Parsing an invalid `Row` causes an error describing the problem
    /// assert_eq!(
    ///     Row::parse("112345"),
    ///     Err(InvalidRowErr::DuplicateBell(Bell::from_number(1).unwrap()))
    /// );
    /// assert_eq!(
    ///     Row::parse("12745"),
    ///     Err(InvalidRowErr::BellOutOfStage(Bell::from_number(7).unwrap(), Stage::DOUBLES))
    /// );
    /// ```
    pub fn parse(s: &str) -> RowResult {
        Row::from_iter(s.chars().filter_map(Bell::from_name))
    }

    /// Utility function that creates a `Row` from an iterator of [`Bell`]s, performing the
    /// validity check.
    fn from_iter<I>(iter: I) -> RowResult
    where
        I: Iterator<Item = Bell>,
    {
        Row {
            bells: iter.collect(),
        }
        .check_validity()
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

    /// Returns the [`Stage`] of this `Row`.
    #[inline]
    pub fn stage(&self) -> Stage {
        self.bells.len().into()
    }

    /// Concatenates the names of the [`Bell`]s in this `Row` to the end of a [`String`]
    pub fn push_to_string(&self, string: &mut String) {
        for b in &self.bells {
            string.push_str(&b.name());
        }
    }

    /// Returns a [`String`] representing this `Row`.
    pub fn to_string(&self) -> String {
        let mut s = String::with_capacity(self.stage().as_usize());
        self.push_to_string(&mut s);
        s
    }
}

impl std::fmt::Display for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
