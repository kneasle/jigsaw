use crate::derived_state::{CallLabel, ExpandedRow, MethodLabel};
use proj_core::{
    AnnotBlock, AnnotRow, Bell, Call, IncompatibleStages, Method, PnBlock, Row, Stage,
};
use std::{
    fmt::{Display, Formatter},
    rc::Rc,
};

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
mod part_heads {
    use proj_core::{InvalidRowError, Row, Stage};
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
        #[serde(rename = "part_head_spec")]
        spec: String,
        #[serde(serialize_with = "crate::ser_utils::ser_rows")]
        part_heads: Vec<Row>,
    }

    // The invariant of always having at least one part head means that `is_empty` would always
    // return `false`
    #[allow(clippy::len_without_is_empty)]
    impl PartHeads {
        /// Returns a string slice of the specification string that generated these `PartHeads`.
        #[inline]
        pub fn spec_string(&self) -> &str {
            &self.spec
        }

        /// The number of part heads in this set of `PartHeads`.
        #[inline]
        pub fn len(&self) -> usize {
            self.part_heads.len()
        }

        /// Returns a slice over the part heads in this set of `PartHeads`
        #[inline]
        pub fn rows(&self) -> &[Row] {
            &self.part_heads
        }

        /// Given a [`str`]ing specifying some part heads, attempts to parse and expand these PHs,
        /// or generate a [`ParseError`] explaining the problem.
        pub fn parse(s: &str, stage: Stage) -> Result<Self, ParseError> {
            let part_heads = Row::parse_with_stage(s, stage)?.closure_from_rounds();
            Ok(PartHeads {
                part_heads,
                spec: String::from(s),
            })
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
    shorthand: String,
    method: Method,
}

impl MethodSpec {
    #[inline]
    pub fn name(&self) -> &str {
        self.method.name()
    }

    #[inline]
    pub fn shorthand(&self) -> &str {
        &self.shorthand
    }
}

/// The location of a [`Row`] within a method.  This is used to generate method splice text and
/// calculate ATW stats.
#[derive(Debug, Clone, Copy)]
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
    calling_positions: Vec<char>,
}

impl CallSpec {
    /// Generates the [`CallLabel`] which represents this call placed at a given [`Row`]
    fn to_label(&self, start_rows: &[Row]) -> CallLabel {
        let tenor = Bell::tenor(start_rows[0].stage()).unwrap();
        CallLabel::new(
            self.call.notation(),
            start_rows
                .iter()
                .map(|r| {
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
                    self.calling_positions[place_at_end].to_string()
                })
                .collect(),
        )
    }
}

/// The specification of where within a [`Call`] a given row comes.  This is used to generate the
/// call labels on the fly.
#[derive(Debug, Clone, Copy)]
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

/// The information that every [`Row`] in a [`Frag`] is annotated with.
#[derive(Debug, Clone, Default)]
struct Annot {
    is_lead_end: bool,
    method: Option<MethodRef>,
    call: Option<CallRef>,
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

/// A single unexpanded fragment of a composition
#[derive(Clone, Debug)]
pub struct Frag {
    start_row: Row,
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

    /* Non-mutating operations */

    /// Splits this fragment into two pieces so that the first one has length `split_index`.  Both
    /// `Frag`s will inherit all values from `self`, except that the 2nd one will have y-coordinate
    /// specified by `new_y`.
    pub fn split(&mut self, split_index: usize, new_y: f32) -> Result<Frag, FragSplitError> {
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

    /// Create a new `Frag` of `other` onto the end of `self`, transposing `other` if necessary.
    /// Both `self` and `other` will be cloned in the process.
    fn join_with(&mut self, other: &Frag) -> Result<(), IncompatibleStages> {
        Rc::make_mut(&mut self.block).extend_with_cloned(&other.block)?;
        Ok(())
    }

    /// Creates a new `Frag` which contains `self` joined to itself repeatedly until a round block
    /// is generated.  If `self` is a plain lead, then this will generate a whole course of that
    /// method.  All other properties (location, mutedness, etc.) are inherited (and cloned) from
    /// `self`.
    pub fn expand_to_round_block(&self) -> Frag {
        // PERF: This function causes way too many unnecessary allocations
        let own_start_row = self.first_row().row();
        let mut current_start_row = own_start_row.clone();
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
            current_start_row = rows.last().unwrap().row().clone();
            // If we've reached the first row again, then return.  This must terminate because the
            // permutation group over any finite stage is always finite, so no element can have
            // infinite order.
            if own_start_row == &current_start_row {
                return self.clone_with_new_block(
                    own_start_row.clone(),
                    AnnotBlock::from_annot_rows(rows).unwrap(),
                );
            }
        }
    }

