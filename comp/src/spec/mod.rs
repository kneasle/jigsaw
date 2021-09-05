pub mod part_heads;

use std::{
    cell::{Cell, Ref, RefCell},
    collections::HashSet,
    convert::{TryFrom, TryInto},
    ops::Deref,
    rc::Rc,
};

use bellframe::{
    music::Regex, row::RowAccumulator, AnnotBlock, IncompatibleStages, Row, RowBuf, Stage,
};
use emath::Pos2;
use index_vec::index_vec;
use jigsaw_utils::indexed_vec::{
    ChunkIdx, ChunkVec, FragIdx, FragVec, MethodSlice, MethodVec, RowIdx, RowVec,
};

use crate::{
    expanded_frag::{ExpandedFrag, RowData},
    Music,
};

use self::part_heads::PartHeads;

/// The minimal but complete specification for a (partial) composition.  `CompSpec` is used for
/// undo history, and is designed to be a very compact representation which is cheap to clone and
/// modify.  Contrast this with [`FullState`](crate::full::FullState), which is computed from
/// `CompSpec` and is designed to be efficient to query and display to the user (and so contains a
/// large amount of redundant information).
// PERF: Maybe wrap the `Vec`s in `Rc`s
#[derive(Debug, Clone)]
pub struct CompSpec {
    fragments: FragVec<Rc<Fragment>>,
    part_heads: Rc<PartHeads>,
    methods: MethodVec<Rc<Method>>,
    calls: Vec<Rc<Call>>,
    // TODO: Make this structure use `Rc`s internally
    music: Rc<Vec<Music>>,
    stage: Stage,
}

// This `impl` block is the entire public surface of `CompSpec`
impl CompSpec {
    //////////////////
    // CONSTRUCTORS //
    //////////////////

    /// Creates a [`CompSpec`] with a given [`Stage`] but no [`PartHeads`], [`Method`]s, [`Call`]s
    /// or [`Fragment`]s.
    #[allow(dead_code)]
    pub fn empty(stage: Stage) -> Self {
        CompSpec {
            fragments: index_vec![],
            part_heads: Rc::new(PartHeads::one_part(stage)),
            methods: index_vec![],
            calls: vec![],
            music: Rc::new(vec![]),
            stage,
        }
    }

    /// Generates an example composition.
    pub fn example() -> Self {
        const STAGE: Stage = Stage::MAJOR;

        /// Create a new [`Method`] by parsing a string of place notation
        fn gen_method(shorthand: &str, name: &str, pn_str: &str) -> Rc<Method> {
            let method = Method::with_lead_end_ruleoff(
                bellframe::Method::from_place_not_string(String::new(), STAGE, pn_str).unwrap(),
                name.to_owned(),
                shorthand.to_string(),
            );
            Rc::new(method)
        }

        // The methods used in the composition
        let methods = index_vec![
            /* 0. */ gen_method("D", "Deva", "-58-14.58-58.36-14-58-36-18,18"),
            /* 1. */ gen_method("B", "Bristol", "-58-14.58-58.36.14-14.58-14-18,18"),
            /* 2. */ gen_method("E", "Lessness", "-38-14-56-16-12-58-14-58,12"),
            /* 3. */ gen_method("Y", "Yorkshire", "-38-14-58-16-12-38-14-78,12"),
            /* 4. */ gen_method("K", "York", "-38-14-12-38.14-14.38.14-14.38,12"),
            /* 5. */ gen_method("S", "Superlative", "-36-14-58-36-14-58-36-78,12"),
            /* 6. */ gen_method("W", "Cornwall", "-56-14-56-38-14-58-14-58,18"),
        ];

        // Touch is Deva, Yorkshire, York, Superlative, Lessness
        let chunks = [0usize, 3, 4, 5, 2]
            .iter()
            .map(|method_idx| {
                let method = methods[*method_idx].clone();
                let lead_len = method.inner.lead_len();
                // Add an entire lead of each method
                Rc::new(Chunk::method(method, 0, lead_len))
            })
            .collect::<ChunkVec<_>>();

        let fragment = Fragment {
            position: Pos2::new(200.0, 100.0),
            start_row: Rc::new(RowBuf::rounds(STAGE)),
            chunks,
            is_proved: true,
        };

        let music = Rc::new(vec![
            Music::Group(
                "56s/65s".to_owned(),
                vec![
                    Music::Regex(Some("65s".to_owned()), Regex::parse("*6578")),
                    Music::Regex(Some("56s".to_owned()), Regex::parse("*5678")),
                ],
            ),
            Music::runs_front_and_back(Stage::MAJOR, 4),
            Music::runs_front_and_back(Stage::MAJOR, 5),
            Music::runs_front_and_back(Stage::MAJOR, 6),
            Music::runs_front_and_back(Stage::MAJOR, 7),
            Music::Regex(Some("Queens".to_owned()), Regex::parse("13572468")),
            Music::Regex(Some("Backrounds".to_owned()), Regex::parse("87654321")),
        ]);

        CompSpec {
            fragments: index_vec![Rc::new(fragment)],
            part_heads: Rc::new(
                PartHeads::parse("18234567", STAGE).unwrap(), /* PartHeads::one_part(STAGE) */
            ),
            methods,
            calls: vec![], // No calls for now
            music,
            stage: STAGE,
        }
    }

