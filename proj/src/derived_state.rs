use crate::spec::{AnnotatedRow, Spec};
use itertools::Itertools;
use proj_core::{Bell, Row};

/// Helper function to calculate the length of the longest run off the start of a given
/// [`Iterator`]
fn run_len(iter: impl Iterator<Item = Bell>) -> usize {
    iter.map(|b| b.index())
        .tuple_windows::<(usize, usize)>()
        .take_while(|&(i1, i2)| (i1 as isize - i2 as isize).abs() == 1)
        .count()
        + 1
}

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
    expanded_rows: Vec<Row>,
    pub highlight_ranges: Vec<(usize, usize)>,
}

impl ExpandedRow {
    fn generate_hl_ranges(row: &Row) -> Vec<(usize, usize)> {
        // Build the ranges, given the run lengths off front and back
        // TODO: Remove overlapping ranges, and allow accurate music detection across multiparts
        let mut hl_ranges = Vec::new();
        let run_len_f = run_len(row.iter());
        if run_len_f >= 4 {
            hl_ranges.push((0, run_len_f));
        }
        let run_len_b = run_len(row.iter().rev());
        if run_len_b >= 4 {
            let stage = row.stage().as_usize();
            hl_ranges.push((stage - run_len_b, stage));
        }
        hl_ranges
    }

    pub fn new(row: &AnnotatedRow, part_heads: &[Row]) -> Self {
        let all_rows: Vec<Row> = part_heads.iter().map(|ph| ph * &row.row).collect();
        ExpandedRow {
            call_str: row.call_str.clone(),
            method_str: row.method_str.clone(),
            is_lead_end: row.is_lead_end,
            highlight_ranges: Self::generate_hl_ranges(&all_rows[0]),
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

#[cfg(test)]
mod tests {
    use super::run_len as rl;
    use proj_core::Row;

    #[test]
    fn run_len() {
        for &(row, run_len_f) in &[("123456", 6), ("456231", 3), ("612345", 1)] {
            assert_eq!(rl(Row::parse(row).unwrap().iter()), run_len_f);
        }
    }
}
