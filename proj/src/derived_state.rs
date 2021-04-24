use crate::spec::{CallSpec, MethodRef, MethodSpec, PartHeads, Spec};
use itertools::Itertools;
use proj_core::{run_len, Row, Stage};
use serde::Serialize;
use std::rc::Rc;
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

// Imports used only for the doc comments
#[allow(unused_imports)]
use crate::spec::Frag;

/* ========== UTIL STRUCTS TO LINK EXPANDED ROWS BACK TO THEIR LOCATIONS ========== */

/// A small datatype that represents **where** a given row comes from in the composition.  This is
/// useful because the composition contains many fragments, and each row of each fragment could
/// expand into multiple actual rows (one for each part).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct RowOrigin {
    /// The index of the part that this [`Row`] came from
    part: usize,
    /// The index of the fragment that this [`Row`] came from
    frag: usize,
    /// The index of the row within the fragment that this [`Row`] came from
    row: usize,
}

impl RowOrigin {
    /// Creates a new `RowOrigin` from its parts
    fn new(part: usize, frag: usize, row: usize) -> RowOrigin {
        RowOrigin { part, frag, row }
    }
}

/// A small datatype that represents **where** a given row comes from in the composition (without
/// knowledge of which part this comes from).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct RowLocation {
    /// The index of the fragment that this [`Row`] came from
    frag: usize,
    /// The index of the row within the fragment that this [`Row`] came from
    row: usize,
}

impl From<RowOrigin> for RowLocation {
    fn from(o: RowOrigin) -> Self {
        RowLocation {
            frag: o.frag,
            row: o.row,
        }
    }
}

/* ========== DERIVED STATE OF EACH ROW ========== */

/// A data structure to store a method splice label
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct MethodLabel {
    name: String,
    shorthand: String,
}

impl MethodLabel {
    pub fn new(name: String, shorthand: String) -> Self {
        MethodLabel { name, shorthand }
    }
}

/// A data structure to store a point on the comp where a call is labelled
#[derive(Debug, Clone)]
pub struct CallLabel {
    /// What label should this call be given in each part
    labels: Vec<String>,
    // This is not needed by JS code but is needed by `DerivedState` to generate statistics about
    // how calls are used.
    call_index: usize,
}

impl CallLabel {
    pub fn new<'a>(
        call_index: usize,
        notation: char,
        positions: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        CallLabel {
            labels: positions
                .into_iter()
                .map(|pos| format!("{}{}", notation, pos))
                .collect(),
            call_index,
        }
    }
}

/// The information required by JS in order to render a [`Fold`]
#[derive(Debug, Clone, Copy, Serialize)]
pub struct DerivedFold {
    is_open: bool,
}

impl DerivedFold {
    pub fn new(is_open: bool) -> Self {
        DerivedFold { is_open }
    }
}

/// A single expanded [`Row`] of the composition.  This corresponds to a single source [`Row`] from
/// the [`Spec`], but does **not** correspond to a single on-screen row because of the folding.
/// Because of this, we don't need to serialise this or send it to JS - we use [`DisplayRow`] for
/// that instead.
#[derive(Debug, Clone)]
pub struct ExpandedRow {
    call_label: Option<CallLabel>,
    method_label: Option<MethodLabel>,
    fold: Option<DerivedFold>,
    is_ruleoff: bool,
    is_proved: bool,
    is_leftover: bool,
    method_ref: Option<MethodRef>,
    /// One [`Row`] for each part of the composition
    rows: Vec<Row>,
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
    /// and (assuming that the music being scored is runs off either end of the row) the highlights
    /// would be:
    /// ```ignore
    /// vec![
    ///     vec![],
    ///     vec![1],
    ///     vec![0, 1],
    ///     vec![0, 1, 2],
    ///     vec![0, 1, 2, 3],
    ///     vec![0, 1, 2, 3],
    ///     vec![0, 1, 2, 3],
    ///     vec![0, 1, 2, 3]
    /// ]
    /// ```
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
                music[..run_len_f].iter_mut().for_each(|m| m.push(part));
            }
            // Highlight runs of >=4 bells of the **back**
            let run_len_b = run_len(r.bells().rev());
            if run_len_b >= 4 {
                // The 'max' prevents the two ranges from overlapping and causing music in multiple
                // runs from being counted twice
                music[(stage.as_usize() - run_len_b).max(run_len_f)..]
                    .iter_mut()
                    .for_each(|m| m.push(part));
            }
        }
        music
    }

    /// Create a new `ExpandedRow` from its constituent parts
    pub fn new(
        all_rows: Vec<Row>,
        call_label: Option<CallLabel>,
        method_str: Option<MethodLabel>,
        method_ref: Option<MethodRef>,
        fold: Option<DerivedFold>,
        is_ruleoff: bool,
        is_proved: bool,
        is_leftover: bool,
    ) -> Self {
        ExpandedRow {
            call_label,
            method_label: method_str,
            method_ref,
            fold,
            is_ruleoff,
            music_highlights: Self::calculate_music(&all_rows, all_rows[0].stage()),
            rows: all_rows,
            is_proved,
            is_leftover,
        }
    }

    /// Marks this `ExpandedRow` as a ruleoff
    pub fn set_ruleoff(&mut self) {
        self.is_ruleoff = true;
    }
}