    ////////////////////////////
    // GETTERS/EXPANSION CODE //
    ////////////////////////////

    pub(crate) fn expand_fragments(&self) -> FragVec<ExpandedFrag> {
        self.fragments
            .iter()
            .map(|f| f.expand(&self.part_heads))
            .collect()
    }

    pub(crate) fn part_heads(&self) -> &Rc<PartHeads> {
        &self.part_heads
    }

    pub(crate) fn methods(&self) -> &MethodSlice<Rc<Method>> {
        &self.methods
    }

    pub(crate) fn music(&self) -> &[Music] {
        &self.music
    }

    pub(crate) fn stage(&self) -> Stage {
        self.stage
    }

    /////////////////////////
    // MODIFIERS & ACTIONS //
    /////////////////////////

    // All modifiers and actions will create steps in the undo history

    /// Overwrites the [`PartHeads`] of `self`.
    ///
    /// # Panics
    ///
    /// Panics if the [`Stage`]s of `self` and the new [`PartHeads`] don't match
    pub fn set_part_heads(&mut self, part_heads: PartHeads) {
        assert_eq!(self.stage, part_heads.stage());
        self.part_heads = Rc::new(part_heads);
    }

    /// Solo a single [`Fragment`], or unmute everything if this is the only unmuted [`Fragment`].
    pub fn solo_frag(&mut self, frag_idx: FragIdx) -> Result<(), EditError> {
        /// Helper function to set `f.is_proved`, without cloning any fragments which don't need to
        /// be changed
        fn set_frag_proved(f: &mut Rc<Fragment>, is_proved: bool) {
            if f.is_proved != is_proved {
                Rc::make_mut(f).is_proved = is_proved;
            }
        }

        // Abort with error if `frag_idx` is out-of-bounds
        self.get_fragment(frag_idx)?;

        let is_the_only_unmuted_frag = self
            .fragments
            .iter_enumerated()
            // `true` when all fragments are proved if and only if they have index `idx`
            .all(|(idx, frag)| (idx == frag_idx) == (frag.is_proved));
        if is_the_only_unmuted_frag {
            // Unmute all fragments
            for frag in self.fragments.iter_mut() {
                set_frag_proved(frag, true);
            }
        } else {
            for (idx, frag) in self.fragments.iter_mut_enumerated() {
                set_frag_proved(frag, idx == frag_idx);
            }
        }
        Ok(())
    }

    /// Deletes the [`Fragment`] with a given [`FragIdx`]
    pub fn delete_fragment(&mut self, frag_idx: FragIdx) -> Result<(), EditError> {
        self.get_fragment(frag_idx)?; // Return error if `frag_idx` is out-of-bounds
        self.fragments.remove(frag_idx);
        Ok(())
    }

    /// Splits a given fragment into two fragments, at a given location
    pub fn split_fragment(
        &mut self,
        frag_idx: FragIdx,
        row_idx: isize,
        new_frag_pos: Pos2,
    ) -> Result<(), EditError> {
        let frag_to_split = self.get_fragment_mut(frag_idx)?;
        let new_frag = frag_to_split.split(frag_idx, row_idx, new_frag_pos)?;
        self.fragments.push(Rc::new(new_frag));
        Ok(())
    }

