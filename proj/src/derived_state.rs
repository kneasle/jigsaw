use crate::spec::{AnnotatedRow, Spec};
use proj_core::{run_len, Bell, Row, Stage};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// A small datatype that represents **where** a given row comes from in the composition.  This is
/// useful because the composition contains many fragments, and each row of each fragment could
/// expand into multiple actual rows (one for each part).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct RowOrigin {
    /// The index of the part that this [`Row`] came from
    part: usize,
    /// The index of the fragment that this [`Row`] came from
    frag: usize,
    /// The index of the row within the fragment that this [`Row`] came from
    row: usize,
}

impl RowOrigin {
    /// Creates a new `RowOrigin` from it's parts
    pub fn new(part: usize, frag: usize, row: usize) -> RowOrigin {
        RowOrigin { part, frag, row }
    }
}

/// A small datatype that represents **where** a given row comes from in the composition (without
/// knowledge of which part this comes from).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RowLocation {
    /// The index of the fragment that this [`Row`] came from
    frag: usize,
    /// The index of the row within the fragment that this [`Row`] came from
    row: usize,
}

impl RowLocation {
    /// Creates a new `RowOrigin` from it's parts
    pub fn new(frag: usize, row: usize) -> Self {
        RowLocation { frag, row }
    }
}

impl From<RowOrigin> for RowLocation {
    fn from(o: RowOrigin) -> Self {
        RowLocation {
            frag: o.frag,
            row: o.row,
        }
    }
}

// Required so that we can omit `"is_leftover" = false` when serialising
fn is_false(b: &bool) -> bool {
    !b
}
fn is_all_empty(vs: &Vec<Vec<usize>>) -> bool {
    vs.iter().all(Vec::is_empty)
}

/// All the information required for JS to render a single [`Row`] from the [`Spec`].  Note that
/// because of multipart expansion, this single on-screen [`Row`] actually represents many expanded
/// [`Row`]s, and this datatype reflects that.
#[derive(Serialize, Debug, Clone)]
pub struct ExpandedRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    call_str: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    method_str: Option<String>,
    is_lead_end: bool,
    #[serde(skip_serializing_if = "is_false")]
    is_leftover: bool,
    /// One [`Row`] for each part of the composition
    expanded_rows: Vec<Vec<usize>>,
    /// For each bell, shows which parts contain music
    ///
    /// E.g. for `21345678` under part heads `12345678, 18234567, ...` would form rows
    /// ```text
    /// 0: 21345678
    /// 1: 81234567
    /// 2: 71823456
    /// 3: 61782345
    /// 4: 51678234
    /// 5: 41567823
    /// 6: 31456782
    /// ```
    /// and the highlights would be:
    /// ```
    /// vec![
    ///     vec![],
    ///     vec![1],
    ///     vec![1, 2],
    ///     vec![0, 1, 2],
    ///     vec![0, 1, 2, 3],
    ///     vec![0, 1, 2, 3],
    ///     vec![0, 1, 2, 3],
    ///     vec![0, 1, 2, 3]
    /// ]
    /// ```
    #[serde(skip_serializing_if = "is_all_empty")]
    music_highlights: Vec<Vec<usize>>,
}

impl ExpandedRow {
    fn calculate_music(all_rows: &[Row], stage: Stage) -> Vec<Vec<usize>> {
        // Initialise the music scores with 0 for every place
        let mut music = vec![Vec::new(); stage.as_usize()];
        // For each part that contains music, add one to the bells which are covered by the music
        for (part, r) in all_rows.iter().enumerate() {
            // Highlight runs of >=4 bells of the **front**
            let run_len_f = run_len(r.bells());
            if run_len_f >= 4 {
                for i in 0..run_len_f {
                    music[i].push(part);
                }
            }
            // Highlight runs of >=4 bells of the **back**
            let run_len_b = run_len(r.bells().rev());
            if run_len_b >= 4 {
                // The 'max' prevents the two ranges from overlapping and causing music in multiple
                // parts from being counted twice
                for i in (stage.as_usize() - run_len_b).max(run_len_f)..stage.as_usize() {
                    music[i].push(part);
                }
            }
        }
        music
    }