/// All the information required for JS to render a single [`Row`] from the [`Spec`].  Note that
/// because of multipart expansion, this single on-screen [`Row`] actually represents many expanded
/// [`Row`]s, and this datatype reflects that.  Furthermore, this single [`DisplayRow`] may contain
/// a range of [`ExpandedRow`]s which are folded into this, which is again reflected by the fields.
#[derive(Serialize, Debug, Clone)]
struct DisplayRow {
    /// What call string should be displayed in each part (i.e. this is text that is displayed to
    /// the left of this Row).
    #[serde(skip_serializing_if = "crate::ser_utils::is_all_empty_string")]
    call_strings: Vec<String>,
    /// What method string should be displayed (i.e. this is text that is displayed to the right of
    /// this Row).
    #[serde(skip_serializing_if = "String::is_empty")]
    method_string: String,
    /// Which range of source Rows are 'folded into' this on-screen row
    range: Range<usize>,
    /// The foldedness status of this row
    #[serde(skip_serializing_if = "Option::is_none")]
    fold: Option<DerivedFold>,
    /// Where the user has enabled blueline rendering, should this row be displayed with bell names
    #[serde(skip_serializing_if = "crate::ser_utils::is_false")]
    use_bell_names: bool,
    /// `true` if this row should have a line rendered underneath it
    #[serde(skip_serializing_if = "crate::ser_utils::is_false")]
    is_ruleoff: bool,
    /// `true` if this row is used in the proving of the composition
    #[serde(skip_serializing_if = "crate::ser_utils::is_true")]
    is_proved: bool,
    /// One [`Row`] per part
    #[serde(serialize_with = "crate::ser_utils::ser_rows")]
    rows: Vec<Row>,
    /// See [`ExpandedRow::music_highlights`] for docs
    #[serde(skip_serializing_if = "crate::ser_utils::is_all_empty")]
    music_highlights: Vec<Vec<usize>>,
}

impl DisplayRow {
    fn from_range(expanded_rows: &[ExpandedRow], range: Range<usize>) -> Self {
        // Unpack useful values
        let slice = &expanded_rows[range.clone()];
        let first_exp_row = &slice[0];
        let num_parts = first_exp_row.rows.len();
        // Generate the call strings for each part
        let mut call_strings = vec![String::new(); num_parts];
        for call_label in slice.iter().filter_map(|r| r.call_label.as_ref()) {
            for (i, l) in call_label.labels.iter().enumerate() {
                call_strings[i].push_str(l);
            }
        }
        // Calculate how many method names are contained in this range.  We then have the cases:
        // This == 0: No method string is required (actually a special case of the next case)
        // This == 1: We display the full method name
        // This >= 2: We combine the calls and shorthands into a compact string (ala CompLib)
        // TODO: Make this count _any_ lead start/discontinuity, even if we're restarting the same
        // method.  Otherwise the lead summary strings won't be correct
        let num_method_labels = slice.iter().filter(|r| r.method_label.is_some()).count();
        // Create the displayed row
        DisplayRow {
            call_strings,
            method_string: match num_method_labels {
                0 | 1 => slice
                    .iter()
                    .filter_map(|r| r.method_label.as_ref())
                    .map(|l| &l.name)
                    .join(""),
                _ => unimplemented!(),
            },
            range,
            // All DisplayRows start using bell names.  This is then set to false for all rows
            // covered by a `LineRange`
            use_bell_names: true,
            // This should be a ruleoff iff the last row in the range was a ruleoff
            is_ruleoff: slice.last().unwrap().is_ruleoff,
            is_proved: first_exp_row.is_proved,
            fold: first_exp_row.fold,
            rows: first_exp_row.rows.clone(),
            music_highlights: first_exp_row.music_highlights.clone(),
        }
    }
}

