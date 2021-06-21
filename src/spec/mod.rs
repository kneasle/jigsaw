use std::{
    cell::{Cell, RefCell},
    fmt::{Display, Formatter},
    rc::Rc,
};

use crate::{
    comp::MethodEdit,
    derived_state::{CallLabel, DerivedCall, DerivedFold, ExpandedRow, MethodLabel},
};
use bellframe::{
    place_not::PnBlockParseError, AnnotBlock, AnnotRow, Bell, Call, IncompatibleStages, Method,
    PnBlock, Row, RowBuf, Stage,
};
use serde::{Deserialize, Serialize};

pub mod save_load;

// Imports used solely by doc comments
#[allow(unused_imports)]
use crate::derived_state::DerivedState;

// Pretend the `PartHeads` is contained within this module, not it's own submodule
pub use self::part_heads::PartHeads;

/* ========== PART HEADS ========== */

/// A module to hold the code for part head specification.  This is in its own module to hide the
/// fields from the rest of the code and make sure that the only way to construct a [`PartHeads`]
/// is to use the fallible constructors provided (which guarutee that the [`PartHeads`]s created
/// uphold all the required invariants).
pub mod part_heads {
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
}

/* ========== METHOD HANDLING ========== */

/// The specification of what a method is in this composition.
#[derive(Debug, Clone)]
pub struct MethodSpec {
    name: RefCell<String>,
    shorthand: RefCell<String>,
    method: Method,
    place_not_string: String,
    is_panel_open: Cell<bool>,
}

impl MethodSpec {
    /// Creates a new `MethodSpec` from its parts, adding [`Cell`]/[`RefCell`]s when necessary
    pub fn new(name: String, shorthand: String, pn: String, block: &PnBlock) -> Self {
        MethodSpec {
            name: RefCell::new(name),
            shorthand: RefCell::new(shorthand),
            method: Method::with_lead_end(String::new(), block),
            place_not_string: pn,
            is_panel_open: Cell::new(false),
        }
    }

    /// Creates a new `MethodSpec` by parsing a string of place notation
    pub fn from_pn(
        name: String,
        shorthand: String,
        pn: String,
        stage: Stage,
    ) -> Result<Self, PnBlockParseError> {
        let block = PnBlock::parse(&pn, stage)?;
        Ok(Self::new(name, shorthand, pn, &block))
    }

    #[inline]
    pub fn name(&self) -> String {
        self.name.borrow().to_owned()
    }

    #[inline]
    pub fn shorthand(&self) -> String {
        self.shorthand.borrow().to_owned()
    }

    #[inline]
    pub fn place_not_string(&self) -> &str {
        &self.place_not_string
    }

    /// Creates a new `MethodEdit` from this PlaceNot
    pub fn to_edit(&self) -> MethodEdit {
        MethodEdit::with_pn_string(
            self.name(),
            self.shorthand(),
            self.method.stage(),
            self.place_not_string.clone(),
        )
    }
}

/// The location of a [`Row`] within a method.  This is used to generate method splice text and
/// calculate ATW stats.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MethodRef {
    method_index: usize,
    sub_lead_index: usize,
}

impl MethodRef {
    #[inline]
    pub fn method_index(&self) -> usize {
        self.method_index
    }

    #[inline]
    pub fn sub_lead_index(&self) -> usize {
        self.sub_lead_index
    }
}

/* ========== CALL HANDLING ========== */

/// A wrapper around [`Call`], which adds extra information required by this project
#[derive(Debug, Clone)]
pub struct CallSpec {
    call: Call,
    calling_positions: Vec<String>,
}

impl CallSpec {
    /// Generates the [`CallLabel`] which represents this call placed at a given [`Row`]
    fn to_label(&self, index: usize, start_rows: &[RowBuf]) -> CallLabel {
        let tenor = Bell::tenor(start_rows[0].stage()).unwrap();
        CallLabel::new(
            index,
            self.call.notation(),
            start_rows.iter().map(|r| {
                // Get the place of the tenor at the _start_ of the call
                let place_at_start = r.place_of(tenor).unwrap();
                // Use the transposition of the call to generate where the tenor will be at the
                // _end_ of the call
                let place_at_end = self
                    .call
                    .transposition()
                    .place_of(Bell::from_index(place_at_start))
                    .unwrap();
                // Use this resulting place as an index find the call label
                self.calling_positions[place_at_end].as_str()
            }),
        )
    }

    /// Creates a `14` lead-end bob, with calling positions
    pub fn le_14_bob(stage: Stage) -> Self {
        if stage != Stage::MAJOR {
            unimplemented!();
        }
        CallSpec {
            call: Call::le_bob(PnBlock::parse("14", stage).unwrap()),
            calling_positions: "LIBFVMWH".chars().map(|c| c.to_string()).collect(),
        }
    }

    /// Creates a `14` lead-end bob, with calling positions
    pub fn le_1234_single(stage: Stage) -> Self {
        if stage != Stage::MAJOR {
            unimplemented!();
        }
        CallSpec {
            call: Call::le_single(PnBlock::parse("1234", stage).unwrap()),
            calling_positions: "LBTFVMWH".chars().map(|c| c.to_string()).collect(),
        }
    }

    /// Generates a [`DerivedCall`] from this `CallSpec`
    pub fn to_derived_call(&self) -> DerivedCall {
        DerivedCall::new(self.call.notation(), self.call.location().to_owned())
    }
}

/// The specification of where within a [`Call`] a given row comes.  This is used to generate the
/// call labels on the fly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallRef {
    call_index: usize,
    row_index: usize,
}

impl CallRef {
    #[inline]
    pub fn call_index(&self) -> usize {
        self.call_index
    }

    #[inline]
    pub fn sub_lead_index(&self) -> usize {
        self.row_index
    }
}

/* ========== ROW ANNOTATIONS ========== */

/// A point in the composition where the [`Row`]s could be folded
#[derive(Debug, Clone, Serialize)]
pub struct Fold {
    is_open: Cell<bool>,
}