    fn get_fragment(&self, idx: FragIdx) -> Result<&Fragment, EditError> {
        self.fragments
            .get(idx)
            .ok_or(EditError::FragOutOfRange {
                idx,
                len: self.fragments.len(),
            })
            .map(Deref::deref)
    }

    pub(crate) fn get_fragment_mut(&mut self, idx: FragIdx) -> Result<&mut Fragment, EditError> {
        let len = self.fragments.len();
        self.fragments
            .get_mut(idx)
            .ok_or(EditError::FragOutOfRange { idx, len })
            .map(Rc::make_mut)
    }
}

/// A single `Fragment` of composition.
#[derive(Debug, Clone)]
pub struct Fragment {
    /// The on-screen location of the top-left corner of the top row this `Frag`
    position: Pos2,
    start_row: Rc<RowBuf>,
    /// A sequence of [`Chunk`]s that make up this `Fragment`
    chunks: ChunkVec<Rc<Chunk>>,
    /// Set to `false` if this `Fragment` is visible but 'muted' - i.e. visually greyed out and not
    /// included in the proving, ATW calculations, statistics, etc.
    is_proved: bool,
}

impl Fragment {
    /// Toggles whether or not this `Fragment` is proved.  This never fails, but returns a
    /// [`Result`] to make it more convenient to use with
    /// [`History::apply_frag_edit`](crate::history::History::apply_frag_edit).
    pub fn toggle_mute(&mut self) -> Result<(), EditError> {
        self.is_proved = !self.is_proved;
        Ok(())
    }

    /// Gets the number of non-leftover [`Row`]s in this [`Fragment`] in one part of the
    /// composition.
    pub(crate) fn len(&self) -> usize {
        self.chunks.iter().map(|c| c.len()).sum()
    }

    /// Shortens `self` such that the row at `split_idx` becomes leftover, returning a new
    /// `Fragment` containing the remaining [`Row`]s
    fn split(
        &mut self,
        frag_idx: FragIdx,
        split_idx: isize,
        new_frag_pos: Pos2,
    ) -> Result<Self, EditError> {
        // Compute which chunk contains the split point
        let (chunk_idx, sub_chunk_idx, new_frag_start_row) =
            self.get_row_data(frag_idx, split_idx)?;
        // Split the chunk arrays, singling out the chunk which must be split
        let other_chunks = self.chunks.split_off(chunk_idx + 1);
        let chunk_being_split = self.chunks.pop().unwrap();
        // Split the chunk
        let (chunk_before_split, chunk_after_split) = chunk_being_split.split(sub_chunk_idx)?;
        // Put the first half of the split chunk back onto `self` (if it's non-empty)
        self.chunks.extend(chunk_before_split);

        // Construct the chunks for the other fragment
        let mut new_frag_chunks = ChunkVec::with_capacity(other_chunks.len() + 1);
        new_frag_chunks.extend(chunk_after_split);
        new_frag_chunks.extend(other_chunks);
        // Construct and return the fragment containing the part of `self` after the split
        Ok(Fragment {
            position: new_frag_pos,
            start_row: Rc::new(new_frag_start_row),
            chunks: new_frag_chunks,
            is_proved: self.is_proved, // Inherit proved-ness from `self`
        })
    }

