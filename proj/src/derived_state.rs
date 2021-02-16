use crate::spec::{AnnotatedRow, Spec};
use proj_core::{run_len, Row, Stage};

/// A small datatype that represents **where** a given row comes from in the composition.  This is
/// useful because the composition contains many fragments, and each row of this could expand into
/// multiple actual rows (one for each part).
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

/// All the information required for JS to render a single [`Row`] from the [`Spec`].  Note that
/// because of multipart expansion, this single on-screen [`Row`] actually represents many expanded
/// [`Row`]s, and this datatype reflects that.
#[derive(Debug, Clone)]
pub struct ExpandedRow {
    call_str: Option<String>,
    method_str: Option<String>,
    is_lead_end: bool,
    /// One [`Row`] for each part of the composition
    pub expanded_rows: Vec<Row>,
    pub music_highlights: Vec<usize>,
}

impl ExpandedRow {
    fn calculate_music(all_rows: &[Row], stage: Stage) -> Vec<usize> {
        // Initialise the music scores with 0 for every place
        let mut music = vec![0; stage.as_usize()];
        // For each part that contains music, add one to the bells which are covered by the music
        for r in all_rows {
            // Highlight runs of >=4 bells of the **front**
            let run_len_f = run_len(r.iter());
            if run_len_f >= 4 {
                for i in 0..run_len_f {
                    music[i] += 1;
                }
            }
            // Highlight runs of >=4 bells of the **back**
            let run_len_b = run_len(r.iter().rev());
            if run_len_b >= 4 {
                // The 'max' prevents the two ranges from overlapping and causing music in multiple
                // parts from being counted twice
                for i in (stage.as_usize() - run_len_b).max(run_len_f)..stage.as_usize() {
                    music[i] += 1;
                }
            }
        }
        music
    }

    pub fn new(row: &AnnotatedRow, part_heads: &[Row]) -> Self {
        let all_rows: Vec<Row> = part_heads.iter().map(|ph| ph * &row.row).collect();
        ExpandedRow {
            call_str: row.call_str.clone(),
            method_str: row.method_str.clone(),
            is_lead_end: row.is_lead_end,
            music_highlights: Self::calculate_music(&all_rows, row.row.stage()),
            expanded_rows: all_rows,
        }
    }
}

/// The information required for JS to render the rows inside a [`Frag`]
#[derive(Debug, Clone)]
pub struct AnnotFrag {
    false_row_groups: Vec<()>,
    pub exp_rows: Vec<ExpandedRow>,
}

#[derive(Debug, Clone)]
pub struct DerivedState {
    pub annot_frags: Vec<AnnotFrag>,
}

impl DerivedState {
    pub fn from_spec(spec: &Spec) -> DerivedState {
        DerivedState {
            annot_frags: spec
                .frags
                .iter()
                .enumerate()
                .map(|(_frag_index, f)| AnnotFrag {
                    false_row_groups: Vec::new(),
                    exp_rows: f
                        .rows
                        .iter()
                        .map(|r| ExpandedRow::new(r, &spec.part_heads))
                        .collect(),
                })
                .collect(),
        }
    }
}