impl Fold {
    fn from_sub_lead_index(index: usize) -> Option<Self> {
        if index == 0 {
            Some(Fold {
                is_open: Cell::from(true),
            })
        } else {
            None
        }
    }
}

/// The information that every [`Row`] in a [`Frag`] is annotated with.
#[derive(Debug, Clone, Default)]
struct Annot {
    is_lead_end: bool,
    method: Option<MethodRef>,
    call: Option<CallRef>,
    fold: Option<Fold>,
}

impl Annot {
    fn row_from_course_iter(
        method_index: usize,
        call: Option<CallRef>,
        sub_lead_index: usize,
        lead_loc: Option<&str>,
        row: RowBuf,
    ) -> AnnotRow<Self> {
        AnnotRow::new(
            row,
            Annot {
                is_lead_end: lead_loc.is_some(),
                method: Some(MethodRef {
                    method_index,
                    sub_lead_index,
                }),
                call,
                fold: Fold::from_sub_lead_index(sub_lead_index),
            },
        )
    }
}

/* ========== FRAGMENTS ========== */

/// The possible ways splitting a [`Frag`] could fail
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum FragSplitError {
    /// Splitting the [`Frag`] would have produced a [`Frag`] with no rows
    ZeroLengthFrag,
    /// The index we were given by JS points outside the bounds of the [`Frag`] array
    IndexOutOfRange { index: usize, num_frags: usize },
}

impl Display for FragSplitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FragSplitError::ZeroLengthFrag => write!(f, "Can't create a 0-length Frag"),
            FragSplitError::IndexOutOfRange { index, num_frags } => {
                write!(
                    f,
                    "Frag #{} out of range of slice with len {}",
                    index, num_frags
                )
            }
        }
    }
}

/// The possible ways setting/removing a [`Call`] could fail
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SetCallError {
    /// The index we were given by JS points outside the bounds of the [`Frag`] array
    FragOutOfRange {
        index: usize,
        num_frags: usize,
    },
    MethodOutOfRange {
        index: usize,
        num_methods: usize,
    },
    RowOutOfRange {
        index: usize,
        num_rows: usize,
    },
    CallOutOfRange {
        index: usize,
        num_calls: usize,
    },
    NoMethodAtRow(usize),
    WrongLocation {
        call_loc: String,
        actual_loc: String,
    },
    ReplacingIncompleteCall,
    OverlappingCalls(CallRef),
    NoCallLocation,
    NoChange,
}

impl Display for SetCallError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SetCallError::FragOutOfRange { index, num_frags } => write!(
                f,
                "Editing frag #{} but there are only {} frags",
                index, num_frags
            ),
            SetCallError::MethodOutOfRange { index, num_methods } => write!(
                f,
                "Editing method #{} but there are only {} methods",
                index, num_methods
            ),
            SetCallError::RowOutOfRange { index, num_rows } => write!(
                f,
                "Trying to replace row {} but frag only has {} rows",
                index, num_rows
            ),
            SetCallError::CallOutOfRange { index, num_calls } => write!(
                f,
                "Setting call #{} but there are only {} calls",
                index, num_calls
            ),
            SetCallError::WrongLocation {
                call_loc,
                actual_loc,
            } => write!(
                f,
                "Call want's location '{}' but true location is '{}'",
                call_loc, actual_loc
            ),
            SetCallError::ReplacingIncompleteCall => write!(f, "Can't replace an incomplete call"),
            SetCallError::NoMethodAtRow(ind) => write!(f, "No method annotation at row {}", ind),
            SetCallError::OverlappingCalls(i) => {
                write!(
                    f,
                    "Calls can't overlap (replacing row {} of call #{})",
                    i.row_index, i.call_index
                )
            }
            SetCallError::NoCallLocation => write!(f, "Can't modify a row with no call location"),
            SetCallError::NoChange => write!(f, "No change occured"),
        }
    }
}

impl std::error::Error for SetCallError {}

/// A single unexpanded fragment of a composition
#[derive(Clone, Debug)]
pub struct Frag {
    start_row: RowBuf,
    block: Rc<AnnotBlock<Annot>>,
    is_muted: bool,
    x: f32,
    y: f32,
}

// Frags cannot have length 0, so an `is_empty` method would always return `false`
#[allow(clippy::len_without_is_empty)]
impl Frag {
    /* Getters */

    /// Returns the [`Stage`] of this `Frag`
    pub fn stage(&self) -> Stage {
        self.first_row().stage()
    }

