use crate::derived_state::ExpandedRow;
use proj_core::{IncompatibleStages, Row, Stage};
use serde::Serialize;
use std::{
    fmt::{Display, Formatter},
    rc::Rc,
};

// Imports used solely by doc comments
#[allow(unused_imports)]
use crate::derived_state::DerivedState;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct AnnotatedRow {
    is_lead_end: bool,
    method_str: Option<MethodName>,
    call_str: Option<String>,
    row: Row,
}

impl AnnotatedRow {
    /// Creates an [`AnnotatedRow`] representing a given [`Row`] with no annotations
    pub fn unannotated(row: Row) -> AnnotatedRow {
        AnnotatedRow {
            is_lead_end: false,
            method_str: None,
            call_str: None,
            row,
        }
    }

    /// Mutates this `AnnotatedRow` so that it has no annotations.
    pub fn clear_annotations(&mut self) {
        self.method_str = None;
        self.call_str = None;
        self.is_lead_end = false;
    }
}

/// A convenient data structure of the long and short method names
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize)]
pub struct MethodName {
    name: String,
    shorthand: String,
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
    pub fn joined_with(&self, other: &Frag) -> Frag {
        // Figure out which rows we're trying to join together
        let end_row = &self.leftover_row().row;
        let start_row = &other.first_row().row;
        // Create a Vec with enough space for both Frags, and insert this Frag (minus its leftover
        // row)
        let mut rows = Vec::with_capacity(self.len() + other.len() + 1);
        rows.extend(self.rows[..self.len()].iter().cloned());
        // If the joining rows are the same then we do a simple clone, otherwise
        if end_row == start_row {
            rows.extend(other.rows.iter().cloned());
        } else {
            let transposition = end_row.mul_unchecked(&!start_row);
            rows.extend(other.rows.iter().map(|r| {
                let mut new_row = r.clone();
                new_row.row = transposition.mul_unchecked(&r.row);
                new_row
            }));
        }
        self.clone_with_new_rows(rows)
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
            // Add a copy of `self` to rows (skipping the first row, since that will be provided as
            // the leftover row of the last `Frag` we added)
            rows.extend(self.rows[1..].iter().map(|r| {
                let mut new_row = r.clone();
                new_row.row = current_start_row.mul_unchecked(&r.row);
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

    /// Returns a transposed copy of `self`, where the `row_ind`th [`Row`] is a given [`Row`]
    pub fn transpose_row_to(
        &self,
        row_ind: usize,
        target_row: &Row,
    ) -> Result<Frag, IncompatibleStages> {
        self.transposed(&(target_row * &!&self.rows[row_ind].row))
    }

    /// Returns a copy of `self` in which all the rows are (pre)mulitplied by some other [`Row`].
    pub fn transposed(&self, transposition: &Row) -> Result<Frag, IncompatibleStages> {
        // Do the stage check once, rather than every time a row gets permuted
        IncompatibleStages::test_err(transposition.stage(), self.stage())?;
        Ok(self.clone_with_new_rows(
            self.rows
                .iter()
                .map(|r| {
                    let mut new_row = r.clone();
                    new_row.row = transposition.mul_unchecked(&r.row);
                    new_row
                })
                .collect(),
        ))
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
    pub fn cyclic_s8() -> Frag {
        let mut rows: Vec<_> = include_str!("cyclic-s8")
            .lines()
            .map(|x| Row::parse(x).unwrap())
            .map(AnnotatedRow::unannotated)
            .collect();
        /* ANNOTATIONS */
        // Method names and LE ruleoffs
        let method_names = [
            ("Deva", "V"),
            ("Bristol", "B"),
            ("Lessness", "E"),
            ("Yorkshire", "Y"),
            ("York", "K"),
            ("Superlative", "S"),
            ("Cornwall", "W"),
            ("Bristol", "B"),
        ];
        for i in 0..rows.len() / 32 {
            let (method_name, method_short) = method_names[i];
            rows[i * 32].method_str = Some(MethodName {
                name: method_name.to_owned(),
                shorthand: method_short.to_owned(),
            });
            rows[i * 32 + 31].is_lead_end = true;
        }
        // Calls
        rows[31].call_str = Some("sB".to_owned());
        rows[63].call_str = Some("sB".to_owned());
        rows[223].call_str = Some("sH".to_owned());
        rows[255].call_str = Some("sH".to_owned());
        // Create the fragment and return
        Self::from_rows(rows)
    }

    pub fn cyclic_max_eld() -> Frag {
        // Parse row and method locations
        let mut rows: Vec<_> = include_str!("cyclic-max-eld")
            .lines()
            .map(|x| Row::parse(x).unwrap())
            .map(AnnotatedRow::unannotated)
            .collect();

        // Add method names & ruleoffs to the appropriate rows
        let methods = [
            ("Mount Mackenzie Alliance", "Mm", 36),
            ("Baluan Alliance", "B", 36),
            ("Ganges Alliance", "Ga", 40),
            ("Cauldron Dome Little Delight", "Ca", 32),
            ("Europa Little Treble Place", "Eu", 16),
            ("Diamond Head Alliance", "D2", 44),
            ("Callisto Little Alliance", "Ca", 36),
            ("Darwin Little Alliance", "D", 44),
            ("Hallasan Alliance", "Ha", 32),
            ("Asaph Hall Surprise", "As", 48),
            ("Alcedo Alliance", "A", 40),
            ("Kilauea Differential", "Li", 14),
        ];
        let mut method_start = 0;
        for (full, short, length) in &methods {
            rows[method_start].method_str = Some(MethodName {
                name: full.to_string(),
                shorthand: short.to_string(),
            });
            rows[method_start + length - 1].is_lead_end = true;
            method_start += length;
        }
        assert_eq!(method_start + 1, rows.len());

        // Add calls
        rows[403].call_str = Some("-".to_owned());
        rows[417].call_str = Some("-".to_owned());

        Self::from_rows(rows)
    }

    /// Returns a `Frag` of the first plain lead of Plain Bob Major
    pub fn one_lead_pb_maj(x: f32, y: f32) -> Frag {
        let mut rows: Vec<_> = include_str!("pb-8")
            .lines()
            .map(|x| Row::parse(x).unwrap())
            .map(AnnotatedRow::unannotated)
            .collect();

        rows[0].method_str = Some(MethodName {
            name: "Plain Bob".to_owned(),
            shorthand: "P".to_owned(),
        });
        rows[15].is_lead_end = true;

        Self::new(rows, x, y, false)
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
    part_heads: Rc<Vec<Row>>,
    stage: Stage,
}

impl Spec {
    /* Constructors */

    /// Creates an example Spec
    pub fn cyclic_s8() -> Spec {
        // Generate all the cyclic part heads, and make sure that we start with rounds
        let mut part_heads = Row::parse("81234567").unwrap().closure();
        let rounds = part_heads.pop().unwrap();
        part_heads.insert(0, rounds);
        // Create a Spec and return
        Self::single_frag(Frag::cyclic_s8(), part_heads, Stage::MAJOR)
    }

    pub fn cyclic_max_eld() -> Spec {
        // Generate all the cyclic part heads, and make sure that we start with rounds
        let mut part_heads = Row::parse("890ET1234567").unwrap().closure();
        let rounds = part_heads.pop().unwrap();
        part_heads.insert(0, rounds);
        // Create a Spec and return
        Self::single_frag(Frag::cyclic_max_eld(), part_heads, Stage::MAXIMUS)
    }

    fn single_frag(frag: Frag, part_heads: Vec<Row>, stage: Stage) -> Spec {
        // Check that all the stages match
        for annot_r in frag.rows.iter() {
            assert_eq!(annot_r.row.stage(), stage);
        }
        for p in &part_heads {
            assert_eq!(p.stage(), stage);
        }
        Spec {
            frags: vec![Rc::new(frag)],
            part_heads: Rc::new(part_heads),
            stage,
        }
    }

    /* Operations */

    /// Perform some `action` on a clone of a specific [`Frag`] in this `Spec`.  This has the
    /// effect of performing the action whilst preserving the original `Spec` (to be used in the
    /// undo history).
    pub fn make_action_frag(&mut self, frag_ind: usize, action: impl Fn(&mut Frag)) {
        let mut new_frag = self.frags[frag_ind].as_ref().clone();
        action(&mut new_frag);
        self.frags[frag_ind] = Rc::new(new_frag);
    }

    /// Add a new [`Frag`] to the composition, returning its index.  For the time being, we always
    /// create the plain lead or course of Plain Bob Major.  This doesn't directly do any
    /// transposing but the JS code will immediately enter transposing mode after the frag has been
    /// added, thus allowing the user to add arbitrary [`Frag`]s with minimal code duplication.
    pub fn add_frag(&mut self, x: f32, y: f32, add_course: bool) -> usize {
        let new_frag = Frag::one_lead_pb_maj(x, y);
        self.frags.push(Rc::new(if add_course {
            new_frag.expand_to_round_block()
        } else {
            new_frag
        }));
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
        let joined_frag = self.frags[frag_1_ind].joined_with(&self.frags[frag_2_ind]);
        self.frags[frag_1_ind] = Rc::new(joined_frag);
        self.frags.remove(frag_2_ind);
    }

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
                let mut new_frag = f.as_ref().clone();
                new_frag.is_muted = should_be_muted;
                *f = Rc::new(new_frag);
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

    /// Gets the number of [`Row`]s that should be proved in the expanded version of this comp,
    /// without expanding anything.
    pub fn len(&self) -> usize {
        self.part_heads.len() * self.part_len()
    }

    /// Gets the number of [`Row`]s that are generated in one part of this composition
    pub fn part_len(&self) -> usize {
        self.frags.iter().map(|f| f.len()).sum::<usize>()
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
    pub fn expand(&self) -> (Vec<Vec<ExpandedRow>>, Vec<Row>) {
        let part_heads = self.part_heads.as_ref().clone();
        let expanded_rows = self
            .frags
            .iter()
            .enumerate()
            .map(|(frag_ind, f)| {
                f.rows
                    .iter()
                    .enumerate()
                    .map(|(row_ind, r)| {
                        ExpandedRow::new(
                            &r.row,
                            r.call_str.clone(),
                            r.method_str.clone(),
                            r.is_lead_end,
                            &part_heads,
                            // Figure out if this frag should be proved
                            row_ind != f.len() && !self.frags[frag_ind].is_muted,
                        )
                    })
                    .collect()
            })
            .collect();

        (expanded_rows, part_heads)
    }
}
