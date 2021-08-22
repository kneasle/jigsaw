//! Code for expanding a [`CompSpec`] into a [`FullComp`] that represents the same data.

use std::{collections::HashMap, rc::Rc};

use bellframe::{Row, RowBuf, SameStageVec, Stage};
use itertools::Itertools;

use crate::state::{
    full::{self, FullState},
    music,
};

use super::{part_heads::PartHeads, Chunk, CompSpec, Fragment, Method};

type MethodMap = HashMap<*const super::Method, full::Method>;

/// Convert a [`CompSpec`] to a [`FullComp`] which represents the same composition.  [`FullComp`]
/// explicitly specifies all the information that is implied by a [`CompSpec`], so this function
/// essentially computes that extra information.
pub(in crate::state) fn expand(spec: &CompSpec, music: &[music::Music]) -> FullState {
    // Stats will be accumulated during the expansion process
    let mut stats = full::Stats::default();

    // Maps source methods [`super::Method`] (hashed by their memory addresses) to the expanded
    // [`self::Method`].  This is used so that the fragment expansion, which receives rows
    // containing `Rc<super::Method>` can know which `full::Method` it corresponds to (so its row
    // counters can be updated).
    let mut method_map = spec
        .methods
        .iter()
        .map(|m| {
            let expanded_method = expand_method(m);
            let source_ptr = m.as_ref() as *const Method;
            (source_ptr, expanded_method)
        })
        .collect::<HashMap<_, _>>();

    // Expand as much of the fragment information as we can without using relations **between**
    // fragments.  Other things (like falseness) will be computed after all the fragments have been
    // expanded individually.
    let fragments = spec
        .fragments
        .iter()
        .map(|f| expand_fragment(f, &spec.part_heads, &mut method_map, &mut stats))
        .collect_vec();

    // Expand music
    let (music_groups, total_count, max_count) = expand_music_groups(music, &fragments, spec.stage);
    let music = full::Music {
        groups: music_groups,
        total_count,
        max_count,
    };

    // TODO: Compute information (like falseness, atw, etc.) which requires data from multiple
    // fragments/methods/calls, etc.

    FullState {
        part_heads: spec.part_heads.clone(),
        fragments,
        music,
        // TODO: In Rust `1.54` we can use `into_values()`
        methods: method_map.into_iter().map(|(_k, v)| v).collect_vec(),
        stats,
    }
}

/// Expand a [`spec::Fragment`] into a [`Fragment`]
fn expand_fragment(
    fragment: &Fragment,
    part_heads: &PartHeads,
    method_map: &mut MethodMap,
    stats: &mut full::Stats,
) -> full::Fragment {
    // Update statistics
    stats.part_len += fragment.len(); // Update the length

    // Expand the fragment's chunks
    let mut expanded_rows = Vec::<full::ExpandedRow>::with_capacity(fragment.len());
    let mut chunk_start_row = fragment.start_row.as_ref().to_owned();
    for chunk in &fragment.chunks {
        // Update method stats for this chunk
        let num_rows_in_all_parts = chunk.len() * part_heads.len();
        let source_method_ptr = chunk.method() as *const Method;
        let full_method = method_map.get_mut(&source_method_ptr).unwrap();
        full_method.num_rows += num_rows_in_all_parts;
        if fragment.is_proved {
            full_method.num_proved_rows += num_rows_in_all_parts;
        }

        // TODO: Update ATW stats

        // Extend rows
        match chunk.as_ref() {
            Chunk::Method {
                method,
                start_sub_lead_index,
                length,
            } => {
                // Compute the lead head of the lead containing the first row in this chunk
                let first_lead = method.inner.first_lead();
                let start_row_in_first_lead = first_lead
                    .get_row(*start_sub_lead_index)
                    .expect("Chunk's sub-lead index out of range");
                let first_lead_head =
                    Row::solve_xa_equals_b(start_row_in_first_lead, &chunk_start_row)
                        .expect("All methods should have the same stage");

                // Create an iterator over the rows in this chunk
                let mut iter = first_lead.repeat_iter(first_lead_head).unwrap();
                // Consume the right number of rows from it
                let mut row_buf = RowBuf::rounds(Stage::ONE);
                for _ in 0..*length {
                    iter.next_into(&mut row_buf)
                        .expect("Method should have non-zero lead length");
                    expanded_rows.push(expand_row(&row_buf, part_heads, fragment.is_proved));
                }
                // Make sure that the next chunk starts with the correct row
                iter.next_into(&mut chunk_start_row).unwrap();
            }
            Chunk::Call { call, .. } => {
                let block = call.inner.block();
                for r in block.rows() {
                    expanded_rows.push(expand_row(r, part_heads, fragment.is_proved));
                }
                chunk_start_row = chunk_start_row.as_row() * block.leftover_row();
            }
        }
    }
    // The contents of `chunk_start_row` become the leftover row of the Fragment (we set
    // `is_proved = false` because leftover rows are never proved).
    expanded_rows.push(expand_row(&chunk_start_row, part_heads, false));

    // TODO: Populate the fields of the `ExpandedRow`s that require cross-row information

    full::Fragment {
        position: fragment.position(),
        link_group_top: None,    // Link groups will be filled later
        link_group_bottom: None, // Link groups will be filled later
        expanded_rows,
    }
}

