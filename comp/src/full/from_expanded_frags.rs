// This lint gives false positives for raw pointers (which are hashed by the memory address they
// point to).  See https://github.com/rust-lang/rust-clippy/issues/6745
#![allow(clippy::mutable_key_type)]

use std::{collections::HashMap, rc::Rc};

use bellframe::Stage;
use itertools::Itertools;
use jigsaw_utils::types::{FragSlice, FragVec, MethodIdx, MethodSlice, RowVec};

use crate::{
    expanded_frag::ExpandedFrag,
    full, music,
    spec::{self, part_heads::PartHeads},
    FullState,
};

use super::Stats;

/// Mapping from [`spec::Method`] to both indices and [`full::Method`]s, where the source methods
/// are hashed by their memory address (so two distinct but identical methods would be hashed
/// differently).
type MethodMap = HashMap<*const spec::Method, (MethodIdx, full::Method)>;

pub(super) fn from_expanded_frags(
    expanded_frags: FragVec<ExpandedFrag>,
    spec_methods: &MethodSlice<Rc<spec::Method>>,
    part_heads: Rc<PartHeads>,
    music: &[music::Music],
    stage: Stage,
) -> FullState {
    let method_map = expand_methods(spec_methods, &expanded_frags, part_heads.len());
    let stats = generate_stats(&expanded_frags);
    let (music, frag_musics) = music_gen::compute_music(music, &expanded_frags, stage);
    let fragments = annotate_frags(expanded_frags, frag_musics);

    FullState {
        part_heads,
        fragments,
        // Make sure that the methods are sorted by their index.  Otherwise, the `HashMap`
        // non-determinism will make the methods change order every time the `FullState` is built.
        methods: method_map
            // TODO: In Rust `1.54`+ we can use `into_values()`
            .into_iter()
            .map(|(_k, v)| v)
            .sorted_by_key(|(idx, _m)| *idx)
            .map(|(_idx, m)| m)
            .collect(),
        music,
        stats,
        stage,
    }
}

fn expand_methods(
    methods: &MethodSlice<Rc<spec::Method>>,
    frags: &FragSlice<ExpandedFrag>,
    num_parts: usize,
) -> MethodMap {
    // Maps source methods [`spec::Method`] (hashed by their memory addresses) to the expanded
    // [`full::Method`].  This is used so that the fragment expansion, which receives rows
    // containing `Rc<spec::Method>` can know which `full::Method` it corresponds to (so its row
    // counters can be updated).
    let mut method_map = methods
        .iter_enumerated()
        .map(|(idx, m)| {
            let source_ptr = m.as_ref() as *const spec::Method;
            let expanded_method = full::Method {
                source: m.clone(),
                // Will be accumulated later
                num_rows: 0,
                num_proved_rows: 0,
            };
            (source_ptr, (idx, expanded_method))
        })
        .collect::<HashMap<_, _>>();

    // Iterate through all the fragments, and count up how many rows (proven or muted) are
    // generated by each method
    for f in frags {
        for row_data in &f.row_data {
            if let Some((spec_method, _)) = &row_data.method_source {
                let spec_method_ptr = spec_method.as_ref() as *const spec::Method;
                let (_idx, annot_method) = method_map
                    .get_mut(&spec_method_ptr)
                    .expect("Row owned by an unlisted method");
                // This single `row_data` corresponds to one row for each part so, accordingly, we
                // update the counters by multiples of `num_parts`.
                annot_method.num_rows += num_parts;
                if row_data.is_proved {
                    annot_method.num_proved_rows += num_parts;
                }
            }
        }
    }

    method_map
}

fn generate_stats(frags: &FragSlice<ExpandedFrag>) -> Stats {
    // The total length of a part is the sum of the lengths of fragments
    let part_len = frags.iter().map(|f| f.len()).sum();
    Stats { part_len }
}

////////////////////
// MUSIC COUNTING //
////////////////////

mod music_gen {
    use std::rc::Rc;

    use bellframe::Stage;
    use index_vec::index_vec;
    use itertools::Itertools;
    use jigsaw_utils::types::{FragSlice, FragVec, PartVec, RowIdx, RowLocation};

    use crate::{expanded_frag::ExpandedFrag, full, music};

    pub(super) fn compute_music(
        music: &[music::Music],
        expanded_frags: &FragSlice<ExpandedFrag>,
        stage: Stage,
    ) -> (full::Music, FragVec<FragMusic>) {
        // Create a set of `FragMusic`s per part, who's counters will be incremented whilst computing
        // the music
        let mut frag_musics: FragVec<FragMusic> = expanded_frags
            .iter()
            .map(|frag| FragMusic::all_counters_zero(frag, stage))
            .collect();
        let (groups, total_count, max_count) =
            expand_music_groups(music, expanded_frags, &mut frag_musics, stage);

        let music = full::Music {
            groups,
            total_count,
            max_count,
        };
        (music, frag_musics)
    }

