//! Module for parsing and handling place notation

use crate::{Bell, Stage};
use itertools::Itertools;
use std::fmt::{Display, Formatter};

static CROSS_NOTATIONS: [&str; 3] = ["-", "x", "X"];

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ParseError {
    PlaceOutOfStage { place: usize, stage: Stage },
    AmbiguousPlacesBetween { p: usize, q: usize },
    OddStageCross { stage: Stage },
    NoPlacesGiven,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::OddStageCross { stage } => {
                write!(f, "Cross notation for odd stage {}", stage)
            }
            ParseError::PlaceOutOfStage { place, stage } => {
                write!(
                    f,
                    "Place '{}' is out stage {}",
                    Bell::from_index(*place),
                    stage
                )
            }
            ParseError::AmbiguousPlacesBetween { p, q } => write!(
                f,
                "Ambiguous gap of {} bells between places '{}' and '{}'.",
                q - p - 1,
                Bell::from_index(*p),
                Bell::from_index(*q)
            ),
            ParseError::NoPlacesGiven => {
                write!(f, "No places given.  Use 'x' or '-' for a cross.")
            }
        }
    }
}

/// A single piece of place notation on any [`Stage`].
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PlaceNot {
    /// A **0-indexed** list of which places are made during this `PlaceNot`.  We maintain the
    /// following invariants:
    /// - The places are fully expanded and unambiguous.  This means that every place made is
    ///   explicitly stored, with no implicit or ambiguous places.
    ///
    ///   For example, suppose the [`Stage`] is [`MAJOR`](Stage::MAJOR).
    ///    - The place notation "4" has an implicit place made at lead and so would be stored as
    ///      `vec![0, 3]`.
    ///    - The place notation "146" has an implicit/ambiguous place made in 5ths, so would be
    ///      stored as `vec![0, 3, 4, 5]`.
    /// - The places are stored **in ascending order**.  So "4817" would be stored as
    ///   `vec![0, 3, 6, 7]`.
    ///
    /// Enforcing these invariants improves the the speed of permutation and equality tests at the
    /// cost of (slightly) slower parsing, but I think this trade-off is justified since all
    /// `PlaceNot`s are parsed only once but permuting/equality tests happen many many times.
    places: Vec<usize>,
    stage: Stage,
}