    pub fn new(row: &AnnotatedRow, part_heads: &[Row], is_leftover: bool) -> Self {
        let all_rows: Vec<Row> = part_heads.iter().map(|ph| ph * &row.row).collect();
        ExpandedRow {
            call_str: row.call_str.clone(),
            method_str: row.method_str.clone(),
            is_lead_end: row.is_lead_end,
            music_highlights: Self::calculate_music(&all_rows, row.row.stage()),
            expanded_rows: all_rows
                .into_iter()
                .map(|r| r.bells().map(Bell::index).collect())
                .collect(),
            is_leftover,
        }
    }
}

/// A range of rows which should be highlighted as all false in the same way.  This is supposed to
/// cover `start..=end` rows (i.e. the ranges are **inclusive**).
#[derive(Serialize, Debug, Clone)]
pub struct FalseRowRange {
    start: usize,
    end: usize,
    group: usize,
}

/// The information required for JS to render a [`Frag`]
#[derive(Serialize, Debug, Clone)]
pub struct AnnotFrag {
    false_row_ranges: Vec<FalseRowRange>,
    exp_rows: Vec<ExpandedRow>,
}

#[derive(Serialize, Debug, Clone)]
pub struct DerivedState {
    annot_frags: Vec<AnnotFrag>,
    num_rows: usize,
    num_false_rows: usize,
}

