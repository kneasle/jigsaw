//! Code for expanding a [`CompSpec`] into a [`FullComp`] that represents the same data.

use bellframe::{AnnotRow, SameStageVec};
use itertools::Itertools;

use super::{ExpandedRow, Fragment, FullComp};
use crate::{
    part_heads::PartHeads,
    spec::{self, CompSpec},
};

/// Convert a [`CompSpec`] to a [`FullComp`] which represents the same composition.  [`FullComp`]
/// explicitly specifies all the information that is implied by a [`CompSpec`], so this function
/// essentially computes that extra information.
pub fn expand(spec: &CompSpec) -> FullComp {
    let fragments = spec
        .fragments()
        .map(|f| expand_fragment(f, spec.part_heads()))
        .collect_vec();

    // TODO: Compute information (like falseness, atw, etc.) which requires data from multiple
    // fragments/methods/calls, etc.

    FullComp { fragments }
}

/// Expand a [`spec::Fragment`] into a [`Fragment`]
fn expand_fragment(fragment: &spec::Fragment, part_heads: &PartHeads) -> Fragment {
    let expanded_rows = fragment
        .annot_rows()
        .map(|r| expand_row(r, part_heads, fragment.is_proved()))
        .collect_vec();

    // TODO: Populate the fields of the `ExpandedRow`s that require cross-row information

    Fragment {
        position: fragment.position(),
        link_group_top: None,    // Link groups will be filled later
        link_group_bottom: None, // Link groups will be filled later
        expanded_rows,
    }
}

/// Expand a source row as much as possible without requiring information about other rows or
/// fragments.
fn expand_row(
    annot_row: AnnotRow<spec::RowData>,
    part_heads: &PartHeads,
    is_frag_proved: bool,
) -> ExpandedRow {
    // Generate one expanded row per part head
    let mut row_per_part = SameStageVec::with_capacity(annot_row.row().stage(), part_heads.len());
    for part_head in part_heads.rows() {
        let row_in_part = part_head.as_row() * annot_row.row();
        row_per_part
            .push(&row_in_part)
            .expect("Part heads should have same stage as rows");
    }

    ExpandedRow {
        rows: row_per_part,
        is_proved: is_frag_proved,
        is_false: false, // Will be filled in later
    }
}
