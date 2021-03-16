use crate::derived_state::ExpandedRow;
use proj_core::{IncompatibleStages, Row, Stage};
use serde::Serialize;
use std::rc::Rc;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AnnotatedRow {
    pub(crate) is_lead_end: bool,
    pub(crate) method_str: Option<MethodName>,
    pub(crate) call_str: Option<String>,
    pub(crate) row: Row,
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

impl Frag {
    /* Getters */

    /// Returns the [`Stage`] of this `Frag`
    pub fn stage(&self) -> Stage {
        self.first_row().row.stage()
    }

    /// Returns the (x, y) coordinates of this `Frag`ment
    pub fn pos(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    /// Returns the mutedness state of this `Frag`
    pub fn is_muted(&self) -> bool {
        self.is_muted
    }

    /// The number of rows in this `Frag` (**not including the leftover row**)
    #[inline]
    pub fn len(&self) -> usize {
        self.rows.len() - 1
    }

    /// Gets the [`AnnotatedRow`] of this [`Frag`] at a given index
    #[inline]
    pub fn get_annot_row(&self, index: usize) -> &AnnotatedRow {
        &self.rows[index]
    }

    /// Returns the first [`AnnotatedRow`] of this `Frag`.  This does not return an [`Option`]
    /// because `Frag`s must have length at least 1, meaning that there is always a first
    /// [`AnnotatedRow`].
    #[inline]
    pub fn first_row(&self) -> &AnnotatedRow {
        &self.rows[0]
    }

    /// Returns the leftover row of this `Frag` (as an [`AnnotatedRow`]).  This does not return an
    /// [`Option`] because all `Frag`s must have a leftover row.
    #[inline]
    pub fn leftover_row(&self) -> &AnnotatedRow {
        self.rows.last().unwrap()
    }

    /* Setters/mutating operations */

    /// Updates the coordinates of this `Frag` to match the new ones
    pub fn move_to(&mut self, new_x: f32, new_y: f32) {
        self.x = new_x;
        self.y = new_y;
    }

    /// Toggles whether or not this [`Frag`] is in state [`FragState::Muted`].
    pub fn toggle_mute(&mut self) {
        self.is_muted = !self.is_muted;
    }

    /* Non-mutating operations */

    /// Splits this fragment into two pieces so that the first one has length `split_index`.  Both
    /// `Frag`s will have the same x-coordinate, but the 2nd one will have y-coordinate specified
    /// by `new_y`.  This panics if `split_index` is out of range of the number of rows.
    pub fn split(&self, split_index: usize, new_y: f32) -> (Frag, Frag) {
        // Panic if splitting would create a 0-size fragment
        if split_index == 0 || split_index >= self.len() {
            panic!(
                "Splitting at index {} would create a 0-length Fragment",
                split_index
            );
        }
        // Generate the rows for each subfragment
        let mut rows1: Vec<_> = self.rows[..split_index + 1].iter().cloned().collect();
        let rows2: Vec<_> = self.rows[split_index..].iter().cloned().collect();
        // Make sure that the leftover row of the 1st subfragment has no annotations
        rows1.last_mut().unwrap().clear_annotations();
        // Build new fragments out of the cloned rows
        (
            Frag::new(rows1, self.x, self.y, self.is_muted),
            Frag::new(rows2, self.x, new_y, self.is_muted),
        )
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
    pub fn clone_with_new_rows(&self, rows: Vec<AnnotatedRow>) -> Frag {
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

/// The _specfication_ for a composition.  This is what the user edits, and it is used to derive
/// the fully expanded set of rows and their origins.
#[derive(Debug, Clone)]
pub struct Spec {
    pub(crate) frags: Vec<Rc<Frag>>,
    pub(crate) part_heads: Rc<Vec<Row>>,
    pub(crate) stage: Stage,
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

    /// [`Frag`] soloing ala FL Studio; this has two cases:
    /// 1. `frag_ind` is the only unmuted [`Frag`], in which case we unmute everything
    /// 2. `frag_ind` isn't the only unmuted [`Frag`], in which case we mute everything except it
    pub fn solo_single_frag(&mut self, frag_ind: usize) {
        // `is_only_unmuted_frag` is true if:
        //     \forall frags f: (f is unmuted) <=> (f has index `frag_ind`)
        let is_only_unmuted_frag = self
            .frags
            .iter()
            .enumerate()
            .all(|(i, f)| !f.is_muted == (i == frag_ind));
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

    /// Gets the number of [`Row`]s that should be proved in the expanded version of this comp,
    /// without expanding anything.
    pub fn len(&self) -> usize {
        self.part_heads.len() * self.part_len()
    }

    /// Gets the number of [`Row`]s that are generated in one part of this composition
    pub fn part_len(&self) -> usize {
        self.frags.iter().map(|f| f.len()).sum::<usize>()
    }

    /// Generates all the rows generated by this `Spec`, storing them in the following
    /// datastructure:
    /// ```ignore
    /// Vec< // One per Frag
    ///     (
    ///         Vec< // One per row in that Frag, including the leftover row
    ///             ExpandedRow // Contains one Row per part
    ///         >,
    ///         bool // Is the entire [`Frag`] proved?
    ///     )
    /// >,
    /// ```
    pub fn gen_rows(&self) -> Vec<Vec<ExpandedRow>> {
        self.frags
            .iter()
            .enumerate()
            .map(|(frag_ind, f)| {
                // Figure out if this frag should be proved
                f.rows
                    .iter()
                    .enumerate()
                    .map(|(row_ind, r)| {
                        ExpandedRow::new(
                            r,
                            &self.part_heads,
                            row_ind != f.len() && !self.frags[frag_ind].is_muted,
                        )
                    })
                    .collect()
            })
            .collect()
    }
}