    /// Transposes `self` so that the `row_ind`th [`Row`] matches `target_row`
    pub fn transpose_row_to(
        &mut self,
        row_ind: usize,
        target_row: &Row,
    ) -> Result<(), IncompatibleStages> {
        // PERF: Possibly cache the results of this, since we are allocating a lot of temporary
        // values here)
        self.transpose(&(target_row * &!(&self.start_row * self.block.get_row(row_ind).unwrap())))
    }

    /// Transposes `self` - i.e. (pre)mulitplies all the [`Row`]s by some other [`Row`].  This will
    /// clone the underlying [`AnnotBlock`] the first time this is called, but every other time
    /// will not reallocate the [`AnnotBlock`].
    pub fn transpose(&mut self, transposition: &Row) -> Result<(), IncompatibleStages> {
        self.start_row = transposition.mul(&self.start_row)?;
        Ok(())
    }

    /// Create a new `Frag` which is identical to `self`, except that it contains different
    /// [`Row`]s
    fn clone_with_new_block(&self, start_row: Row, block: AnnotBlock<Annot>) -> Frag {
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
        part_heads: &[Row],
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
            let all_rows: Vec<Row> = part_heads
                .iter()
                // PERF: This causes more allocations than we need, because `ExpandedRow::new` does
                // not consume the row given to it.  Even using a persistent buffer for the value
                // of `self.start_row * row` would improve things.  Even better would be to use a
                // 'RowAccum' to accumulate the values without causing reallocations.
                .map(|ph| &(ph * &self.start_row) * row)
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
                exp_rows
                    .last_mut()
                    .map(|r: &mut ExpandedRow| r.set_ruleoff());
                // Return the method name to use as a label for this Row
                annot.method.map(|m| {
                    let new_method = &methods[m.method_index];
                    MethodLabel::new(
                        String::from(new_method.method.name()),
                        new_method.shorthand.clone(),
                    )
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
                .map(|call_ref| calls[call_ref.call_index].as_ref().to_label(&all_rows));

            /* Construct and push an `ExpandedRow` */
            exp_rows.push(ExpandedRow::new(
                all_rows,
                call_label,
                method_label,
                annot.method,
                // Ruleoffs should happen at lead ends and whenever there is a splice
                annot.is_lead_end,
                // If a row is leftover or contained in a muted frag, than it shouldn't be
                // proven
                row_ind != self.len() && !self.is_muted,
            ));
        }
        exp_rows
    }

    /* Constructors */

    /// Create a new `Frag` from its parts (creating [`Rc`]s where necessary)
    fn new(start_row: Row, block: AnnotBlock<Annot>, x: f32, y: f32, is_muted: bool) -> Frag {
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
        let first_row = rows[0].row().clone();
        let inv_first_row = !&first_row;
        // Transpose all the rows so that the block starts with rounds
        let mut row_buf = Row::empty();
        rows.iter_mut().for_each(|annot_row| {
            row_buf.clone_from(annot_row.row());
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
    fn cyclic_s8() -> (Frag, Vec<Rc<MethodSpec>>, Vec<Rc<CallSpec>>) {
        let mut rows: Vec<AnnotRow<Annot>> = include_str!("cyclic-s8")
            .lines()
            .map(|x| AnnotRow::with_default(Row::parse(x).unwrap()))
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
        .map(|(name, shorthand, pn)| {
            Rc::new(MethodSpec {
                shorthand: String::from(*shorthand),
                method: Method::with_lead_end(
                    String::from(*name),
                    &PnBlock::parse(pn, Stage::MAJOR).unwrap(),
                ),
            })
        })
        .collect();
        let calls = vec![
            Rc::new(CallSpec {
                call: Call::le_bob(PnBlock::parse("14", Stage::MAJOR).unwrap()),
                calling_positions: "LIBFVMWH".chars().collect(),
            }),
            Rc::new(CallSpec {
                call: Call::le_single(PnBlock::parse("1234", Stage::MAJOR).unwrap()),
                calling_positions: "LBTFVMWH".chars().collect(),
            }),
        ];

        /* ANNOTATIONS */
        let meths = [0, 1, 2, 3, 4, 5, 6, 1];
        // Method names and LE ruleoffs
        for i in 0..rows.len() / 32 {
            for j in 0..32 {
                rows[i * 32 + j].annot_mut().method = Some(MethodRef {
                    method_index: meths[i],
                    sub_lead_index: j,
                });
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

    /// Creates an example Spec
    pub fn cyclic_s8() -> Spec {
        let (frag, methods, calls) = Frag::cyclic_s8();
        Self::single_frag(frag, methods, calls, "81234567", Stage::MAJOR)
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

    /// Perform some `action` on a clone of a specific [`Frag`] in this `Spec`.  This has the
    /// effect of performing the action whilst preserving the original `Spec` (to be used in the
    /// undo history).
    pub fn make_action_frag(&mut self, frag_ind: usize, action: impl Fn(&mut Frag)) {
        action(Rc::make_mut(&mut self.frags[frag_ind]));
    }

    /// Helper function used to create a new [`Frag`], triggered by default by the user pressing
    /// `a` (single lead) or `A` (full course).  This is used by [`Self::extend_frag`] and
    /// [`Self::add_frag`].
    fn new_frag(&self, x: f32, y: f32, add_course: bool, method_ind: usize) -> Frag {
        let new_frag = {
            let method_spec = &self.methods[method_ind];
            let mut block = AnnotBlock::from_annot_rows(
                method_spec
                    .method
                    .lead()
                    .annot_rows()
                    .iter()
                    .enumerate()
                    .map(|(i, annot_row)| {
                        AnnotRow::new(
                            annot_row.row().clone(),
                            Annot {
                                is_lead_end: annot_row.annot().is_some(),
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
            Frag::new(Row::rounds(self.stage), block, x, y, false)
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

    /* Getters */

    /// Returns `true` if this `Spec` contains any rows
    pub fn is_empty(&self) -> bool {
        // self.frags.is_empty() is a necessary condition for this `Spec` containing no rows,
        // because `Frag`s must have non-zero length, and `Spec`s cannot have no parts
        self.frags.is_empty()
    }

    /// The [`PartHeads`] of this `Spec`
    pub fn part_heads(&self) -> &PartHeads {
        &self.part_heads
    }

    /// The number of parts that this `Spec` has
    pub fn num_parts(&self) -> usize {
        self.part_heads.len()
    }

    /// Gets the number of [`Row`]s that should be proved in the expanded version of this comp,
    /// without expanding anything.
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

    /// Generates all the rows generated by this `Spec`, storing them in the following
    /// datastructure:
    /// ```ignore
    /// (
    ///     Vec< // One per Frag
    ///         Vec< // One per row in that Frag, including the leftover row
    ///             ExpandedRow // Contains one Row per part
    ///         >,
    ///     >,
    ///     Vec<Row>, // Part heads; one per part
    /// )
    /// ```
    pub fn expand(&self) -> (Vec<Vec<ExpandedRow>>, Rc<PartHeads>, &[Rc<MethodSpec>]) {
        let part_heads = self.part_heads.rows();
        (
            // Expanded frags
            self.frags
                .iter()
                .map(|f| f.expand(part_heads, &self.methods, &self.calls))
                .collect(),
            // Part heads
            self.part_heads.clone(),
            // Methods
            &self.methods,
        )
    }
}