    /// Returns the (x, y) coordinates of this `Frag`
    pub fn pos(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    /// The number of rows in this `Frag` (**not including the leftover row**)
    #[inline]
    pub fn len(&self) -> usize {
        self.block.len()
    }

    /// Returns the first [`AnnotatedRow`] of this `Frag`.  This does not return an [`Option`]
    /// because `Frag`s must have length at least 1, meaning that there is always a first
    /// [`AnnotatedRow`].
    #[inline]
    fn first_row(&self) -> &AnnotRow<Annot> {
        self.block.first_annot_row()
    }

    /* Setters/mutating operations */

    /// Updates the coordinates of this `Frag` to match the new ones
    pub fn move_to(&mut self, new_x: f32, new_y: f32) {
        self.x = new_x;
        self.y = new_y;
    }

    /// Toggles whether or not this [`Frag`] is is muted
    pub fn toggle_mute(&mut self) {
        self.is_muted = !self.is_muted;
    }

    /// Splits this fragment into two pieces so that the first one has length `split_index`.  Both
    /// `Frag`s will inherit all values from `self`, except that the 2nd one will have y-coordinate
    /// specified by `new_y`.
    fn split(&mut self, split_index: usize, new_y: f32) -> Result<Frag, FragSplitError> {
        let (split_row, block) = Rc::make_mut(&mut self.block)
            .split(split_index)
            .ok_or(FragSplitError::ZeroLengthFrag)?;
        Ok(Self::new(
            &self.start_row * &split_row,
            block,
            self.x,
            new_y,
            self.is_muted,
        ))
    }

    /// Sets or clears which [`Call`] is made at a given location
    fn set_call(
        &self,
        row_ind: usize,
        call_ind: Option<usize>,
        calls: &[Rc<CallSpec>],
        methods: &[Rc<MethodSpec>],
    ) -> Result<Frag, SetCallError> {
        /* === UNPACK USEFUL VALUES WE'RE GOING TO NEED === */

        let call = if let Some(i) = call_ind {
            Some(
                calls
                    .get(i)
                    .ok_or_else(|| SetCallError::CallOutOfRange {
                        index: i,
                        num_calls: calls.len(),
                    })?
                    .as_ref(),
            )
        } else {
            None
        };
        let annotation =
            self.block
                .get_annot(row_ind)
                .ok_or_else(|| SetCallError::RowOutOfRange {
                    index: row_ind,
                    num_rows: self.block.len(),
                })?;
        let method_ref = annotation
            .method
            .ok_or(SetCallError::NoMethodAtRow(row_ind))?;
        let method_spec = methods
            .get(method_ref.method_index)
            .ok_or_else(|| SetCallError::MethodOutOfRange {
                index: method_ref.method_index,
                num_methods: methods.len(),
            })?
            .as_ref();
        let location = method_spec
            .method
            .get_label(method_ref.sub_lead_index)
            .ok_or(SetCallError::NoCallLocation)?;
        let current_call = annotation.call.map(|c| calls[c.call_index].as_ref());

        /* === EARLY RETURN FOR EASY TO CHECK ERRORS === */

        // No effect will occur if `call` and `current_call` are equal up to pointer equality -
        // i.e. they're either both `None` or they are `Some(a) and `Some(b)` where
        // `std::ptr::eq(a, b)`.
        if match (call, current_call) {
            (None, None) => true,
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            _ => false,
        } {
            return Err(SetCallError::NoChange);
        }
        // If the row is already part way through a call, then inserting this call would cause
        // calls to overlap, which is not allowed.
        // TODO: Could this be replaced with some combinator magic?
        if let Some(call_ref) = annotation.call.filter(|c| c.row_index != 0) {
            return Err(SetCallError::OverlappingCalls(call_ref));
        }
        // If the call requires a different location to the one we've got, then report that
        call.filter(|c| c.call.location() != location)
            .map_or(Ok(()), |c| {
                Err(SetCallError::WrongLocation {
                    call_loc: c.call.location().to_owned(),
                    actual_loc: location.to_owned(),
                })
            })?;

        /* === ACTUALLY CHANGE THE CALL ===
         *
         * This is easier said than done because calls are allowed to change the length of the lead
         * that they exist in (and the fact that we could be replacing an existing call).  However,
         * at its core, calls are simply removing a chunk of place notation and replacing it with a
         * new chunk (the lengths don't have to match, but all calls must behave this way).
         */

        /* STEP 1: UNDO THE EXISTING CALL (IF IT EXISTS) */

        // This reports that, when doing the call substitution, the next `consumed_rows` after
        // `row_ind` should be ignored and replaced by the next `exported_plain_rows` rows of the
        // plain lead.  This has the effect of undoing the original call without cloning data.
        let (consumed_rows, num_plain_rows) = match current_call {
            Some(cur_c) => {
                // Check that the call we're removing actually appears in full
                let mut real_row_iter = self.block.annot_rows().iter().skip(row_ind);
                for i in 0..cur_c.call.len() {
                    let real_row = real_row_iter
                        .next()
                        .ok_or(SetCallError::ReplacingIncompleteCall)?;
                    real_row
                        .annot()
                        .call
                        .filter(|c| {
                            // This row is valid to remove if it points to the right index within
                            // the right call.
                            std::ptr::eq(calls[c.call_index].as_ref(), cur_c) && c.row_index == i
                        })
                        .ok_or(SetCallError::ReplacingIncompleteCall)?;
                }
                // If the call does appear in full, then return it's covering behaviour but
                // reversed
                (cur_c.call.len(), cur_c.call.cover_len())
            }
            // If there's no call to undo, then no rows get replaced
            None => (0, 0),
        };

        /* STEP 2: REBUILD THE BLOCK WITH THE NEW CALL
         *
         * Whilst doing this, we need to take into account the replacement done by step 1.  We also
         * check that we aren't generating overlapping calls (e.g. we add a '14' LE bob to a lead
         * of Grandsire which already has a call).
         *
         * TODO: I think the `else` statement is actually a special case of the `if`, so it might
         * be cleaner to merge them at some point
         */

        // Everything before this call happens is untouched by this operation
        let mut new_block = self.block.prefix(row_ind);
        if let Some(c) = call {
            /* CASE 1: WE'RE APPENDING A CALL */

            // This unwrap is safe because we derived `c` from `call_ind`
            let call_index = call_ind.unwrap();
            // First, we append the outputted block of the call
            new_block
                .extend_from_iter_transposed(c.call.rows().enumerate().map(|(row_index, r)| {
                    let sub_lead_index = (method_ref.sub_lead_index
                        + row_index.min(c.call.cover_len()))
                        % method_spec.method.lead_len();
                    AnnotRow::new(
                        r.to_owned(),
                        Annot {
                            is_lead_end: row_index == 0,
                            fold: Fold::from_sub_lead_index(sub_lead_index),
                            method: Some(MethodRef {
                                sub_lead_index,
                                ..method_ref
                            }),
                            call: Some(CallRef {
                                call_index,
                                row_index,
                            }),
                        },
                    )
                }))
                // This unwrap is safe, because we make sure that the stages of all methods are the
                // same as the stages of all the fragments
                .unwrap();
            // Next, we insert any rows that were generated by undoing the exiting call but aren't
            // replaced by this call
            if c.call.cover_len() < num_plain_rows {
                new_block
                    .extend_from_iter_transposed(
                        // Get an iterator over the annotated rows ...
                        method_spec
                            .method
                            .plain_course_iter()
                            // ... then take the region that we want ...
                            .skip(method_ref.sub_lead_index + c.call.cover_len())
                            .take(num_plain_rows - c.call.cover_len())
                            // ... and generate the correct annotations ...
                            .map(|(sub_lead_index, lead_loc, row)| {
                                Annot::row_from_course_iter(
                                    method_ref.method_index,
                                    None,
                                    sub_lead_index,
                                    lead_loc,
                                    row,
                                )
                            }), // ... then collect it into a Vec
                    )
                    // This unwrap is safe, because we make sure that the stages of all methods are
                    // the same as the stages of all the fragments
                    .unwrap();
            }
            // Finally, append the rest of the block as usual (remembering to remove some of the
            // rows from the start if `c` consumed more rows than the call we're exporting
            // generated).
            new_block
                .extend_from_iter_transposed(
                    self.block
                        .annot_rows()
                        .iter()
                        .skip(row_ind + c.call.cover_len().max(num_plain_rows))
                        .cloned(),
                )
                // This unwrap is safe, because we make sure that the stages of all Frags are the
                // same
                .unwrap();
        } else {
            /* CASE 2: WE'RE NOT APPENDING A CALL */

            // If we are replacing with a plain lead, then we only have to handle the
            // pre-replacement
            new_block
                .extend_from_iter_transposed(
                    // Get an iterator over the annotated rows ...
                    method_spec
                        .method
                        .plain_course_iter()
                        // ... then take the region that we want ...
                        .skip(method_ref.sub_lead_index)
                        .take(num_plain_rows + 1)
                        // ... and generate the correct annotations ...
                        .map(|(sub_lead_index, lead_loc, row)| {
                            Annot::row_from_course_iter(
                                method_ref.method_index,
                                None,
                                sub_lead_index,
                                lead_loc,
                                row,
                            )
                        }), // ... then collect it into a Vec
                )
                // This unwrap is safe, because we make sure that the stages of all methods are the
                // same as the stages of all the fragments
                .unwrap();
            // Now copy the remainder of the block, transposing as we go
            new_block
                .extend_from_iter_transposed(
                    self.block
                        .annot_rows()
                        .iter()
                        .skip(row_ind + consumed_rows)
                        .cloned(),
                )
                // This unwrap is safe, because we make sure that the stages of all Frags are the
                // same
                .unwrap();
        }

        // Satisfy the type checker
        Ok(self.clone_with_new_block(self.start_row.to_owned(), new_block))
    }

    /// Create a new `Frag` of `other` onto the end of `self`, transposing `other` if necessary.
    /// Both `self` and `other` will be cloned in the process.
    fn join_with(&mut self, other: &Frag) -> Result<(), IncompatibleStages> {
        Rc::make_mut(&mut self.block).extend_with_cloned(&other.block)?;
        Ok(())
    }

    /// Transposes `self` so that the `row_ind`th [`Row`] matches `target_row`
    pub fn transpose_row_to(
        &mut self,
        row_ind: usize,
        target_row: &Row,
    ) -> Result<(), IncompatibleStages> {
        // PERF: Possibly cache the results of this, since we are allocating a lot of temporary
        // values here)
        self.transpose(
            // TODO: Implement more different versions of * so that this isn't disgusting
            (target_row
                * &!(self.start_row.as_row() * self.block.get_row(row_ind).unwrap()).as_row())
                .as_row(),
        )
    }

    /// Transposes `self` - i.e. (pre)mulitplies all the [`Row`]s by some other [`Row`].  This will
    /// clone the underlying [`AnnotBlock`] the first time this is called, but every other time
    /// will not reallocate the [`AnnotBlock`].
    pub fn transpose(&mut self, transposition: &Row) -> Result<(), IncompatibleStages> {
        self.start_row = transposition.mul_result(&self.start_row)?;
        Ok(())
    }

    /* Non-mutating operations */

    /// Creates a new `Frag` which contains `self` joined to itself repeatedly until a round block
    /// is generated.  If `self` is a plain lead, then this will generate a whole course of that
    /// method.  All other properties (location, mutedness, etc.) are inherited (and cloned) from
    /// `self`.
    pub fn expand_to_round_block(&self) -> Frag {
        // PERF: This function causes way too many unnecessary allocations
        let own_start_row = self.first_row().row();
        let mut current_start_row = own_start_row.to_owned();
        let mut rows: Vec<AnnotRow<Annot>> = vec![self.block.first_annot_row().clone()];
        // Repeatedly add `self` and permute until we return to the start row
        loop {
            // Remove the leftover row from the last iteration
            rows.pop();
            // Add a copy of `self` to rows
            rows.extend(self.block.iter().map(|r| {
                let mut new_row = r.clone();
                // This unsafety is OK because we are only ever transposing by rows taken from
                // `self`, which by invariant all share the same stage
                unsafe { new_row.set_row_unchecked(current_start_row.mul_unchecked(r.row())) };
                new_row
            }));
            // Make sure that the next row starts with the last row generated so far (i.e. the
            // leftover row of the Block we've built so far)
            current_start_row = rows.last().unwrap().row().to_owned();
            // If we've reached the first row again, then return.  This must terminate because the
            // permutation group over any finite stage is always finite, so no element can have
            // infinite order.
            if own_start_row == current_start_row.as_row() {
                return self.clone_with_new_block(
                    own_start_row.to_owned(),
                    AnnotBlock::from_annot_rows(rows).unwrap(),
                );
            }
        }
    }

    /// Create a new `Frag` which is identical to `self`, except that it contains different
    /// [`Row`]s
    fn clone_with_new_block(&self, start_row: RowBuf, block: AnnotBlock<Annot>) -> Frag {
        Frag {
            start_row,
            block: Rc::new(block),
            x: self.x,
            y: self.y,
            is_muted: self.is_muted,
        }
    }

    /// Expand this `Frag` into the [`ExpandedRow`]s that make it up.  Only intended for use in
    /// [`Spec::expand`]
    fn expand(
        &self,
        part_heads: &[RowBuf],
        methods: &[Rc<MethodSpec>],
        calls: &[Rc<CallSpec>],
    ) -> Vec<ExpandedRow> {
        let mut last_method: Option<MethodRef> = None;
        let mut exp_rows: Vec<ExpandedRow> = Vec::with_capacity(self.block.len());
        for (row_ind, annot_row) in self.block.iter().enumerate() {
            /* Destruct the values from `annot_row` for convenience */
            let row = annot_row.row();
            let annot = annot_row.annot();

            /* Expand all the rows */
            let all_rows: Vec<RowBuf> = part_heads
                .iter()
                // PERF: This causes more allocations than we need, because `ExpandedRow::new` does
                // not consume the row given to it.  Even using a persistent buffer for the value
                // of `self.start_row * row` would improve things.  Even better would be to use a
                // 'RowAccum' to accumulate the values without causing reallocations.
                .map(|ph| (ph * &self.start_row).as_row() * row)
                .collect();

            /* METHOD SPLICE LOGIC:
             * This detects method splices and adds labels & ruleoffs accordingly
             */
            // A method splice should happen if this row points to a different method to the
            // last one, or the methods are the same and there's a jump in row indices
            // (ignoring wrapping over lead ends).  Note the '!' to negate the output of the
            // `match`
            let is_splice = match (last_method, annot.method) {
                (Some(lm), Some(m)) => {
                    lm.method_index != m.method_index
                            // In order to be a continuation of the same method, we have to also
                            // check that the row indices are also consecutive (so that things like
                            // restarting a method cause a splice)
                            || (lm.sub_lead_index + 1) % methods[m.method_index].method.lead_len()
                                != m.sub_lead_index
                }
                // Splicing to no method doesn't count as a splice (this will only make sense for
                // leftover rows)
                (_, cur_meth) => cur_meth.is_some(),
            };
            // Update `last_method` for the next iteration
            last_method = annot.method;
            // If there is a splice, then set the last row as a ruleoff (since ruleoffs determine
            // which rows have lines placed _underneath_ them).  Also, calculate what methd label
            // to use.
            let method_label = if is_splice {
                // Make the last row into a ruleoff (if it exists)
                if let Some(r) = exp_rows.last_mut() {
                    r.set_ruleoff()
                }
                // Return the method name to use as a label for this Row
                annot.method.map(|m| {
                    let new_method = &methods[m.method_index];
                    MethodLabel::new(new_method.name(), new_method.shorthand())
                })
            } else {
                None
            };

            /* Calculate what call string to attach to this [`Row`] */
            let call_label = annot
                .call
                // Only label the first row of a call
                .filter(|call_ref| call_ref.row_index == 0)
                // Turn the call reference into a label by first getting the `CallSpec` to which it
                // belongs, and then generating the label from that
                .map(|call_ref| {
                    calls[call_ref.call_index]
                        .as_ref()
                        .to_label(call_ref.call_index, &all_rows)
                });

            /* Construct and push an `ExpandedRow` */
            exp_rows.push(ExpandedRow::new(
                all_rows,
                call_label,
                method_label,
                annot.method,
                annot
                    .fold
                    .as_ref()
                    .map(|f| DerivedFold::new(f.is_open.get())),
                // Ruleoffs should happen at lead ends and whenever there is a splice
                annot.is_lead_end,
                // If a row is leftover or contained in a muted frag, than it shouldn't be
                // proven
                row_ind != self.len() && !self.is_muted,
                // If a row is at the last index, then it must be leftover
                row_ind == self.len(),
            ));
        }
        exp_rows
    }

    /* Constructors */

    /// Create a new `Frag` from its parts (creating [`Rc`]s where necessary)
    fn new(start_row: RowBuf, block: AnnotBlock<Annot>, x: f32, y: f32, is_muted: bool) -> Frag {
        Frag {
            start_row,
            block: Rc::new(block),
            x,
            y,
            is_muted,
        }
    }

    /// Creates a new `Frag` from a sequence of annotated [`Row`]s.
    fn from_rows(mut rows: Vec<AnnotRow<Annot>>, x: f32, y: f32, is_muted: bool) -> Frag {
        // TODO: Move this code into `core`
        // Keep the first row and its inverse
        let first_row = rows[0].row().to_owned();
        let inv_first_row = first_row.inv();
        // Transpose all the rows so that the block starts with rounds
        let mut row_buf = RowBuf::empty();
        rows.iter_mut().for_each(|annot_row| {
            row_buf.overwrite_from(annot_row.row());
            unsafe { annot_row.set_row_unchecked(inv_first_row.mul_unchecked(&row_buf)) };
        });
        Self::new(
            first_row,
            AnnotBlock::from_annot_rows(rows).unwrap(),
            x,
            y,
            is_muted,
        )
    }

    /// Generates an example fragment (in this case, it's https://complib.org/composition/75822)
    fn example() -> (Frag, Vec<Rc<MethodSpec>>, Vec<Rc<CallSpec>>) {
        const STAGE: Stage = Stage::MAJOR;
        let mut rows: Vec<AnnotRow<Annot>> = include_str!("cyclic-s8")
            .lines()
            .map(|x| AnnotRow::with_default(RowBuf::parse(x).unwrap()))
            .collect();
        let methods: Vec<Rc<MethodSpec>> = [
            ("Deva", "V", "-58-14.58-58.36-14-58-36-18,18"),
            ("Bristol", "B", "-58-14.58-58.36.14-14.58-14-18,18"),
            ("Lessness", "E", "-38-14-56-16-12-58-14-58,12"),
            ("Yorkshire", "Y", "-38-14-58-16-12-38-14-78,12"),
            ("York", "K", "-38-14-12-38.14-14.38.14-14.38,12"),
            ("Superlative", "S", "-36-14-58-36-14-58-36-78,12"),
            ("Cornwall", "W", "-56-14-56-38-14-58-14-58,18"),
        ]
        .iter()
        .map(|&(name, shorthand, pn)| {
            Rc::new(
                MethodSpec::from_pn(name.to_owned(), shorthand.to_owned(), pn.to_owned(), STAGE)
                    .unwrap(),
            )
        })
        .collect();
        let calls = vec![
            Rc::new(CallSpec::le_14_bob(STAGE)),
            Rc::new(CallSpec::le_1234_single(STAGE)),
        ];

        /* ANNOTATIONS */
        let meths = [0, 1, 2, 3, 4, 5, 6, 1];
        // Method names and LE ruleoffs
        for i in 0..rows.len() / 32 {
            for j in 0..32 {
                let a = rows[i * 32 + j].annot_mut();
                a.method = Some(MethodRef {
                    method_index: meths[i],
                    sub_lead_index: j,
                });
                a.fold = Fold::from_sub_lead_index(j);
                if let Some(f) = &a.fold {
                    f.is_open.set(i != 0);
                }
            }
            rows[i * 32 + 31].annot_mut().is_lead_end = true;
        }
        // Calls
        let single_ref = Some(CallRef {
            call_index: 1,
            row_index: 0,
        });
        rows[31].annot_mut().call = single_ref;
        rows[63].annot_mut().call = single_ref;
        rows[223].annot_mut().call = single_ref;
        rows[255].annot_mut().call = single_ref;
        // Create the fragment and return
        (Self::from_rows(rows, 0.0, 0.0, false), methods, calls)
    }
}

/* ========== FULL SPECIFICATION ========== */

/// The _specification_ for a composition, and corresponds to roughly the least information
/// required to unambiguously represent the the state of a partial composition.  This on its own is
/// not a particularly useful representation so [`DerivedState`] is used to represent an
/// 'expanded' representation of a `Spec`, which is essentially all the data that is required to
/// render a composition to the screen.
#[derive(Debug, Clone)]
pub struct Spec {
    frags: Vec<Rc<Frag>>,
    part_heads: Rc<PartHeads>,
    methods: Vec<Rc<MethodSpec>>,
    calls: Vec<Rc<CallSpec>>,
    stage: Stage,
}

impl Spec {
    /* Constructors */

