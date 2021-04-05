use crate::derived_state::{ExpandedRow, MethodName};
use proj_core::{Block, IncompatibleStages, PnBlock, Row, Stage};
use std::{
    fmt::{Display, Formatter},
    rc::Rc,
};

// Imports used solely by doc comments
#[allow(unused_imports)]
use crate::derived_state::DerivedState;

// Pretend the `PartHeads` is contained within this module, not it's own minimodule
pub use self::part_heads::PartHeads;

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

/// The specification of where within a method a given row comes.  This is used to generate
/// method splice text and calculate ATW stats.
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

#[derive(Debug, Clone)]
struct AnnotatedRow {
    is_lead_end: bool,
    method: Option<MethodRef>,
    call_str: Option<String>,
    row: Row,
}

impl AnnotatedRow {
    /// Creates an [`AnnotatedRow`] representing a given [`Row`] with no annotations
    pub fn unannotated(row: Row) -> AnnotatedRow {
        AnnotatedRow {
            is_lead_end: false,
            method: None,
            call_str: None,
            row,
        }
    }

    /// Mutates this `AnnotatedRow` so that it has no annotations.
    pub fn clear_annotations(&mut self) {
        self.method = None;
        self.call_str = None;
        self.is_lead_end = false;
    }
}

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
    /// Note that this [`Vec`] stores all the rows that should be displayed in this fragment,
    /// including the leftover row (which has to be displayed, but won't be used for proving)
    rows: Rc<Vec<AnnotatedRow>>,
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
        self.first_row().row.stage()
    }

    /// Returns the (x, y) coordinates of this `Frag`
    pub fn pos(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    /// The number of rows in this `Frag` (**not including the leftover row**)
    #[inline]
    pub fn len(&self) -> usize {
        self.rows.len() - 1
    }

    /// Returns the first [`AnnotatedRow`] of this `Frag`.  This does not return an [`Option`]
    /// because `Frag`s must have length at least 1, meaning that there is always a first
    /// [`AnnotatedRow`].
    #[inline]
    fn first_row(&self) -> &AnnotatedRow {
        &self.rows[0]
    }

    /// Returns the leftover row of this `Frag` (as an [`AnnotatedRow`]).  This does not return an
    /// [`Option`] because all `Frag`s must have a leftover row.
    #[inline]
    fn leftover_row(&self) -> &AnnotatedRow {
        self.rows.last().unwrap()
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
    pub fn split(&self, split_index: usize, new_y: f32) -> Result<(Frag, Frag), FragSplitError> {
        // Panic if splitting would create a 0-size fragment
        if split_index == 0 || split_index >= self.len() {
            return Err(FragSplitError::ZeroLengthFrag);
        }
        // Generate the rows for each subfragment
        let mut rows1 = self.rows[..split_index + 1].to_vec();
        let rows2 = self.rows[split_index..].to_vec();
        // Make sure that the leftover row of the 1st subfragment has no annotations
        rows1.last_mut().unwrap().clear_annotations();
        // Build new fragments out of the cloned rows
        Ok((
            Frag::new(rows1, self.x, self.y, self.is_muted),
            Frag::new(rows2, self.x, new_y, self.is_muted),
        ))
    }

    /// Create a new `Frag` of `other` onto the end of `self`, transposing `other` if necessary.
    /// Both `self` and `other` will be cloned in the process.
    fn joined_with(&self, other: &Frag) -> Result<Frag, IncompatibleStages> {
        IncompatibleStages::test_err(self.stage(), other.stage())?;
        // Figure out which rows we're trying to join together
        let end_row = &self.leftover_row().row;
        let start_row = &other.first_row().row;
        // Create a Vec with enough space for both Frags, and insert this Frag (minus its leftover
        // row)
        let mut rows = Vec::with_capacity(self.len() + other.len() + 1);
        rows.extend_from_slice(&self.rows[..self.len()]);
        // If the joining rows are the same then we do a simple clone, otherwise
        if end_row == start_row {
            rows.extend(other.rows.iter().cloned());
        } else {
            // All the unsafety in this block is OK because we have already asserted that the
            // stages of these `Frag`s match, and by invariant all the `Row`s must be valid
            let transposition = unsafe { end_row.mul_unchecked(&!start_row) };
            rows.extend(other.rows.iter().map(|r| {
                let mut new_row = r.clone();
                new_row.row = unsafe { transposition.mul_unchecked(&r.row) };
                new_row
            }));
        }
        Ok(self.clone_with_new_rows(rows))
    }

    /// Creates a new `Frag` which contains `self` joined to itself repeatedly until a round block
    /// is generated.  If `self` is a plain lead, then this will generate a whole course of that
    /// method.  All other properties (location, mutedness, etc.) are inherited (and cloned) from
    /// `self`.
    pub fn expand_to_round_block(&self) -> Frag {
        let own_start_row = &self.first_row().row;
        let mut current_start_row = own_start_row.clone();
        let mut rows: Vec<AnnotatedRow> = vec![self.rows[0].clone()];
        // Repeatedly add `self` and permute until we return to the start row
        loop {
            // Remove the leftover row from the last iteration
            rows.pop();
            // Add a copy of `self` to rows
            rows.extend(self.rows.iter().map(|r| {
                let mut new_row = r.clone();
                // This unsafety is OK because we are only ever transposing by rows taken from
                // `self`, which by invariant all share the same stage
                new_row.row = unsafe { current_start_row.mul_unchecked(&r.row) };
                new_row
            }));
            // Make sure that the next row starts with the last row generated so far (i.e. the
            // leftover row of the Block we've built so far)
            current_start_row = rows.last().unwrap().row.clone();
            // If we've reached the first row again, then return.  This must terminate because the
            // permutation group over any finite stage is always finite, so no element can have
            // infinite order.
            if own_start_row == &current_start_row {
                return self.clone_with_new_rows(rows);
            }
        }
    }

    /// Transposes `self` so that the `row_ind`th [`Row`] matches `target_row`
    pub fn transpose_row_to(
        &mut self,
        row_ind: usize,
        target_row: &Row,
    ) -> Result<(), IncompatibleStages> {
        self.transpose(&(target_row * &!&self.rows[row_ind].row))
    }

    /// Transposes `self` - i.e. (pre)mulitplies all the [`Row`]s by some other [`Row`].
    pub fn transpose(&mut self, transposition: &Row) -> Result<(), IncompatibleStages> {
        // Do the stage check once, rather than every time a row gets permuted
        IncompatibleStages::test_err(transposition.stage(), self.stage())?;
        let mut row_buf = Row::empty();
        for r in Rc::make_mut(&mut self.rows) {
            row_buf.clone_from(&r.row);
            // The unsafety here is OK because we maintain an invariant that all the `Row`s
            // in this `Frag` have the same `Stage`
            unsafe { transposition.mul_into_unchecked(&row_buf, &mut r.row) };
        }
        Ok(())
    }

    /// Create a new `Frag` which is identical to `self`, except that it contains different
    /// [`AnnotatedRow`]s
    fn clone_with_new_rows(&self, rows: Vec<AnnotatedRow>) -> Frag {
        Frag {
            rows: Rc::new(rows),
            x: self.x,
            y: self.y,
            is_muted: self.is_muted,
        }
    }

    /// Expand this `Frag` into the [`ExpandedRow`]s that make it up.  Only intended for use in
    /// [`Spec::expand`]
    fn expand(&self, part_heads: &[Row], methods: &[Rc<Method>]) -> Vec<ExpandedRow> {
        let mut last_method: Option<MethodRef> = None;
        let mut exp_rows: Vec<ExpandedRow> = Vec::with_capacity(self.rows.len());
        for (row_ind, r) in self.rows.iter().enumerate() {
            // A method splice should happen if this row points to a different method to the
            // last one, or the methods are the same and there's a jump in row indices
            // (ignoring wrapping over lead ends).  Note the '!' to negate the output of the
            // `match`
            let is_splice = !match (last_method, r.method) {
                (Some(lm), Some(m)) => {
                    lm.method_index == m.method_index
                            // In order to be a continuation of the same method, we have to also
                            // check that the row indices are also consecutive (so that things like
                            // restarting a method cause a splice)
                            && (lm.sub_lead_index + 1) % methods[m.method_index].first_lead.len()
                                == m.sub_lead_index
                }
                // Splicing to no method doesn't count as a splice (this will only make sense for
                // leftover rows)
                (_, cur_meth) => cur_meth.is_none(),
            };
            last_method = r.method;
            // If there is a splice, then set the last row as a ruleoff (since ruleoffs determine
            // which rows have lines placed _underneath_ them)
            if is_splice {
                exp_rows
                    .last_mut()
                    .map(|r: &mut ExpandedRow| r.set_ruleoff());
            }
            // Push the new ExpandedRow
            exp_rows.push(ExpandedRow::new(
                &r.row,
                r.call_str.clone(),
                r.method
                    .map(|m| {
                        let new_method = &methods[m.method_index];
                        MethodName::new(new_method.name.clone(), new_method.shorthand.clone())
                    })
                    .filter(|_| is_splice),
                r.method,
                // Ruleoffs should happen at lead ends and whenever there is a splice
                r.is_lead_end,
                part_heads,
                // If a row is leftover or contained in a muted frag, than it shouldn't be
                // proven
                row_ind != self.len() && !self.is_muted,
            ));
        }
        exp_rows
    }

    /* Constructors */

    /// Create a new `Frag` from its parts (creating [`Rc`]s where necessary)
    fn new(rows: Vec<AnnotatedRow>, x: f32, y: f32, is_muted: bool) -> Frag {
        Frag {
            rows: Rc::new(rows),
            x,
            y,
            is_muted,
        }
    }

    fn from_rows(rows: Vec<AnnotatedRow>) -> Frag {
        Self::new(rows, 0.0, 0.0, false)
    }

    /// Generates an example fragment (in this case, it's https://complib.org/composition/75822)
    fn cyclic_s8() -> (Frag, Vec<Rc<Method>>) {
        let mut rows: Vec<_> = include_str!("cyclic-s8")
            .lines()
            .map(|x| Row::parse(x).unwrap())
            .map(AnnotatedRow::unannotated)
            .collect();
        let methods: Vec<Rc<Method>> = [
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
            Rc::new(Method {
                name: String::from(*name),
                shorthand: String::from(*shorthand),
                first_lead: PnBlock::parse(pn, Stage::MAJOR)
                    .unwrap()
                    .block_starting_with(&Row::rounds(Stage::MAJOR))
                    .unwrap(),
            })
        })
        .collect();

        /* ANNOTATIONS */
        let meths = [0, 1, 2, 3, 4, 5, 6, 1];
        // Method names and LE ruleoffs
        for i in 0..rows.len() / 32 {
            for j in 0..32 {
                rows[i * 32 + j].method = Some(MethodRef {
                    method_index: meths[i],
                    sub_lead_index: j,
                });
            }
            rows[i * 32 + 31].is_lead_end = true;
        }
        // Calls
        rows[31].call_str = Some("sB".to_owned());
        rows[63].call_str = Some("sB".to_owned());
        rows[223].call_str = Some("sH".to_owned());
        rows[255].call_str = Some("sH".to_owned());
        // Create the fragment and return
        (Self::from_rows(rows), methods)
    }
}