    /// Recursively expand a sequence of music groups, totalling the number of occurrences
    fn expand_music_groups(
        music: &[music::Music],
        expanded_frags: &FragSlice<ExpandedFrag>,
        frag_musics: &mut FragSlice<FragMusic>,
        stage: Stage,
    ) -> (Vec<Rc<full::MusicGroup>>, usize, usize) {
        // Expand groups individually
        let music_groups = music
            .iter()
            .map(|m| expand_music_group(m, expanded_frags, frag_musics, stage))
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
        expanded_frags: &FragSlice<ExpandedFrag>,
        frag_musics: &mut FragSlice<FragMusic>,
        stage: Stage,
    ) -> full::MusicGroup {
        match group {
            music::Music::Regex(name, regex) => {
                // Compute where this `Regex` is matched in the composition
                let mut rows_matched = Vec::<RowLocation>::new();
                // For each fragment ...
                for ((frag_index, expanded_frag), frag_music) in
                    expanded_frags.iter_enumerated().zip_eq(frag_musics)
                {
                    // ... for each part ...
                    for ((part_index, rows), part_music_counters) in expanded_frag
                        .rows_per_part
                        .iter_enumerated()
                        .zip_eq(&mut frag_music.music_highlights_per_part)
                    {
                        // ... for each row ...
                        //
                        // PERF: This whole calculation can probably be done in one vectorised pass
                        for (row_index, ((row, music_counters), row_data)) in rows
                            .iter()
                            .zip_eq(part_music_counters.chunks_mut(stage.num_bells()))
                            .zip_eq(&expanded_frag.row_data)
                            .enumerate()
                        {
                            // Sanity check that all the elements are the same length.  The code
                            // will likely panic anyway, but this assertion is easier to debug
                            assert_eq!(music_counters.len(), stage.num_bells());
                            // ... if the row matches this music pattern ...
                            if let Some(matched_places) = regex.match_pattern(row) {
                                // ... mark the row's places as highlight-able
                                for matched_place in matched_places {
                                    let counter = &mut music_counters[matched_place];
                                    match counter.checked_add(1) {
                                        // No problem if the counter didn't overflow
                                        Some(v) => *counter = v,
                                        None => {
                                            eprintln!("WARNING: A place is matched by more than 255 music scores, clamping value to 255");
                                            // Don't write to the counter, because its value is
                                            // already 255
                                        }
                                    }
                                }
                                // ... and if the row is proved, include this row's location in the
                                // music group
                                if row_data.is_proved {
                                    rows_matched.push(RowLocation {
                                        frag_index,
                                        row_index: RowIdx::new(row_index),
                                        part_index,
                                    });
                                }
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
                // For a music group, expand the sub-groups in turn and total the match counts
                let (sub_groups, count, max_count) =
                    expand_music_groups(&source_sub_groups, expanded_frags, frag_musics, stage);
                full::MusicGroup {
                    name: name.to_owned(),
                    max_count,
                    inner: full::MusicGroupInner::Group { count, sub_groups },
                }
            }
        }
    }

    /// The music annotations for a single [`Fragment`]
    #[derive(Debug, Clone)]
    pub(super) struct FragMusic {
        /// For each part, how many leaf music groups match each place in the [`Fragment`].  We use
        /// `u8`s here, because I find it highly unlikely that we will be able to include a single
        /// place in more than 255 different music classes.  If we do manage that, the code will
        /// gracefully print a warning and saturate the value at 255.
        pub(super) music_highlights_per_part: PartVec<Vec<u8>>,
    }

    impl FragMusic {
        fn all_counters_zero(frag: &ExpandedFrag, stage: Stage) -> Self {
            let num_parts = frag.rows_per_part.len();
            Self {
                music_highlights_per_part: {
                    // For each part ...
                    index_vec![
                        // ... for each place, we initialise the counters to 0
                        vec![0u8; frag.row_data.len() * stage.num_bells()];
                        num_parts
                    ]
                },
            }
        }
    }
}

fn annotate_frags(
    expanded_frags: FragVec<ExpandedFrag>,
    frag_music: FragVec<music_gen::FragMusic>,
) -> FragVec<full::Fragment> {
    expanded_frags
        .into_iter()
        .zip(frag_music)
        .map(|(exp_frag, music)| expand_frag(exp_frag, music))
        .collect()
}

fn expand_frag(exp_frag: ExpandedFrag, music: music_gen::FragMusic) -> full::Fragment {
    // Generate `row_data` elements, with some fields ready to be filled in later
    let mut row_data: RowVec<full::RowData> = exp_frag
        .row_data
        .iter()
        .map(|row_data| full::RowData {
            ruleoff_above: false, // Set later in this function
            is_proved: row_data.is_proved,
        })
        .collect();

    for ((prev_row, row), full_row) in exp_frag
        .row_data
        .iter()
        .tuple_windows()
        .zip_eq(row_data.iter_mut().skip(1))
    {
        // This unwrap is safe, because `prev_row` can't be the last element of `exp_frag.row_data`
        // (because otherwise `row` wouldn't exist).  Therefore, `prev_row` can't be leftover, and
        // must belong to a method.
        let (prev_meth, prev_sub_lead_idx) = prev_row.method_source.as_ref().unwrap();

        // Set ruleoff it this method has a ruleoff at this index (usually because this row is
        // a lead **end**, or a six end in e.g. Stedman).
        if prev_meth.is_ruleoff_below(*prev_sub_lead_idx) {
            // The GUI draws ruleoffs **above** rows, but in order to allow ruleoffs to be
            // rendered above the leftover row, we detect ruleoffs using the previous row then
            // write them to the row below that
            full_row.ruleoff_above = true;
        }
        // Set a ruleoff **and a new method name** if there is:
        // a) a method splice: two adjacent rows which belong to different methods
        // b) a discontinuity: two adjacent rows cause the ringing to jump to a new place in a lead.
        if let Some((meth, sub_lead_idx)) = &row.method_source {
            if Rc::ptr_eq(prev_meth, meth) {
                // Methods are the same, so check for discontinuity
                let expected_sub_lead_idx = (*prev_sub_lead_idx + 1) % meth.lead_len();
                if *sub_lead_idx != expected_sub_lead_idx {
                    full_row.ruleoff_above = true;
                }
            } else {
                // Methods are different, so this is a method splice
                full_row.ruleoff_above = true;
            }
        }
    }

    full::Fragment {
        position: exp_frag.position,
        rows_per_part: exp_frag.rows_per_part,
        music_highlights_per_part: music.music_highlights_per_part,
        row_data,
    }
}