fn expand_method(method: &Rc<Method>) -> full::Method {
    full::Method {
        source: method.clone(),
        // These counters will be accumulated by `expanded_row`, called by `expand_fragment`
        num_rows: 0,
        num_proved_rows: 0,
    }
}

/////////////////////////
// ROW/CHUNK EXPANSION //
/////////////////////////

/// Expand a leftover [`Row`] as much as possible without requiring information about other
/// rows or fragments.
fn expand_row(row: &Row, part_heads: &PartHeads, is_proved: bool) -> full::ExpandedRow {
    full::ExpandedRow {
        rows: get_rows_per_part(row, part_heads),
        is_proved,
        is_false: false, // Will be filled in later by the truth proving
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

/////////////////////
// MUSIC EXPANSION //
/////////////////////

/// Recursively expand a sequence of music groups, totalling the number of occurrences
fn expand_music_groups(
    music: &[music::Music],
    fragments: &[full::Fragment],
    stage: Stage,
) -> (Vec<full::MusicGroup>, usize, usize) {
    // Expand groups individually
    let music_groups = music
        .iter()
        .map(|m| expand_music_group(m, &fragments, stage))
        .collect_vec();
    // Sum their instances (ignoring the fact that we might double count identical regexes in
    // different groups)
    let total_count = music_groups.iter().map(full::MusicGroup::count).sum();
    let max_count = music_groups.iter().map(full::MusicGroup::max_count).sum();
    (music_groups, total_count, max_count)
}

/// Recursively expand a single [`music::Music`] group
fn expand_music_group(
    group: &music::Music,
    fragments: &[full::Fragment],
    stage: Stage,
) -> full::MusicGroup {
    match group {
        music::Music::Regex(name, regex) => {
            // Count occurrences with a truly beautiful set of nested loops
            let mut count = 0;
            for f in fragments {
                for exp_row in &f.expanded_rows {
                    if !exp_row.is_proved {
                        continue; // Don't count music in rows which aren't proved
                    }
                    for row in &exp_row.rows {
                        if regex.matches(row) {
                            count += 1;
                        }
                    }
                }
            }
            // Use the music group's name, falling back on the regex's representation
            let name = name
                .as_ref()
                .map_or_else(|| regex.to_string(), String::clone);
            let max_count = regex
                .num_matching_rows(stage)
                .expect("Overflow whilst computing num rows");
            full::MusicGroup::Regex {
                name,
                count,
                max_count,
            }
        }
        music::Music::Group(name, source_sub_groups) => {
            let (sub_groups, count, max_count) =
                expand_music_groups(&source_sub_groups, fragments, stage);
            full::MusicGroup::Group {
                name: name.to_owned(),
                count,
                max_count,
                sub_groups,
            }
        }
    }
}