    /// Creates a `Spec` with a given [`Stage`] but no [`CallSpec`]s, [`MethodSpec`]s or [`Frag`]s.
    pub fn empty(stage: Stage) -> Self {
        Spec {
            frags: Vec::new(),
            part_heads: Rc::new(PartHeads::parse("", stage).unwrap()),
            methods: Vec::new(),
            calls: Vec::new(),
            stage,
        }
    }

    /// Creates an example Spec (in this case it's a practice night touch of 7-spliced).
    pub fn example() -> Spec {
        let (frag, methods, calls) = Frag::example();
        Self::single_frag(frag, methods, calls, "", Stage::MAJOR)
    }

    fn single_frag(
        frag: Frag,
        methods: Vec<Rc<MethodSpec>>,
        calls: Vec<Rc<CallSpec>>,
        part_head_spec: &str,
        stage: Stage,
    ) -> Spec {
        // Check that all the stages match
        for annot_row in frag.block.iter() {
            assert_eq!(annot_row.row().stage(), stage);
        }
        Spec {
            frags: vec![Rc::new(frag)],
            part_heads: Rc::new(PartHeads::parse(part_head_spec, stage).unwrap()),
            methods,
            calls,
            stage,
        }
    }

    /* Operations */

    /// Overwrite the [`PartHeads`] of this `Spec`
    pub fn set_part_heads(&mut self, part_heads: PartHeads) {
        self.part_heads = Rc::new(part_heads);
    }