    /// Given a (possibly negative) row index, this returns a tuple of
    /// `(chunk index, sub-chunk index, row)` at that index, or `None` if the index is
    /// out-of-bounds.
    fn get_row_data(
        &self,
        frag_idx: FragIdx,
        idx: isize,
    ) -> Result<(ChunkIdx, usize, RowBuf), EditError> {
        self.get_row_data_option(idx)
            .ok_or_else(|| EditError::RowOutOfRange {
                frag_idx,
                row_idx: idx,
                frag_len: self.len(),
            })
    }
    /// Given a (possibly negative) row index, this returns a tuple of
    /// `(chunk index, sub-chunk index, row)` at that index, or `None` if the index is
    /// out-of-bounds.
    fn get_row_data_option(&self, row_idx: isize) -> Option<(ChunkIdx, usize, RowBuf)> {
        let row_idx: usize = row_idx.try_into().ok()?; // Negative rows are never in-bounds

        let mut chunk_start_idx = 0usize;
        let mut chunk_start_row = RowAccumulator::new(self.start_row.as_ref().clone());
        for (chunk_idx, chunk) in self.chunks.iter_enumerated() {
            assert!(chunk_start_idx <= row_idx);

            let next_chunk_start_idx = chunk_start_idx + chunk.len();
            // If this chunk ends **after** the row's index, then this chunk must contain the row
            // at `idx`
            if row_idx < next_chunk_start_idx {
                let sub_chunk_idx = row_idx - chunk_start_idx;
                // Accumulate the part of the chunk which contains `row_idx`
                let mut final_row_accum = chunk_start_row;
                chunk
                    .accumulate_transposition_to(sub_chunk_idx, &mut final_row_accum)
                    .unwrap();
                let final_row = final_row_accum.into_total();
                return Some((chunk_idx, sub_chunk_idx, final_row));
            }
            // If this chunk didn't contain `idx`, then keep searching
            chunk_start_idx = next_chunk_start_idx;
            chunk_start_row *= chunk.transposition();
        }
        // If none of the chunks contain `idx`, then it must be out of range
        assert!(row_idx >= chunk_start_idx);
        None
    }

    /// Runs a bounds check on a row index (i.e. checking that the row at `idx` is non-leftover),
    /// and generates a helpful error message when out-of-bounds.
    #[allow(dead_code)] // TODO: This is probably replaced by `get_row_data`, so if it isn't used
                        // for a while after 2021-08-30 then delete
    fn test_row_idx(&self, frag_idx: FragIdx, idx: isize) -> Result<RowIdx, EditError> {
        /// Returns `Some(RowIdx)` if `idx` is within `0..len`, else `None`
        fn test_idx_option(idx: isize, len: usize) -> Option<RowIdx> {
            let positive_idx = usize::try_from(idx).ok()?;
            (positive_idx < len).then(|| positive_idx).map(RowIdx::from)
        }

        let len = self.len();
        test_idx_option(idx, len).ok_or(EditError::RowOutOfRange {
            frag_idx,
            row_idx: idx,
            frag_len: len,
        })
    }
}

/// A `Chunk` of a [`Fragment`], consisting of either a contiguous segment of a [`Method`] or a
/// [`Call`] rung all the way through
#[derive(Debug, Clone)]
enum Chunk {
    Method {
        method: Rc<Method>,
        start_sub_lead_index: usize,
        length: usize,
        transposition: RowBuf,
    },
    Call {
        call: Rc<Call>,
        method: Rc<Method>,
        start_sub_lead_index: usize,
    },
}

impl Chunk {
    /// Creates a new [`Chunk::Method`], computing the `transposition` field.
    ///
    /// # Panics
    ///
    /// Panics if `length` is `0`
    fn method(method: Rc<Method>, start_sub_lead_index: usize, length: usize) -> Self {
        assert_ne!(length, 0);
        let start_sub_lead_index = start_sub_lead_index % method.lead_len();

        // Compute the transposition
        let start_transposition = method.inner.row_in_plain_lead(start_sub_lead_index);
        let end_transposition = method
            .inner
            .row_in_plain_course(start_sub_lead_index + length);
        let transposition =
            // Unwrap is safe, because `start_transposition` and `end_transposition` both originate
            // from the same Method
            Row::solve_ax_equals_b(start_transposition, &end_transposition).unwrap();

        Chunk::Method {
            method,
            start_sub_lead_index,
            length,
            transposition,
        }
    }