/* ========== DERIVED STATE OF FRAGMENTS (AND THEIR LINKS) ========== */

/// A range of rows which should be highlighted as all false in the same way.
#[derive(Serialize, Debug, Clone)]
struct FalseRowRange {
    #[serde(flatten)]
    range: Range<usize>,
    group: usize,
}

/// A struct that says that [`Frag`] #`to` can be linked onto the end of [`Frag`] #`from`.  This
/// will be stored in a `Vec`, representing a non-symmetric relation over the [`Frag`]s in the
/// composition.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FragLink {
    from: usize,
    to: usize,
    group: usize,
}

/// A struct determining which linking groups the top and bottom of a [`Frag`] belongs to.  This
/// will determine what colour line will be displayed on each end of the [`Frag`], to make round
/// blocks detectable.
#[derive(Serialize, Debug, Clone, Default)]
struct FragLinkGroups {
    #[serde(skip_serializing_if = "Option::is_none")]
    top: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bottom: Option<usize>,
}

/// A section of the composition where blue-lines should be rendered
#[derive(Serialize, Debug, Clone)]
struct LineRange {
    /// The row which appears just off the top of this range.  Contains one [`Row`] per part, and
    /// is [`None`] if there is no [`Row`] before this range.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "crate::ser_utils::ser_opt_rows")]
    top_rows: Option<Vec<Row>>,
    /// The row which appears just off the bottom of this range.  Contains one [`Row`] per part,
    /// and is [`None`] if there is no [`Row`] after this range.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "crate::ser_utils::ser_opt_rows")]
    bottom_rows: Option<Vec<Row>>,
    #[serde(flatten)]
    range: Range<usize>,
}

/// The information required for JS to render a [`Frag`]
#[derive(Serialize, Debug, Clone)]
struct DerivedFrag {
    /* Row data */
    /// A full list of the expanded rows generated by this [`Frag`], including those which don't
    /// appear on-screen
    #[serde(skip)]
    expanded_rows: Vec<ExpandedRow>,
    #[serde(rename = "rows")]
    display_rows: Vec<DisplayRow>,
    /* Misc data */
    /// Ranges on the display where bell lines should be rendered, along with the rows on either
    /// side of these ranges (which allow the lines to look like they're connecting to hidden
    /// rows).
    false_row_ranges: Vec<FalseRowRange>,
    link_groups: FragLinkGroups,
    line_ranges: Vec<LineRange>,
    is_proved: bool,
    x: f32,
    y: f32,
}

/* ========== DERIVED STATE NOT SPECIFIC TO EACH FRAGMENTS ========== */

/// The derived state of a single method definition.
#[derive(Debug, Clone, Serialize)]
struct DerivedMethod {
    name: String,
    shorthand: String,
    place_not_string: String,
    num_proved_rows: usize,
    num_rows: usize,
}

impl From<&MethodSpec> for DerivedMethod {
    fn from(method: &MethodSpec) -> Self {
        DerivedMethod {
            name: method.name(),
            shorthand: method.shorthand(),
            place_not_string: method.place_not_string().to_owned(),
            num_proved_rows: 0,
            num_rows: 0,
        }
    }
}

/// The derived state of a single call definition
#[derive(Debug, Clone, Serialize)]
pub struct DerivedCall {
    notation: char,
    location: String,
    count: usize,
    proved_count: usize,
}

impl DerivedCall {
    /// Creates a new `DerivedCall` with no instances.
    pub fn new(notation: char, location: String) -> Self {
        DerivedCall {
            notation,
            location,
            count: 0,
            proved_count: 0,
        }
    }
}

/// General statistics about the composition, to be displayed in the top-left corner of the screen
#[derive(Serialize, Debug, Clone)]
struct DerivedStats {
    part_len: usize,
    num_false_rows: usize,
    num_false_groups: usize,
}

/* ========== FULL DERIVED STATE ========== */