#[derive(Debug, Clone)]
pub struct Method {
    name: String,
    shorthand: String,
    first_lead: Block,
}

impl Method {
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn shorthand(&self) -> &str {
        &self.shorthand
    }
}

/// The _specification_ for a composition, and corresponds to roughly the least information
/// required to unambiguously represent the the state of a partial composition.  This on its own is
/// not a particularly useful representation so [`DerivedState`] is used to represent an
/// 'expanded' representation of a `Spec`, which is essentially all the data that is required to
/// render a composition to the screen.
#[derive(Debug, Clone)]
pub struct Spec {
    frags: Vec<Rc<Frag>>,
    part_heads: Rc<PartHeads>,
    methods: Vec<Rc<Method>>,
    stage: Stage,
}

impl Spec {
    /* Constructors */

    /// Creates an example Spec
    pub fn cyclic_s8() -> Spec {
        let (frag, methods) = Frag::cyclic_s8();
        Self::single_frag(frag, methods, "81234567", Stage::MAJOR)
    }

    fn single_frag(
        frag: Frag,
        methods: Vec<Rc<Method>>,
        part_head_spec: &str,
        stage: Stage,
    ) -> Spec {
        // Check that all the stages match
        for annot_r in frag.rows.iter() {
            assert_eq!(annot_r.row.stage(), stage);
        }
        Spec {
            frags: vec![Rc::new(frag)],
            methods,
            part_heads: Rc::new(PartHeads::parse(part_head_spec, stage).unwrap()),
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
            // Generate the rows
            let mut rows: Vec<AnnotatedRow> = self.methods[method_ind]
                .first_lead
                .rows()
                .cloned()
                .map(AnnotatedRow::unannotated)
                .collect();
            // Annotate the rows with method indices
            for (i, r) in rows.iter_mut().enumerate() {
                r.method = Some(MethodRef {
                    method_index: method_ind,
                    sub_lead_index: i,
                });
            }
            // Handle the last row separately
            let row_len = rows.len();
            rows[row_len - 2].is_lead_end = true;
            rows.last_mut().unwrap().method = None;
            // Create new frag
            Frag::new(rows, x, y, false)
        };
        if add_course {
            new_frag.expand_to_round_block()
        } else {
            new_frag
        }
    }