impl DerivedState {
    pub fn from_spec(spec: &Spec) -> DerivedState {
        // We use a hashset because if the part heads form a group then any falseness will be the
        // same between all the parts, so will appear lots of times.
        let mut false_rows: HashSet<Vec<RowLocation>> = HashSet::new();
        let num_false_rows = {
            // Expand all the rows and their origins from the composition into a `Vec` to be
            // proved, excluding the last Row of each Frag, since that is 'left over' and as such
            // shouldn't be used of proving
            let mut all_rows: Vec<(RowOrigin, Row)> = Vec::with_capacity(spec.len());
            for (p_ind, part_head) in spec.part_heads.iter().enumerate() {
                for (f_ind, frag) in spec.frags.iter().enumerate() {
                    for (r_ind, row) in frag.rows[..frag.rows.len() - 1].iter().enumerate() {
                        all_rows.push((RowOrigin::new(p_ind, f_ind, r_ind), part_head * &row.row));
                    }
                }
            }
            // Sort all_rows only by their rows, so that false rows are appear next to each other
            all_rows.sort_by(|(_, r1), (_, r2)| r1.cmp(r2));
            // The origins of the current set of duplicated rows.  Most of the time, we hope that
            // this has length 1, i.e. all rows are unique.
            let mut current_false_row_group: Vec<RowLocation> = Vec::with_capacity(10);
            let mut last_row = None;
            let mut num_false_rows = 0usize;
            // Iterate over all the rows, compiling groups as we go
            for (o, r) in all_rows {
                if let Some(l_r) = &last_row {
                    if l_r != &r {
                        // If we reach this branch of the code, then it means that we are just
                        // starting a new set of rows and we need to check the last group for
                        // falseness.
                        if current_false_row_group.len() > 1 {
                            // Add these rows to the falseness counter
                            num_false_rows += current_false_row_group.len();
                            // If we saw more than 1 identical rows, then this counts as falseness
                            // and so we add this to the set of false rows.  We sort the row
                            // locations first, to make sure that if the same set of `RowLocation`s
                            // is found twice they are always added once (regardless of which order
                            // the rows were discovered).
                            current_false_row_group.sort();
                            false_rows.insert(std::mem::take(&mut current_false_row_group));
                        } else {
                            current_false_row_group.clear();
                        }
                    }
                }
                // Make sure that the current row becomes the last row for the next iteration, and
                // add this location to the current group.
                last_row = Some(r);
                current_false_row_group.push(RowLocation::from(o));
            }
            // Make sure that we don't miss the last false row group
            if current_false_row_group.len() > 1 {
                // Add these rows to the falseness counter
                num_false_rows += current_false_row_group.len();
                // If we saw more than 1 identical rows, then this counts as falseness and so we
                // add this to the set of false rows.  We sort the row locations first, to make
                // sure that if the same set of `RowLocation`s is found twice they are always added
                // once (regardless of which order the rows were discovered).
                current_false_row_group.sort();
                false_rows.insert(current_false_row_group);
            }
            // Return the number of false rows out of the block
            num_false_rows
        };
        /* Combine adjacent false row groups so that we use up fewer colours.  This relies on the
         * fact that all the `Vec`s in `false_rows` are sorted in increasing order by frag index and
         * then row index (and a unit test checks that). */
        let mut ranges_by_frag: HashMap<usize, Vec<FalseRowRange>> = HashMap::new();
        {
            /// A cheeky helper function which adds the ranges between two groups of false rows to
            /// the right places in a HashMap (the map will only ever be `row_groups_by_frag`)
            fn add_ranges(
                map: &mut HashMap<usize, Vec<FalseRowRange>>,
                start: &[RowLocation],
                end: &[RowLocation],
                group_id: usize,
            ) {
                // Check that we aren't losing information by zipping the two groups
                assert_eq!(start.len(), end.len());
                // Zip through the locations in each group.  Because of their sortedness, we can
                // guaruntee that the pairs are joined up correctly
                for (start_loc, end_loc) in start.iter().zip(end) {
                    // Check that each both locations belong to the same group.  This should be
                    // guarunteed by the adjacency test, but we test it anyway.
                    assert_eq!(start_loc.frag, end_loc.frag);
                    // Create a FalseRowGroup, making sure that start <= end (because we could have
                    // sets of rows which are false against each other but in the opposite order)
                    let false_row_range = FalseRowRange {
                        start: start_loc.row.min(end_loc.row),
                        end: start_loc.row.max(end_loc.row),
                        group: group_id,
                    };
                    // Insert the newly created group to the HashMap to make sure it's displayed on
                    // the correct fragment
                    map.entry(start_loc.frag)
                        .or_insert(vec![])
                        .push(false_row_range);
                }
            }
            // Firstly, convert the existing HashSet into a Vec, and sort it
            let mut false_rows_vec = false_rows.into_iter().collect::<Vec<_>>();
            false_rows_vec.sort();
            // Because `false_rows_vec` itself and all its contents are sorted, we can guaruntee
            // that the rows making up each false row group are sequential in the listing
            let mut iter = false_rows_vec.iter();
            let mut group_id = 0;
            if let Some(first_group) = iter.next() {
                let mut last_group = first_group;
                let mut first_group_in_meta_group = first_group;
                for group in iter {
                    // Decide if this group is adjacent to the last one (for two groups to be
                    // adjacent, they need to have the same length and all the `RowLocation`s must
                    // also be adjacent).
                    let is_adjacent_to_last = group.len() == last_group.len()
                        && group.iter().zip(last_group.iter()).all(|(loc1, loc2)| {
                            loc1.frag == loc2.frag
                                && (loc1.row as isize - loc2.row as isize).abs() == 1
                        });
                    if !is_adjacent_to_last {
                        /* If this group is not adjacent to the last one, then we have just
                         * finished a group.  We therefore need to calculate the ranges for the
                         * group we finished, adding them to a HashMap (index by fragment) to be
                         * displayed */
                        // The next meta-group should start with the group we found
                        add_ranges(
                            &mut ranges_by_frag,
                            first_group_in_meta_group,
                            last_group,
                            group_id,
                        );
                        first_group_in_meta_group = group;
                        group_id += 1;
                    }
                    last_group = group;
                }
                // Make sure that we add the ranges containing the last group
                add_ranges(
                    &mut ranges_by_frag,
                    first_group_in_meta_group,
                    last_group,
                    group_id,
                );
            }
        }
        // Compile all of the derived state into one struct
        DerivedState {
            annot_frags: spec
                .frags
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let last_row_ind = f.rows.len() - 1;
                    AnnotFrag {
                        false_row_ranges: ranges_by_frag.remove(&i).unwrap_or(vec![]),
                        exp_rows: f
                            .rows
                            .iter()
                            .enumerate()
                            .map(|(i, r)| ExpandedRow::new(r, &spec.part_heads, i == last_row_ind))
                            .collect(),
                    }
                })
                .collect(),
            num_rows: spec.proof_len(),
            num_false_rows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RowLocation;

    /// Check that [`RowLocation`]s are sorted by frag index and then row index.  This is required
    /// for the group coalescing to work.
    #[test]
    fn row_loc_ord() {
        /// Helper constructor for [`RowLocation`]s
        fn rl(frag: usize, row: usize) -> RowLocation {
            RowLocation { frag, row }
        }
        assert!(rl(0, 0) < rl(1, 0));
        assert!(rl(0, 1) < rl(1, 0));
        assert!(rl(2, 1) > rl(1, 3));
        assert!(rl(0, 1) < rl(0, 3));
        assert!(rl(1, 0) > rl(0, 100));
    }
}