/// The full `DerivedState` of a composition.  This is (almost) the only information that the
/// JavaScript rendering code gets access to - every time the viewed [`Spec`] is updated, a new
/// `DerivedState` is built then serialised to JSON (which is then deserialised into a native JS
/// object).  Essentially this means that JavaScript's `derived_state` global variable is a
/// read-only copy of `DerivedState`, complete with private fields.
#[derive(Serialize, Debug, Clone)]
pub struct DerivedState {
    frags: Vec<DerivedFrag>,
    frag_links: Vec<FragLink>,
    stats: DerivedStats,
    part_heads: Rc<PartHeads>,
    methods: Vec<DerivedMethod>,
    calls: Vec<DerivedCall>,
    stage: usize,
}

impl DerivedState {
    /// Given a [`Spec`]ification, derive a new `DerivedState` from it.
    pub fn from_spec(spec: &Spec) -> DerivedState {
        // PERF: This whole function could be improved by reusing the storage from an existing
        // `DerivedState` rather than creating a new one fully from scratch

        // Fully expand the comp from the [`Spec`]
        let (generated_rows, part_heads, methods, calls) = spec.expand();

        // Truth proving pipeline
        let (flat_proved_rows, part_len) = flatten_proved_rows(&generated_rows, spec.len());
        let (false_rows, num_false_rows) = gen_false_row_groups(flat_proved_rows);
        let (mut ranges_by_frag, num_false_groups) = coalesce_false_row_groups(false_rows);

        // Determine how the frags link together
        let (frag_links, frag_link_groups) = gen_frag_links(&generated_rows, &part_heads);

        // Derive stats about the methods and calls
        let der_methods = derive_methods(methods, &generated_rows);
        let der_calls = derive_calls(calls, &generated_rows);

        // Compile all of the derived state into one struct
        assert_eq!(frag_link_groups.len(), generated_rows.len());
        DerivedState {
            frag_links,
            part_heads,
            frags: generated_rows
                .into_iter()
                .zip(frag_link_groups.into_iter())
                .enumerate()
                .map(|(i, (expanded_rows, link_groups))| {
                    // Sanity check that leftover rows should never be used in the proving
                    assert!(expanded_rows.last().map_or(false, |r| !r.is_proved));
                    // Unpack/derive useful data about this Frag
                    let (x, y) = spec.frag_pos(i).unwrap();
                    let fold_regions = get_fold_ranges(&expanded_rows);
                    let line_ranges = get_line_ranges(&fold_regions, &expanded_rows);
                    // Calculate which rows should be displayed to the user
                    let mut display_rows: Vec<DisplayRow> = fold_regions
                        .into_iter()
                        .map(|r| DisplayRow::from_range(&expanded_rows, r))
                        .collect();
                    for l in &line_ranges {
                        // Prevent JS from drawing coloured bell names where there are line ranges
                        for r in &mut display_rows[l.range.clone()] {
                            r.use_bell_names = false;
                        }
                    }
                    // Combine all this data into a single struct
                    DerivedFrag {
                        false_row_ranges: ranges_by_frag.remove(&i).unwrap_or_default(),
                        display_rows,
                        line_ranges,
                        expanded_rows,
                        is_proved: !spec.is_frag_muted(i).unwrap(),
                        link_groups,
                        x,
                        y,
                    }
                })
                .collect(),
            stats: DerivedStats {
                part_len,
                num_false_groups,
                num_false_rows,
            },
            methods: der_methods,
            calls: der_calls,
            stage: spec.stage().as_usize(),
        }
    }

    /// Gets the [`Row`] at a given location in this `DerivedState`, returning `None` if the
    /// location doesn't correspond to a [`Row`].
    pub fn get_row(&self, part_ind: usize, frag_ind: usize, row_ind: usize) -> Option<&Row> {
        Some(
            self.frags
                .get(frag_ind)?
                .expanded_rows
                .get(row_ind)?
                .rows
                .get(part_ind)?,
        )
    }

    /// Returns `true` if the specified method is used anywhere in the composition (even muted).
    /// Returns `None` if the method index was out of range.
    #[inline]
    pub fn is_method_used(&self, method_ind: usize) -> Option<bool> {
        self.methods.get(method_ind).map(|m| m.num_rows > 0)
    }

    /// Gets the part head at a given index, or returning `None` if the index is bigger than the
    /// number of parts.
    #[inline]
    pub fn get_part_head(&self, part_ind: usize) -> Option<&Row> {
        self.part_heads.rows().get(part_ind)
    }

    /// Maps an on-screen row location to the row index in the fragment's unfolded form
    pub fn source_row_ind(&self, frag_ind: usize, row_ind: usize) -> usize {
        self.frags[frag_ind].display_rows[row_ind].range.start
    }
}

