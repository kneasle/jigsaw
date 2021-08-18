//! Code for expanding a [`CompSpec`] into a [`FullComp`] that represents the same data.

use std::{collections::HashMap, rc::Rc};

use bellframe::{AnnotRow, Row, SameStageVec};
use itertools::Itertools;

use super::{
    spec::{self, part_heads::PartHeads, CompSpec},
    ExpandedRow, Fragment, FullState, Method, Stats,
};

/// Convert a [`CompSpec`] to a [`FullComp`] which represents the same composition.  [`FullComp`]
/// explicitly specifies all the information that is implied by a [`CompSpec`], so this function
/// essentially computes that extra information.
pub(crate) fn expand(spec: &CompSpec) -> FullState {
    // Stats will be accumulated during the expansion process
    let mut stats = Stats::default();

    // Maps pointers to the source [`spec::Method`] (which are hashed by their addresses) to the
    // expanded [`self::Method`]
    let mut method_map = spec
        .method_rcs()
        .iter()
        .map(|m| {
            let expanded_method = expand_method(m);
            let source_ptr = m.as_ref() as *const spec::Method;
            (source_ptr, expanded_method)
        })
        .collect::<HashMap<_, _>>();

    // Expand as much of the fragment information as we can without using relations **between**
    // fragments.  Other things (like falseness) will be computed after all the fragments have been
    // expanded individually.
    let fragments = spec
        .fragments()
        .map(|f| expand_fragment(f, spec.part_heads(), &mut method_map, &mut stats))
        .collect_vec();

    // TODO: Compute information (like falseness, atw, etc.) which requires data from multiple
    // fragments/methods/calls, etc.

    FullState {
        part_heads: spec.part_heads_rc(),
        fragments,
        // TODO: In Rust `1.54` we can use `into_values()`
        methods: method_map.into_iter().map(|(_k, v)| v).collect_vec(),
        stats,
    }
}

/// Expand a [`spec::Fragment`] into a [`Fragment`]
fn expand_fragment(
    fragment: &spec::Fragment,
    part_heads: &PartHeads,
    method_map: &mut HashMap<*const spec::Method, Method>,
    stats: &mut Stats,
) -> Fragment {
    stats.part_len += fragment.len(); // Update the length

    // Expand all rows, including the leftover row - i.e. pre-multiply by each part head to compute
    // the rows in each part
    let mut expanded_rows = fragment
        .annot_rows()
        .map(|annot_row| expand_row(annot_row, part_heads, fragment.is_proved(), method_map))
        .collect_vec();
    // Expand the leftover row as a special case
    expanded_rows.push(expand_leftover_row(fragment.leftover_row(), part_heads));

    // TODO: Populate the fields of the `ExpandedRow`s that require cross-row information

    Fragment {
        position: fragment.position(),
        link_group_top: None,    // Link groups will be filled later
        link_group_bottom: None, // Link groups will be filled later
        expanded_rows,
    }
}

fn expand_method(method: &Rc<spec::Method>) -> Method {
    Method {
        source: method.clone(),
        // These counters will be accumulated by `expanded_row`, called by `expand_fragment`
        num_rows: 0,
        num_proved_rows: 0,
    }
}

///////////////////
// ROW EXPANSION //
///////////////////

/// Expand a non-leftover source row as much as possible without requiring information about other
/// rows or fragments.
fn expand_row(
    annot_row: AnnotRow<spec::RowData>,
    part_heads: &PartHeads,
    is_frag_proved: bool,
    method_map: &mut HashMap<*const spec::Method, Method>,
) -> ExpandedRow {
    let row = annot_row.row();
    let data = annot_row.annot();

    // Accumulate row counters of this Row's Method
    let source_method_ptr = data.method() as *const spec::Method;
    let method = method_map.get_mut(&source_method_ptr).unwrap();
    method.num_rows += part_heads.len(); // The rows in each part are all owned by the same method
    if is_frag_proved {
        method.num_proved_rows += part_heads.len();
    }

    // Pre-multiply this row by each part head
    let row_per_part = get_rows_per_part(row, part_heads);

    ExpandedRow {
        rows: row_per_part,
        is_proved: is_frag_proved,
        is_false: false, // Will be filled in later during falseness checking
    }
}

/// Expand a leftover [`Row`] as much as possible without requiring information about other
/// rows or fragments.
fn expand_leftover_row(row: &Row, part_heads: &PartHeads) -> ExpandedRow {
    ExpandedRow {
        rows: get_rows_per_part(row, part_heads),
        is_proved: false, // Leftover rows are never proved
        is_false: false,  // Won't be filled in later, because unproved rows can't be false
    }
}

/// Helper function that generates a [`SameStageVec`] containing a [`Row`] pre-transposed by each
/// part head.
fn get_rows_per_part(row: &Row, part_heads: &PartHeads) -> SameStageVec {
    let mut row_per_part = SameStageVec::with_capacity(row.stage(), part_heads.len());
    for part_head in part_heads.rows() {
        let row_in_part = part_head.as_row() * row;
        row_per_part
            .push(&row_in_part)
            .expect("Part heads should have same stage as rows");
    }
    row_per_part
}