    /// Perform some `action` on a clone of a specific [`Frag`] in this `Spec`, forwarding the
    /// return value out of this function.  This has the effect of performing the action whilst
    /// preserving the original `Spec` (to be used in the undo history).
    pub fn make_action_frag<R>(&mut self, frag_ind: usize, action: impl Fn(&mut Frag) -> R) -> R {
        action(Rc::make_mut(&mut self.frags[frag_ind]))
    }

    /// Helper function used to create a new [`Frag`], triggered by default by the user pressing
    /// `a` (single lead) or `A` (full course).  This is used by [`Self::extend_frag`] and
    /// [`Self::add_frag`].
    fn new_frag(&self, x: f32, y: f32, add_course: bool, method_ind: usize) -> Frag {
        let new_frag = {
            let method_spec = &self.methods[method_ind];
            // TODO: Make this use `Method::course_iter` instead of needlessly creating Frags
            let mut block = AnnotBlock::from_annot_rows(
                method_spec
                    .method
                    .lead()
                    .annot_rows()
                    .iter()
                    .enumerate()
                    .map(|(i, annot_row)| {
                        AnnotRow::new(
                            annot_row.row().to_owned(),
                            Annot {
                                is_lead_end: annot_row.annot().is_some(),
                                fold: Fold::from_sub_lead_index(i),
                                method: Some(MethodRef {
                                    method_index: method_ind,
                                    sub_lead_index: i,
                                }),
                                call: None,
                            },
                        )
                    })
                    .collect(),
            )
            .unwrap();
            block.leftover_annot_mut().method = None;
            // Create new frag
            Frag::new(RowBuf::rounds(self.stage), block, x, y, false)
        };
        if add_course {
            new_frag.expand_to_round_block()
        } else {
            new_frag
        }
    }