    /// Accumulates the (post-) transposition from the first [`Row`] of `self` to the row at
    /// `row_idx`.
    ///
    /// # Panics
    ///
    /// Panics if `row_idx >= self.len`
    fn accumulate_transposition_to(
        &self,
        row_idx: usize,
        accum: &mut RowAccumulator,
    ) -> Result<(), IncompatibleStages> {
        match self {
            Chunk::Method {
                method,
                start_sub_lead_index,
                length,
                transposition: _,
            } => {
                assert!(row_idx < *length);
                // The index of the row at `row_idx`, relative to the start of the first lead of
                // this block (i.e. the one that contains `start_sub_lead_index`
                let end_idx = *start_sub_lead_index + row_idx;
                // How many lead ends are there between the start of `self` and the row at
                // `row_idx`
                let num_leads = end_idx / method.lead_len();
                // The sub-lead index of the row at `row_idx`
                let end_sub_lead_index = end_idx % method.lead_len();
                // Update the `accum` to refer to the lead end containing the start row of `self`
                accum.accumulate(&method.inner.row_in_plain_lead(*start_sub_lead_index).inv())?;
                // Accumulate full leads until we reach the lead containing `row_idx`
                for _ in 0..num_leads {
                    accum.accumulate(method.inner.lead_head())?;
                }
                // Accumulate the remaining changes of the lead containing `row_idx`
                accum.accumulate(method.inner.row_in_plain_lead(end_sub_lead_index))
            }
            // For a call, we just accumulate the `row_idx`th row of the call
            Chunk::Call { call, .. } => accum.accumulate(&call.inner.block().row_vec()[row_idx]),
        }
    }

    /// Return the number of [`Row`]s generated by this [`Chunk`]
    fn len(&self) -> usize {
        match self {
            Chunk::Method { length, .. } => *length,
            Chunk::Call { call, .. } => call.inner.len(),
        }
    }

    /// The transposition caused by this `Chunk`
    fn transposition(&self) -> &Row {
        match self {
            Chunk::Method { transposition, .. } => transposition,
            Chunk::Call { call, .. } => call.inner.transposition(),
        }
    }

    /// Splits `self` into two chunks.  Empty `Chunk`s are returned as `None`
    #[allow(clippy::type_complexity)]
    fn split(
        self: Rc<Self>,
        at: usize,
    ) -> Result<(Option<Rc<Chunk>>, Option<Rc<Chunk>>), EditError> {
        // Splits where one side is empty are essentially `no-ops` and so can be applied to any
        // `Chunk` (even calls)
        if at == 0 {
            return Ok((None, Some(self)));
        } else if at == self.len() {
            return Ok((Some(self), None));
        }
        match self.as_ref() {
            // Calls can't be split into two sub-chunks
            Chunk::Call { .. } => Err(EditError::SplitCall),
            Chunk::Method {
                method,
                start_sub_lead_index,
                length,
                transposition: _,
            } => {
                let sub_lead_index_of_split = (start_sub_lead_index + at) % method.lead_len();
                let chunk_before_split = Chunk::method(method.clone(), *start_sub_lead_index, at);
                let chunk_after_split =
                    Chunk::method(method.clone(), sub_lead_index_of_split, length - at);
                Ok((
                    Some(Rc::new(chunk_before_split)),
                    Some(Rc::new(chunk_after_split)),
                ))
            }
        }
    }
}

/// The data required to define a [`Method`] that's used somewhere in the composition.  This is a
/// wrapper around [`bellframe::Method`] adding extra data like method shorthand names.
#[derive(Debug, Clone)]
pub(crate) struct Method {
    inner: bellframe::Method,
    /// The name (not title) of this `Method`.  For example, the method who's title is `"Bristol
    /// Surprise Major"` would have name `"Bristol"`.
    name: RefCell<String>,
    /// A short string which denotes this Method.  There are no restrictions on this - they do not
    /// even have to be unique or non-empty (since the rows store their corresponding method
    /// through an [`Rc`]).  For example, `B` is often used as a shorthand for `"Bristol Surprise
    /// Major"`.
    shorthand: RefCell<String>,
    /// Which locations in the lead should have lines drawn **above** them
    ruleoffs_above: HashSet<usize>, // TODO: Use a bitmask
}

impl Method {
    fn with_lead_end_ruleoff(inner: bellframe::Method, name: String, shorthand: String) -> Self {
        Self::new(inner, name, shorthand, std::iter::once(0).collect())
    }

    fn new(
        inner: bellframe::Method,
        name: String,
        shorthand: String,
        ruleoffs: HashSet<usize>,
    ) -> Self {
        Self {
            inner,
            name: RefCell::new(name),
            shorthand: RefCell::new(shorthand),
            ruleoffs_above: ruleoffs,
        }
    }

    #[inline]
    pub fn lead_len(&self) -> usize {
        self.inner.lead_len()
    }

    pub fn shorthand(&self) -> Ref<String> {
        self.shorthand.borrow()
    }

    pub fn name(&self) -> Ref<String> {
        self.name.borrow()
    }