impl PlaceNot {
    /// Parse a string, interpreting it as a single `PlaceNot` of a given [`Stage`].  Like
    /// [`Row::parse_with_stage`], this ignores chars that don't correspond to valid [`Bell`]
    /// names, including `&`, `.`, `,` and `+` which have reserved meanings in blocks of place
    /// notation.  This will expand implicit places (even between two written places) but will fail
    /// if there is any kind of ambiguity, returning a [`ParseError`] describing the problem.  This
    /// also runs in `O(n)` time except for sorting the places which takes `O(n log n)` time.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Stage, PlaceNot, place_not::ParseError};
    ///
    /// // Parsing a valid place notation is OK
    /// assert_eq!(PlaceNot::parse("14", Stage::MAJOR)?.to_string(), "14");
    /// // Ordering and rogue chars don't matter
    /// assert_eq!(PlaceNot::parse("  4|7~18", Stage::ROYAL)?.to_string(), "1478");
    /// // Implicit places will be expanded
    /// assert_eq!(PlaceNot::parse("467", Stage::MAXIMUS)?.to_string(), "14567T");
    ///
    /// // Parsing invalid or ambiguous PN is not OK, and warns you of the problem
    /// assert_eq!(
    ///     PlaceNot::parse("14T", Stage::MAJOR).unwrap_err().to_string(),
    ///     "Place 'T' is out stage Major"
    /// );
    /// assert_eq!(
    ///     PlaceNot::parse("15", Stage::MAJOR).unwrap_err().to_string(),
    ///     "Ambiguous gap of 3 bells between places '1' and '5'."
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn parse(s: &str, stage: Stage) -> Result<Self, ParseError> {
        // If the string is any one of the cross strings, then return CROSS
        if CROSS_NOTATIONS.contains(&s) {
            return Self::cross(stage).ok_or(ParseError::OddStageCross { stage });
        }
        // Parse the string into bell indices, ignoring any invalid characters
        let mut parsed_places: Vec<usize> = s
            .chars()
            .filter_map(Bell::from_name)
            .map(Bell::index)
            .collect();
        // Check if we were given no places (I'm making this an error because '-' should be used
        // instead)
        parsed_places.last().ok_or(ParseError::NoPlacesGiven)?;
        // Sort the places into ascending order
        parsed_places.sort();
        // Check if any of the bells are out of range
        if let Some(out_of_range_place) = parsed_places.last().filter(|p| **p >= stage.as_usize()) {
            return Err(ParseError::PlaceOutOfStage {
                place: *out_of_range_place,
                stage,
            });
        }

        // Rebuild to a new Vec when adding places to avoid quadratic behaviour
        let mut places = Vec::with_capacity(parsed_places.len() + 5);
        // Add implicit place in lead
        parsed_places
            .first()
            .filter(|p| *p % 2 == 1)
            .map(|_| places.push(0));
        // Copy the contents of `parsed_places`, inserting implicit places where necessary
        for (p, q) in parsed_places.iter().copied().tuple_windows() {
            // Add `p` to `places`
            places.push(p);
            // Check if there is an implicit place made between these, or if the place notation is
            // ambiguous
            let num_intermediate_places = q - p - 1;
            if num_intermediate_places == 1 {
                places.push(p + 1);
            } else if num_intermediate_places % 2 == 1 {
                // Any other even number causes an error
                return Err(ParseError::AmbiguousPlacesBetween { p, q });
            }
            // `q` will be pushed in the next loop iteration
        }
        // Copy the last element from `places`.  This is a special case, because `tuple_windows`
        // won't return the last element as the first element of a tuple window (because there's
        // nothing to pair it with)
        parsed_places.last().map(|p| places.push(*p));
        // Add implicit place at the back if necessary
        parsed_places
            .last()
            .filter(|p| (stage.as_usize() - *p) % 2 == 0)
            .map(|_| places.push(stage.as_usize() - 1));
        // Create struct and return.  We don't need to sort the places, because we only push to
        // them in ascending order.
        Ok(PlaceNot { places, stage })
    }

    /// Returns a new `PlaceNot` representing the 'cross' notation on a given stage.  This will
    /// fail if `stage` doesn't have an even number of bells.
    ///
    /// # Example
    /// ```
    /// use proj_core::{PlaceNot, Stage};
    ///
    /// // These are crosses
    /// assert_eq!(
    ///     PlaceNot::cross(Stage::MAJOR).unwrap(),
    ///     PlaceNot::parse("x", Stage::MAJOR)?
    /// );
    /// # Ok::<(), proj_core::place_not::ParseError>(())
    /// ```
    pub fn cross(stage: Stage) -> Option<Self> {
        if stage.as_usize() % 2 == 0 {
            Some(PlaceNot {
                places: Vec::new(),
                stage,
            })
        } else {
            None
        }
    }

    /// Checks if this `PlaceNot` corresponds to the 'cross' notation.
    ///
    /// # Example
    /// ```
    /// use proj_core::{PlaceNot, Stage};
    ///
    /// // These are crosses
    /// assert!(PlaceNot::cross(Stage::MAJOR).unwrap().is_cross());
    /// assert!(PlaceNot::parse("x", Stage::MAJOR)?.is_cross());
    /// // These are not
    /// assert!(!PlaceNot::parse("14", Stage::MAJOR)?.is_cross());
    /// assert!(!PlaceNot::parse("3", Stage::TRIPLES)?.is_cross());
    /// # Ok::<(), proj_core::place_not::ParseError>(())
    /// ```
    pub fn is_cross(&self) -> bool {
        self.places.is_empty()
    }
}