    /// Extends the end of a [`Frag`] with more leads of some method.  For the time being, this
    /// method is always the first specified.
    pub fn extend_frag_end(&mut self, frag_ind: usize, method_ind: usize, add_course: bool) {
        // PERF: It would be much better to not generate a whole new frag, but instead to the
        // addition in-place
        let new_frag = self.new_frag(0.0, 0.0, add_course, method_ind);
        Rc::make_mut(&mut self.frags[frag_ind])
            .join_with(&new_frag)
            .unwrap();
    }

    /// Add a new [`Frag`] to the composition, returning its index.  For the time being, we always
    /// create the plain lead or course of the first specified method.  This doesn't directly do
    /// any transposing but the JS code will immediately enter transposing mode after the frag has
    /// been added, thus allowing the user to add arbitrary [`Frag`]s with minimal code
    /// duplication.
    pub fn add_frag(&mut self, x: f32, y: f32, method_ind: usize, add_course: bool) -> usize {
        self.frags
            .push(Rc::new(self.new_frag(x, y, add_course, method_ind)));
        // We always push the Frag to the end of the list, so its index is `self.frags.len()`
        self.frags.len() - 1
    }

    /// Deletes a [`Frag`]ment by index
    pub fn delete_frag(&mut self, frag_ind: usize) {
        self.frags.remove(frag_ind);
    }