    pub fn is_ruleoff_below(&self, sub_lead_idx: usize) -> bool {
        // We store which rows have ruleoffs **above** them, so we have to query the row below the
        // one specified by `sub_lead_idx`
        let idx = (sub_lead_idx + 1) % self.inner.lead_len();
        self.ruleoffs_above.contains(&idx)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Call {
    inner: bellframe::Call,
}

/// A point where the composition can be folded.  Composition folding is not part of the undo
/// history and therefore relies on interior mutability.
#[derive(Debug, Clone)]
pub(crate) struct Fold {
    is_open: Cell<bool>,
}

/////////////////
// ERROR TYPES //
/////////////////

/// The possible ways that editing a [`CompSpec`] can fail
#[derive(Debug, Clone)]
pub enum EditError {
    FragOutOfRange {
        idx: FragIdx,
        len: usize,
    },
    RowOutOfRange {
        frag_idx: FragIdx,
        row_idx: isize, // Can be negative if the user was hovering above the first row
        frag_len: usize,
    },
    // Trying to split the region covered by a call
    SplitCall,
}

///////////////
// EXPANSION //
///////////////

impl Fragment {
    fn expand(&self, part_heads: &PartHeads) -> ExpandedFrag {
        let mut rows_in_one_part = AnnotBlock::<()>::empty(self.start_row.stage());
        rows_in_one_part.pre_multiply(&self.start_row).unwrap(); // Set the start row of the first chunk
        let mut row_data = RowVec::<RowData>::with_capacity(self.len() + 1);
        // Expand the chunks for a single part (i.e. the part with a part head of rounds)
        for chunk in &self.chunks {
            chunk.expand_one_part(&mut rows_in_one_part, &mut row_data, self.is_proved);
        }
        // Create row data for the leftover row
        row_data.push(RowData {
            method_source: None,
            call_source: None,
            is_proved: false, // leftover rows are never proved
        });
        // Expand the rows across the part heads, thus generating the rows in each part
        ExpandedFrag::from_single_part(
            rows_in_one_part.into_row_vec(),
            row_data,
            self.is_proved,
            self.position,
            part_heads,
        )
    }
}

impl Chunk {
    fn expand_one_part(
        &self,
        rows_in_one_part: &mut AnnotBlock<()>,
        row_data: &mut RowVec<RowData>,
        is_proved: bool,
    ) {
        match self {
            Chunk::Method {
                method,
                start_sub_lead_index,
                length,
                transposition: _,
            } => {
                let unannotated_first_lead = method
                    .inner
                    .first_lead()
                    .clone_map_annots_with_index(|_, _| ()); // PERF: Compute this once per method
                let lead_len = method.inner.lead_len();
                // Extend row data
                row_data.extend((0..*length).map(|i| {
                    let sub_lead_idx = (*start_sub_lead_index + i) % lead_len;
                    RowData {
                        method_source: Some((method.clone(), sub_lead_idx)),
                        call_source: None,
                        is_proved,
                    }
                }));
                // Extend rows a lead at a time
                let mut start_sub_lead_index = *start_sub_lead_index;
                let mut length_left_to_add = *length;
                // While there's more length to be added ...
                while length_left_to_add > 0 {
                    // ... extend rows by either `length` rows or until the Method's lead repeats
                    // (whichever appears sooner).
                    let end_sub_lead_index =
                        std::cmp::min(start_sub_lead_index + length_left_to_add, lead_len);
                    let sub_lead_range = start_sub_lead_index..end_sub_lead_index;
                    rows_in_one_part
                        .extend_range(&unannotated_first_lead, sub_lead_range)
                        .unwrap();
                    // Update vars for next loop iteration
                    let num_rows_added = end_sub_lead_index - start_sub_lead_index;
                    length_left_to_add -= num_rows_added;
                    start_sub_lead_index = 0; // After the first iteration, we always start chunks
                                              // at lead ends
                }
            }
            Chunk::Call {
                call,
                method: _,
                start_sub_lead_index: _,
            } => {
                let block = call.inner.block();
                // TODO: Extend row data
                // Extend rows
                rows_in_one_part.extend(block).unwrap();
                // Update the start row of the next chunk
                todo!() // Decide what lead indices should be given
            }
        }
    }
}
