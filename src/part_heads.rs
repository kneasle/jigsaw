//! Code for part head specification.

use std::{collections::HashSet, ops::Deref};

use bellframe::{IncompatibleStages, InvalidRowError, Row, RowBuf, Stage};
use serde::Serialize;

/// The possible ways that parsing a part head specification can fail
pub type ParseError = InvalidRowError;

/// A struct that stores a specification for a set of part heads.  This contains the [`String`]
/// that the user entered into the part head box (which must be valid), as well as the
/// generated set of part heads.  The following invariants must be upheld:
/// - There is always at least one part head (0 part compositions can't exist)
/// - All the part_heads have the same [`Stage`]
#[derive(Debug, Clone, Eq, Serialize)]
pub struct PartHeads {
    spec: String,
    #[serde(serialize_with = "crate::ser_utils::ser_rows")]
    rows: Vec<RowBuf>,
    /// A `HashSet` containing the same [`Row`]s as `rows`, but kept for faster lookups
    #[serde(skip)]
    set: HashSet<RowBuf>,
    is_group: bool,
}

// The invariant of always having at least one part head means that `is_empty` would always
// return `false`
#[allow(clippy::len_without_is_empty)]
impl PartHeads {
    /// Given a [`str`]ing specifying some part heads, attempts to parse and expand these PHs,
    /// or generate a [`ParseError`] explaining the problem.
    pub fn parse(s: &str, stage: Stage) -> Result<Self, ParseError> {
        let generators = s
            .split(',')
            .map(|sub_str| RowBuf::parse_with_stage(sub_str, stage))
            .collect::<Result<Vec<_>, InvalidRowError>>()?;
        let (is_group, set, rows) = Self::gen_cartesian_product(generators);
        Ok(PartHeads {
            set,
            rows,
            is_group,
            spec: s.to_owned(),
        })
    }

    fn gen_cartesian_product(generators: Vec<RowBuf>) -> (bool, HashSet<RowBuf>, Vec<RowBuf>) {
        let row_sets: Vec<_> = generators.iter().map(|r| r.closure_from_rounds()).collect();
        let part_heads =
            Row::multi_cartesian_product(row_sets.iter().map(|b| b.iter().map(|r| r.as_row())))
                .unwrap();
        (
            Row::is_group(part_heads.iter().map(RowBuf::as_row)).unwrap(),
            part_heads.iter().cloned().collect(),
            part_heads,
        )
    }

    #[allow(dead_code)]
    fn gen_least_group(generators: Vec<RowBuf>) -> (bool, HashSet<RowBuf>, Vec<RowBuf>) {
        let set = Row::least_group_containing(generators.iter().map(Deref::deref))
            // This unwrap is safe because all the input rows came from
            // `Row::parse_with_stage`
            .unwrap();
        let mut part_heads = set.iter().cloned().collect::<Vec<_>>();
        part_heads.sort();
        (true, set, part_heads)
    }

    /// Returns a string slice of the specification string that generated these `PartHeads`.
    #[inline]
    pub fn spec_string(&self) -> &str {
        &self.spec
    }

    /// The number of part heads in this set of `PartHeads`.
    #[inline]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Returns a slice over the part heads in this set of `PartHeads`
    #[inline]
    pub fn rows(&self) -> &[RowBuf] {
        &self.rows
    }

    /// Returns a slice over the part heads in this set of `PartHeads`
    #[inline]
    pub fn stage(&self) -> Stage {
        self.rows[0].stage()
    }

    /// Given a pair of [`Row`], determines if they should be deemed 'equivalent' under these
    /// `PartHeads`.  I.e. this means that taking any [`Row`] and applying the transposition
    /// between `from` and `to` should produce the same [`Row`]s under part expansion as the
    /// original.
    pub fn are_equivalent(&self, from: &Row, to: &Row) -> Result<bool, IncompatibleStages> {
        // Calculate the transposition `from -> to`, and check that all the stages match
        let transposition = from.tranposition_to(to)?;
        IncompatibleStages::test_err(self.stage(), transposition.stage())?;
        if self.is_group {
            // If the part heads form a group, then any pair of rows whos transposition is
            // contained in the group is considered equal
            Ok(self.set.contains(&transposition))
        } else {
            // PERF: Store this result in a `RefCell<HashMap<Row, bool>>`
            let mut transposed_row_buf = RowBuf::empty();
            for r in &self.rows {
                // The unsafety here is OK because all the rows in `self` must have the same
                // stage, and we checked that `transposition` shares that Stage.
                unsafe { r.mul_into_unchecked(&transposition, &mut transposed_row_buf) };
                if !self.set.contains(&transposed_row_buf) {
                    // If any of the transposed rows aren't in the group, then we return false
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

// Two PartHeads are equal if their specifications are the same; the `part_heads` vec is
// dependent on the spec so if the specs are equal, the `part_heads` must be too.
impl PartialEq for PartHeads {
    fn eq(&self, other: &PartHeads) -> bool {
        self.spec == other.spec
    }
}