    /// Join the [`Frag`] at `frag_2_ind` onto the end of the [`Frag`] at `frag_1_ind`, transposing
    /// the latter to match the former if necessary.  The combined [`Frag`] ends up at the index
    /// and location of `frag_1_ind`, and the [`Frag`] at `frag_2_ind` is removed.  All properties
    /// of the resulting [`Frag`] are inherited from the `frag_1_ind`.
    pub fn join_frags(&mut self, frag_1_ind: usize, frag_2_ind: usize) {
        assert_ne!(frag_1_ind, frag_2_ind);
        // First step, remove the 2nd frag and keep it in a temp variable.  We have to do this
        // first, because otherwise we would borrow `self.frags`s twice which the borrow checker
        // doesn't allow (because that would be a terrible bug if `frag_1_ind` == `frag_2_ind`).
        let frag_2 = self.frags.remove(frag_2_ind);
        // Because we've removed the frag at `frag_2_ind`, `self.frags[frag_1_ind]` might have
        // moved if `frag_2_ind < frag_1_ind`
        let corrected_frag_1_ind = frag_1_ind - if frag_2_ind < frag_1_ind { 1 } else { 0 };
        // Now it's safe to do the join without tripping the borrow checker
        Rc::make_mut(&mut self.frags[corrected_frag_1_ind])
            .join_with(&frag_2)
            .unwrap();
    }

    /// Split a [`Frag`] into two pieces at a given `split_index`, moving the 2nd of these to
    /// a `new_y` coordinate.
    pub fn split_frag(
        &self,
        frag_ind: usize,
        split_index: usize,
        new_y: f32,
    ) -> Result<Spec, FragSplitError> {
        // Perform the split **before** cloning `self`, short-circuiting the function if the
        // splitting fails
        let mut new_self = self.clone();
        let new_frag = Rc::make_mut(new_self.frags.get_mut(frag_ind).ok_or_else(|| {
            FragSplitError::IndexOutOfRange {
                index: frag_ind,
                num_frags: self.frags.len(),
            }
        })?)
        .split(split_index, new_y)?;
        // Replace the 1st frag in-place, and append the 2nd (this stops fragments from jumping
        // to the top of the stack when split).
        new_self.frags.push(Rc::new(new_frag));
        Ok(new_self)
    }

    pub fn set_call(
        &self,
        frag_ind: usize,
        row_ind: usize,
        call_ind: Option<usize>,
    ) -> Result<Spec, SetCallError> {
        let new_frag =
            self.frags[frag_ind].set_call(row_ind, call_ind, &self.calls, &self.methods)?;
        // If the call replacement was successful, then clone self and update the new version
        let mut new_self = self.clone();
        new_self.frags[frag_ind] = Rc::new(new_frag);
        Ok(new_self)
    }

    /// [`Frag`] soloing ala FL Studio; this has two cases:
    /// 1. `frag_ind` is the only unmuted [`Frag`], in which case we unmute everything
    /// 2. `frag_ind` isn't the only unmuted [`Frag`], in which case we mute everything except it
    pub fn solo_single_frag(&mut self, frag_ind: usize) {
        // `is_only_unmuted_frag` is true if and only if:
        //     \forall frags f: (f is unmuted) <=> (f has index `frag_ind`)
        let is_only_unmuted_frag = self
            .frags
            .iter()
            .enumerate()
            .all(|(i, f)| f.is_muted != (i == frag_ind));
        // Set state of all frags
        for (i, f) in self.frags.iter_mut().enumerate() {
            let should_be_muted = !(i == frag_ind || is_only_unmuted_frag);
            if f.is_muted != should_be_muted {
                Rc::make_mut(f).is_muted = should_be_muted;
            }
        }
    }

    /// Removes the [`MethodSpec`] at a given index.
    pub fn remove_method(&mut self, method_ind: usize) {
        self.methods.remove(method_ind);
        // Update all the method references
        for f in &mut self.frags {
            // Before cloning, check if this Frag actually contains any references that need
            // reindexing (I'm pretty sure that checking this is far far cheaper than cloning)
            if f.block
                .annots()
                .filter_map(|a| a.method)
                .all(|m_ref| m_ref.method_index < method_ind)
            {
                continue;
            }
            // If changes are required, perform those changes (whilst `Rc::make_mut`ing along the
            // way)
            for method_ref in Rc::make_mut(&mut Rc::make_mut(f).block)
                .annots_mut()
                .filter_map(|a| a.method.as_mut())
            {
                if method_ref.method_index > method_ind {
                    method_ref.method_index -= 1;
                }
            }
        }
    }

    /// Returns a cloned copy of the [`MethodSpec`] at a given index, if it exists
    pub fn get_method_spec(&self, method_ind: usize) -> Option<MethodSpec> {
        self.methods.get(method_ind).map(Rc::as_ref).cloned()
    }