impl Display for PlaceNot {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_cross() {
            // Always display cross notation as '-' to avoid confusion with bell names
            write!(f, "-")
        } else {
            // Otherwise concatenate all the bell names together
            write!(
                f,
                "{}",
                self.places
                    .iter()
                    .map(|&x| Bell::from_index(x).name())
                    .join("")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ParseError;
    use crate::{PlaceNot, Stage};

    #[test]
    fn parse_ok() {
        for (inp_string, stage, exp_places) in &[
            // No implict places
            ("14", Stage::MAJOR, vec![0, 3]),
            ("1256", Stage::MINOR, vec![0, 1, 4, 5]),
            ("16", Stage::MAXIMUS, vec![0, 5]),
            ("127", Stage::TRIPLES, vec![0, 1, 6]),
            ("3", Stage::CATERS, vec![2]),
            // Implicit places in lead
            ("4", Stage::MINIMUS, vec![0, 3]),
            ("234", Stage::MINOR, vec![0, 1, 2, 3]),
            ("478", Stage::MAJOR, vec![0, 3, 6, 7]),
            ("470", Stage::ROYAL, vec![0, 3, 6, 9]),
            ("2", Stage::TWO, vec![0, 1]),
            // Implicit places in lie
            ("3", Stage::MAJOR, vec![2, 7]),
            ("12", Stage::DOUBLES, vec![0, 1, 4]),
            ("1", Stage::TWO, vec![0, 1]),
            ("14", Stage::CINQUES, vec![0, 3, 10]),
            // Implicit places between two other places
            ("146", Stage::MAJOR, vec![0, 3, 4, 5]),
            ("13", Stage::SINGLES, vec![0, 1, 2]),
            ("13", Stage::TRIPLES, vec![0, 1, 2]),
            ("135", Stage::TRIPLES, vec![0, 1, 2, 3, 4]),
            // Implicit places in multiple places
            ("23", Stage::MAJOR, vec![0, 1, 2, 7]),
            ("4", Stage::TRIPLES, vec![0, 3, 6]),
            ("45", Stage::MINOR, vec![0, 3, 4, 5]),
            ("46", Stage::CATERS, vec![0, 3, 4, 5, 8]),
            // Out of order places
            ("6152", Stage::MINOR, vec![0, 1, 4, 5]),
            ("342", Stage::MINOR, vec![0, 1, 2, 3]),
            ("32", Stage::MAJOR, vec![0, 1, 2, 7]),
            ("21", Stage::DOUBLES, vec![0, 1, 4]),
            // Misc characters
            ("2\t  1 |", Stage::DOUBLES, vec![0, 1, 4]),
            ("   6\n?15&2!", Stage::MINOR, vec![0, 1, 4, 5]),
        ] {
            let pn = PlaceNot::parse(inp_string, *stage).unwrap();
            assert_eq!(pn.stage, *stage);
            assert_eq!(pn.places, *exp_places);
        }
    }

    #[test]
    fn parse_err_odd_bell_cross() {
        for &stage in &[
            Stage::SINGLES,
            Stage::DOUBLES,
            Stage::TRIPLES,
            Stage::CATERS,
            Stage::CINQUES,
            Stage::SEPTUPLES,
            Stage::SEXTUPLES,
        ] {
            for cross_not in &["x", "X", "-"] {
                assert_eq!(
                    PlaceNot::parse(*cross_not, stage),
                    Err(ParseError::OddStageCross { stage })
                );
            }
        }
    }

    #[test]
    fn parse_err_place_out_of_stage() {
        for &(inp_string, stage, place) in &[
            ("148", Stage::MINIMUS, 7),
            ("91562", Stage::MINOR, 8),
            ("  3", Stage::TWO, 2),
        ] {
            assert_eq!(
                PlaceNot::parse(inp_string, stage),
                Err(ParseError::PlaceOutOfStage { stage, place })
            );
        }
    }

    #[test]
    fn parse_err_no_places_given() {
        for stage in 0..12 {
            assert_eq!(
                PlaceNot::parse("", Stage::from(stage)),
                Err(ParseError::NoPlacesGiven)
            );
        }
    }

    #[test]
    fn parse_err_ambiguous_gap() {
        for &(inp_string, stage, exp_p, exp_q) in &[
            // No implict places
            ("15", Stage::MAJOR, 0, 4),
            ("39", Stage::ROYAL, 2, 8),
            ("1925", Stage::MAXIMUS, 4, 8),
            ("1026", Stage::ROYAL, 1, 5),
        ] {
            assert_eq!(
                PlaceNot::parse(inp_string, stage),
                Err(ParseError::AmbiguousPlacesBetween { p: exp_p, q: exp_q })
            );
        }
    }
}
