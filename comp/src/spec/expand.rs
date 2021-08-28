//! Code for expanding a [`CompSpec`] into a [`FullState`] that represents the same data.

// This gives false positives for raw pointers (which are hashed by the memory address they point
// to).  See https://github.com/rust-lang/rust-clippy/issues/6745 for more details.
#![allow(clippy::mutable_key_type)]

use std::{collections::HashMap, rc::Rc};

use bellframe::{Row, RowBuf, SameStageVec, Stage};
use index_vec::IndexSlice;
use itertools::Itertools;

use crate::{
    full::{self, FullState},
    music,
};
use jigsaw_utils::types::{FragIdx, FragVec, PartIdx, RowLocation, RowVec};

use super::{part_heads::PartHeads, Chunk, CompSpec, Fragment, Method};

type MethodMap = HashMap<*const super::Method, full::Method>;

/// Convert a [`CompSpec`] to a [`FullState`] which represents the same composition.  [`FullState`]
/// explicitly specifies all the information that is implied by a [`CompSpec`], so this function
/// essentially computes that extra information.
pub(crate) fn expand(spec: &CompSpec, music: &[music::Music]) -> FullState {
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
    let mut fragments = spec
        .fragments
        .iter()
        .map(|f| expand_fragment(f, &spec.part_heads, &mut method_map, &mut stats))
        .collect::<FragVec<_>>();

    // Expand music, and add highlight to the musical rows
    let (music_groups, total_count, max_count) =
        expand_music_groups(music, &mut fragments, spec.stage);
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
        stage: spec.stage,
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
    let mut expanded_rows = RowVec::<full::ExpandedRow>::with_capacity(fragment.len());
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
                let mut row_buf = RowBuf::rounds(Stage::ONE); // Temporary buffer to avoid allocations
                for _ in 0..*length {
                    let (sub_lead_idx, annot) = iter
                        .next_into(&mut row_buf)
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
        expanded_rows,
        link_group_top: None,    // Link groups will be filled later
        link_group_bottom: None, // Link groups will be filled later
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
        is_false: false,                  // Populated later by the truth proving
        music_highlights: HashMap::new(), // Populated later by the music checking
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
    fragments: &mut IndexSlice<FragIdx, [full::Fragment]>,
    stage: Stage,
) -> (Vec<Rc<full::MusicGroup>>, usize, usize) {
    // Expand groups individually
    let music_groups = music
        .iter()
        .map(|m| expand_music_group(m, fragments, stage))
        .map(Rc::new)
        .collect_vec();
    // Sum their instances (ignoring the fact that we might double count identical regexes in
    // different groups)
    let total_count = music_groups.iter().map(|g| g.inner.count()).sum();
    let max_count = music_groups.iter().map(|g| g.max_count).sum();
    (music_groups, total_count, max_count)
}

/// Recursively expand a single [`music::Music`] group
fn expand_music_group(
    group: &music::Music,
    fragments: &mut IndexSlice<FragIdx, [full::Fragment]>,
    stage: Stage,
) -> full::MusicGroup {
    match group {
        music::Music::Regex(name, regex) => {
            // Compute where this `Regex` is matched in the composition
            let mut rows_matched = Vec::<RowLocation>::new();
            for (frag_index, frag) in fragments.iter_mut_enumerated() {
                for (row_index, exp_row) in frag.expanded_rows.iter_mut_enumerated() {
                    let mut matches_per_part = Vec::<(PartIdx, Vec<usize>)>::new();
                    for (part_index, row) in exp_row.rows.iter().enumerate() {
                        let part_index = PartIdx::new(part_index);
                        if let Some(matched_places) = regex.match_pattern(row) {
                            // Mark on the music pattern that this row matches it (but only if the
                            // row is proved)
                            if exp_row.is_proved {
                                rows_matched.push(RowLocation {
                                    frag_index,
                                    row_index,
                                    part_index,
                                });
                            }
                            // Mark which parts of the row were matched.  We highlight all rows,
                            // even those which aren't used in truth proving.
                            matches_per_part.push((part_index, matched_places));
                        }
                    }
                    // Now that we're not borrowing `exp_row` for the loop iterator, we can add the
                    // match patterns
                    for (part_index, matched_places) in matches_per_part {
                        let existing_counts = exp_row
                            .music_highlights
                            .entry(part_index)
                            .or_insert_with(|| vec![0; stage.num_bells()]);
                        for p in matched_places {
                            existing_counts[p] += 1;
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
            full::MusicGroup {
                name,
                max_count,
                inner: full::MusicGroupInner::Leaf { rows_matched },
            }
        }
        music::Music::Group(name, source_sub_groups) => {
            let (sub_groups, count, max_count) =
                expand_music_groups(&source_sub_groups, fragments, stage);
            full::MusicGroup {
                name: name.to_owned(),
                max_count,
                inner: full::MusicGroupInner::Group { count, sub_groups },
            }
        }
    }
}