    /// Updates a [`MethodSpec`] at a given index, or creates a whole new method if `index` is
    /// `None`.
    pub fn edit_method(
        &mut self,
        index: Option<usize>,
        name: String,
        shorthand: String,
        block: PnBlock,
        place_not_string: String,
    ) {
        let new_method = Method::with_lead_end(String::new(), &block);
        if let Some(i) = index {
            // If the index points to method, then replace the method's fields in-place
            let m = Rc::make_mut(&mut self.methods[i]);
            *m.name.borrow_mut() = name;
            *m.shorthand.borrow_mut() = shorthand;
            m.method = new_method;
            m.place_not_string = place_not_string;
        } else {
            // If the index doesn't point to a method, then create a new MethodSpec and push it to
            // the list
            self.methods.push(Rc::new(MethodSpec::new(
                name,
                shorthand,
                place_not_string,
                &block,
            )));
        }
    }

    /* Setters which use interior mutability */

    /// Sets the shorthand name of a given method (by index).  Panic if no method with this index
    /// exists
    pub fn set_method_shorthand(&self, method_ind: usize, new_shorthand: String) {
        *self.methods[method_ind].shorthand.borrow_mut() = new_shorthand;
    }

    /// Sets the name of a given method (by index).  Panic if no method with this index exists
    pub fn set_method_name(&self, method_ind: usize, new_name: String) {
        *self.methods[method_ind].name.borrow_mut() = new_name;
    }

    /// Sets the name of a given method (by index).  Panic if no method with this index exists
    pub fn toggle_lead_fold(&self, frag_ind: usize, row_ind: usize) {
        let annot = self.frags[frag_ind].block.get_annot(row_ind).unwrap();
        if let Some(f) = &annot.fold {
            let is_open = f.is_open.get();
            f.is_open.set(!is_open);
        }
    }

    /* Getters */

    /// Returns `true` if this `Spec` contains any rows
    #[inline]
    pub fn is_empty(&self) -> bool {
        // self.frags.is_empty() is a necessary condition for this `Spec` containing no rows,
        // because `Frag`s must have non-zero length, and `Spec`s cannot have no parts
        self.frags.is_empty()
    }

    /// The [`PartHeads`] of this `Spec`
    #[inline]
    pub fn part_heads(&self) -> &PartHeads {
        &self.part_heads
    }

    /// The number of parts that this `Spec` has
    #[inline]
    pub fn num_parts(&self) -> usize {
        self.part_heads.len()
    }

    /// Gets the number of [`Row`]s that should be proved in the expanded version of this comp,
    /// without expanding anything.
    #[inline]
    pub fn len(&self) -> usize {
        self.num_parts() * self.part_len()
    }

    /// Gets the number of [`Row`]s that are generated in one part of this composition
    pub fn part_len(&self) -> usize {
        self.frags.iter().map(|f| f.len()).sum::<usize>()
    }

    /// Returns a mutable reference to the [`Frag`] at a given index in this composition, cloning
    /// the underlying [`Frag`] if that allocation is shared between `Spec`s.
    pub fn get_frag_mut(&mut self, frag_ind: usize) -> Option<&mut Frag> {
        self.frags.get_mut(frag_ind).map(Rc::make_mut)
    }

    /// Gets the [`Stage`] of this [`Spec`]
    #[inline]
    pub fn stage(&self) -> Stage {
        self.stage
    }

    /// Returns the position of the [`Frag`] at a given index, returning `None` if that [`Frag`]
    /// doens't exist.
    pub fn frag_pos(&self, frag_ind: usize) -> Option<(f32, f32)> {
        Some(self.frags.get(frag_ind)?.pos())
    }

    /// Returns the mutedness state of the [`Frag`] at a given index, returning `None` if that
    /// [`Frag`] doesn't exist.
    pub fn is_frag_muted(&self, frag_ind: usize) -> Option<bool> {
        Some(self.frags.get(frag_ind)?.is_muted)
    }

    /// Returns the [`Cell`] containing the value of whether or not a given [`Method`]'s panel is
    /// opened or closed
    pub fn method_panel_cell(&self, method_ind: usize) -> Option<&Cell<bool>> {
        self.methods.get(method_ind).map(|m| &m.is_panel_open)
    }

    /// Returns the smallest [`Stage`] that this `Spec` could be reduced to (i.e. if there are
    /// always cover bells beyond this [`Stage`]).
    pub fn effective_stage(&self) -> Stage {
        // A totally empty `Spec` has an effective `Stage` of ZERO
        let mut min_stage = Stage::ZERO;

        // The effective stage can't be greater than any of the methods' stages
        for m in &self.methods {
            min_stage = min_stage.max(m.method.stage());
        }
        // The effective stage can't be greater than any of the calls' stages
        for c in &self.calls {
            min_stage = min_stage.max(c.call.stage());
        }
        // The effective stage can't be greater than the effective stage of any fragment
        for f in &self.frags {
            min_stage = min_stage.max(f.block.effective_stage());
        }

        min_stage
    }

    /// Generates all the rows generated by this `Spec`, storing them in the following
    /// datastructure:
    /// ```ignore
    /// (
    ///     Vec< // One per Frag
    ///         Vec< // One per row in that Frag, including the leftover row
    ///             ExpandedRow // Contains one Row per part
    ///         >,
    ///     >,
    ///     ...
    /// )
    /// ```
    /// This is only intended to by used by [`DerivedState::from_spec`].
    // The return type of this function is a big tuple with fairly complex types, but that's OK -
    // it's quite digestible and anyway is only used once as a utility function.
    #[allow(clippy::type_complexity)]
    pub fn expand(
        &self,
    ) -> (
        Vec<Vec<ExpandedRow>>,
        Rc<PartHeads>,
        &[Rc<MethodSpec>],
        &[Rc<CallSpec>],
    ) {
        let part_heads = self.part_heads.rows();
        (
            self.frags
                .iter()
                .map(|f| f.expand(part_heads, &self.methods, &self.calls))
                .collect(),
            self.part_heads.clone(),
            &self.methods,
            &self.calls,
        )
    }
}