    /// Extends the end of a [`Frag`] with more leads of some method.  For the time being, this
    /// method is always the first specified.
    pub fn extend_frag(&mut self, frag_ind: usize, add_course: bool) {
        let method_ind = 0usize;
        // TODO: We can get away with **many** fewer allocations than this
        let extend_frag = self.frags[frag_ind]
            .joined_with(&self.new_frag(0.0, 0.0, add_course, method_ind))
            .unwrap();
        self.frags[frag_ind] = Rc::new(extend_frag);
    }

    /// Add a new [`Frag`] to the composition, returning its index.  For the time being, we always
    /// create the plain lead or course of the first specified method.  This doesn't directly do
    /// any transposing but the JS code will immediately enter transposing mode after the frag has
    /// been added, thus allowing the user to add arbitrary [`Frag`]s with minimal code
    /// duplication.
    pub fn add_frag(&mut self, x: f32, y: f32, add_course: bool) -> usize {
        // For the time being always add method #0
        let method_ind = 0usize;
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
        let joined_frag = self.frags[frag_1_ind]
            .joined_with(&self.frags[frag_2_ind])
            .unwrap();
        self.frags[frag_1_ind] = Rc::new(joined_frag);
        self.frags.remove(frag_2_ind);
    }

    /// Split a [`Frag`] into two pieces at a given `split_index`, moving the 2nd of these to
    /// a `new_y` coordinate.
    pub fn split_frag(
        &self,
        frag_ind: usize,
        split_index: usize,
        new_y: f32,
    ) -> Result<Self, FragSplitError> {
        // Perform the split **before** cloning `self`, short-circuiting the function if the
        // splitting fails
        let (f1, f2) = self
            .frags
            .get(frag_ind)
            .ok_or_else(|| FragSplitError::IndexOutOfRange {
                index: frag_ind,
                num_frags: self.frags.len(),
            })?
            .split(split_index, new_y)?;
        // Replace the 1st frag in-place, and append the 2nd (this stops fragments from jumping
        // to the top of the stack when split).
        let mut new_self = self.clone();
        new_self.frags[frag_ind] = Rc::new(f1);
        new_self.frags.push(Rc::new(f2));
        // Return empty string for success
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
    pub fn expand(&self) -> (Vec<Vec<ExpandedRow>>, Rc<PartHeads>, &[Rc<Method>]) {
        let part_heads = self.part_heads.rows();
        (
            // Expanded frags
            self.frags
                .iter()
                .map(|f| f.expand(part_heads, &self.methods))
                .collect(),
            // Part heads
            self.part_heads.clone(),
            // Methods
            &self.methods,
        )
    }
}