/* ========== HELPER FUNCTIONS FOR Spec -> DerivedState CONVERSION ========== */

/// Given the expanded rows from each [`Frag`] of the composition, find which pairs of
/// [`Frag`]s should be connected (i.e. [`Frag`]s (x, y) are linked x -> y iff the leftover row
/// of x is the same as the first row of y).  This is then used to determine which [`Frag`]s
/// can be joined together.  This also calculates which groups the top and bottom of each
/// [`Frag`] belongs to.
fn gen_frag_links(
    generated_rows: &[Vec<ExpandedRow>],
    part_heads: &PartHeads,
) -> (Vec<FragLink>, Vec<FragLinkGroups>) {
    let num_frags = generated_rows.len();
    // A map to determine which group ID should be assigned to each Row.  This way,
    // interconnected groups of links are given the same colour.
    let mut link_groups: HashMap<&Row, usize> = HashMap::new();
    let mut frag_links = Vec::new();
    let mut frag_link_groups = vec![FragLinkGroups::default(); num_frags];

    // Test every pair of frags f -> g ...
    for (i, f) in generated_rows.iter().enumerate() {
        for (j, g) in generated_rows.iter().enumerate() {
            // ... if `g` starts with the leftover row of `f`, then f -> g ...
            let leftover_row_of_f = &f.last().unwrap().rows[0];
            let first_row_of_g = &g[0].rows[0];
            if part_heads
                .are_equivalent(leftover_row_of_f, first_row_of_g)
                .unwrap()
            {
                // Decide what group this link should be put in (so that all the links of the
                // same row get coloured the same colour).
                let link_groups_len = link_groups.len();
                let group = *link_groups
                    .entry(leftover_row_of_f)
                    .or_insert(link_groups_len);
                // Add the frag links, and assign the frag tip colours
                frag_links.push(FragLink {
                    from: i,
                    to: j,
                    group,
                });
                frag_link_groups[i].bottom = Some(group);
                frag_link_groups[j].top = Some(group);
            }
        }
    }
    (frag_links, frag_link_groups)
}

/// Take a jagged array of `ExpandedRow`s, and return all the [`Row`]s that should be
/// proven, along with their origin.  This also returns the number of proven rows from each
/// part.  This does **not** sort the flattened rows.
fn flatten_proved_rows(
    generated_rows: &[Vec<ExpandedRow>],
    spec_len: usize,
) -> (Vec<(RowOrigin, &Row)>, usize) {
    // Expand all the rows and their origins from the composition into a `Vec` to be
    // proved, excluding the last Row of each Frag, since that is 'left over' and as such
    // shouldn't be used of proving
    let mut flattened_rows: Vec<(RowOrigin, &Row)> = Vec::with_capacity(spec_len);
    let mut part_len = 0;
    for (frag_index, rows) in generated_rows.iter().enumerate() {
        for (row_index, expanded_row) in rows.iter().filter(|r| r.is_proved).enumerate() {
            for (part_index, row) in expanded_row.rows.iter().enumerate() {
                flattened_rows.push((RowOrigin::new(part_index, frag_index, row_index), row));
            }
            // Count the single ExpandedRow as one row per part (despite it expanding to
            // several individual Rows)
            part_len += 1;
        }
    }
    (flattened_rows, part_len)
}

/// Given the flattened rows of a composition, group the rows on the screen into false groups
/// (note that these are groups of individual rows which are the same, rather than the
/// 'meta-groups' that the user sees).  `spec_len` is used to make sure that we allocate
/// exactly the right amount of space when flattening the rows
fn gen_false_row_groups(
    mut flattened_rows: Vec<(RowOrigin, &Row)>,
) -> (Vec<Vec<RowLocation>>, usize) {
    // Sort all_rows only by their rows, so that false rows are appear next to each other.  The
    // algorithm won't work unless the input rows are sorted.
    flattened_rows.sort_by(|(_, r1), (_, r2)| r1.cmp(r2));

    // We use a hashset because if the part heads form a group then any falseness will be the
    // same between all the parts, so will appear lots of times.
    let mut false_rows: HashSet<Vec<RowLocation>> = HashSet::new();
    // The origins of the current set of duplicated rows.  Most of the time, we hope that
    // this has length 1, i.e. all rows are unique.
    let mut current_false_row_group: Vec<RowLocation> = Vec::with_capacity(10);
    let mut last_row = None;
    let mut num_false_rows = 0usize;
    // Iterate over all the rows, compiling groups as we go
    for (o, r) in flattened_rows.iter() {
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
        current_false_row_group.push(RowLocation::from(*o));
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
    // Convert the HashMap into a Vec (without cloning), and return it along with the number of
    // false rows
    (false_rows.into_iter().collect::<Vec<_>>(), num_false_rows)
}

/// Combine adjacent false row groups so that we use up fewer colours.  This relies on the
/// fact that all the `Vec`s in `false_rows` are sorted in increasing order by frag index and
/// then row index (and a unit test checks that).
fn coalesce_false_row_groups(
    mut false_rows: Vec<Vec<RowLocation>>,
) -> (HashMap<usize, Vec<FalseRowRange>>, usize) {
    let mut ranges_by_frag: HashMap<usize, Vec<FalseRowRange>> = HashMap::new();
    // Firstly, convert the existing HashSet into a Vec, and sort it
    false_rows.sort();
    // Because `false_rows_vec` itself and all its contents are sorted, we can guaruntee
    // that the rows making up each false row group are sequential in the listing
    let mut iter = false_rows.iter();
    let mut group_id = 0;
    if let Some(first_group) = iter.next() {
        let mut last_group = first_group;
        let mut first_group_in_meta_group = first_group;
        for group in iter {
            // Decide if this group is adjacent to the last one (for two groups to be
            // adjacent, they need to have the same length and all the `RowLocation`s must
            // also be adjacent -- we don't worry about the order of each group because
            // they have all been sorted the same way so a simple zipping check will
            // suffice).
            let is_adjacent_to_last = group.len() == last_group.len()
                && group.iter().zip(last_group.iter()).all(|(loc1, loc2)| {
                    loc1.frag == loc2.frag && (loc1.row as isize - loc2.row as isize).abs() == 1
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
    // The final value of group_id is also the number of falseness groups, so return it out
    // of this block along with the ranges we calculated
    (ranges_by_frag, group_id + 1)
}

/// A cheeky helper function which adds the ranges between two groups of false rows to
/// the right places in a HashMap (the map will only ever be `row_groups_by_frag`)
fn add_ranges(
    ranges_per_frag: &mut HashMap<usize, Vec<FalseRowRange>>,
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
            // The `+ 1` makes sure that the range represents `start_loc..=end_loc`
            range: start_loc.row.min(end_loc.row)..start_loc.row.max(end_loc.row) + 1,
            group: group_id,
        };
        // Insert the newly created group to the HashMap to make sure it's displayed on
        // the correct fragment
        ranges_per_frag
            .entry(start_loc.frag)
            .or_insert_with(|| Vec::with_capacity(0))
            .push(false_row_range);
    }
}

/// Derive statistics about each [`Method`] using the [`ExpandedRow`]s of the composition
fn derive_methods(methods: &[Rc<MethodSpec>], exp_rows: &[Vec<ExpandedRow>]) -> Vec<DerivedMethod> {
    // Initialise list of empty methods (which are indexed in the same order as the original
    // methods list
    let mut der_methods: Vec<DerivedMethod> = methods
        .iter()
        .map(|m| DerivedMethod::from(m.as_ref()))
        .collect();
    // Fill in these methods with stats from the derived rows
    for frag_rows in exp_rows {
        for exp_row in frag_rows {
            if let Some(method_ref) = exp_row.method_ref {
                // For each row that has a method, increment that method's row counter by the
                // number of output rows that this row got expanded to.  This is used to make sure
                // that the user doesn't delete methods that are used somewhere in the composition
                // (but their 'row counter' might show 0 because the method only exists in muted
                // fragments).
                der_methods[method_ref.method_index()].num_rows += exp_row.rows.len();
                // If this row was proved, then increment num_proved_rows too.
                if exp_row.is_proved {
                    der_methods[method_ref.method_index()].num_proved_rows += exp_row.rows.len();
                }
            }
        }
    }
    // Return the methods
    der_methods
}

/// Derive statistics about each [`Call`] using the [`ExpandedRow`]s of the composition
fn derive_calls(calls: &[Rc<CallSpec>], exp_rows: &[Vec<ExpandedRow>]) -> Vec<DerivedCall> {
    // Initialise a set of calls with no instances
    let mut der_calls: Vec<DerivedCall> = calls
        .iter()
        .map(|call_spec| call_spec.to_derived_call())
        .collect();
    // Count instances by iterating over all the expanded rows
    for frag_rows in exp_rows {
        for exp_row in frag_rows {
            if let Some(CallLabel { call_index, .. }) = exp_row.call_label {
                // If this expanded row contains a call, then each part will contain the same call.
                // So we can update the counter once without looking at the rows directly.
                // `exp_row.rows.len()` will always be equal to the number of parts, but that isn't
                // accessible here so this is more convenient (and doesn't require any pointer
                // look-ups)
                der_calls[call_index].count += exp_row.rows.len();
                if exp_row.is_proved {
                    der_calls[call_index].proved_count += exp_row.rows.len();
                }
            }
        }
    }
    // Return the `DerivedCall`s
    der_calls
}

/// Detect which regions of [`Row`]s will appear under each line on the screen (i.e. each [`Range`]
/// in the output will correspond to exactly one line on the user's screen, but could contain more
/// [`ExpandedRow`]s if it corresponds to a folded region).
fn get_fold_ranges(exp_rows: &[ExpandedRow]) -> Vec<Range<usize>> {
    let mut ranges = Vec::with_capacity(exp_rows.len());
    let mut current_range_start: Option<usize> = None;
    for (i, r) in exp_rows.iter().enumerate() {
        // The leftover row is always open, and can never be folded into another group
        if r.is_leftover {
            // We've hit a new fold region, so we need to add the last region (if needed)
            if let Some(start) = current_range_start {
                ranges.push(start..i);
            }
            ranges.push(i..i + 1);
            current_range_start = None;
            continue;
        }
        match r.fold {
            Some(f) => {
                // We've hit a new fold region, so we need to add the last region (if needed)
                if let Some(start) = current_range_start {
                    ranges.push(start..i);
                }
                // Now make sure to set current_range_start for the region starting with this row
                if f.is_open {
                    ranges.push(i..i + 1);
                    current_range_start = None;
                } else {
                    current_range_start = Some(i);
                }
            }
            None => {
                if current_range_start.is_none() {
                    ranges.push(i..i + 1);
                }
            }
        }
    }
    ranges
}

/// Find out which regions of the fragment should be drawn with bluelines.
fn get_line_ranges(fold_ranges: &[Range<usize>], exp_rows: &[ExpandedRow]) -> Vec<LineRange> {
    let mut current_range_start: Option<usize> = None;
    let mut last_index_and_range: Option<usize> = None;
    let mut line_ranges = Vec::new();
    // Iterate over the fold ranges and split the blueline intervals whenever there are points of
    // discontinuity
    for (i, r) in fold_ranges.iter().enumerate() {
        if let Some(cur_range_start) = current_range_start {
            let is_continuous = last_index_and_range.map_or(false, |li| li + 1 == r.start);
            let range_length = i - cur_range_start;
            if !is_continuous {
                // Only generate ranges which have length more than one (since there's no point
                // drawing bluelines for one single row)
                if range_length > 1 {
                    line_ranges.push(LineRange {
                        top_rows: fold_ranges[cur_range_start]
                            .start
                            .checked_sub(1)
                            .and_then(|i| exp_rows.get(i))
                            .map(|exp_row| exp_row.rows.clone()),
                        bottom_rows: exp_rows
                            .get(last_index_and_range.unwrap() + 1)
                            .map(|exp_row| exp_row.rows.clone()),
                        range: cur_range_start..i,
                    });
                }
                current_range_start = Some(i);
            }
        } else {
            current_range_start = Some(i);
        }
        last_index_and_range = Some(r.start);
    }
    // Make sure to keep the last range (which won't have any discontinuity to finish it)
    if let Some(cur_range_start) = current_range_start {
        // TODO: Combine this and the version in the loop into one macro to avoid code duplication
        let range_length = fold_ranges.len() - cur_range_start;
        if range_length > 1 {
            line_ranges.push(LineRange {
                top_rows: fold_ranges[cur_range_start]
                    .start
                    .checked_sub(1)
                    .and_then(|i| exp_rows.get(i))
                    .map(|exp_row| exp_row.rows.clone()),
                bottom_rows: None,
                range: cur_range_start..fold_ranges.len(),
            });
        }
    }
    line_ranges
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
        assert!(rl(0, 1) < rl(0, 3));

        assert!(rl(2, 1) > rl(1, 3));
        assert!(rl(1, 0) > rl(0, 100));
    }
}
