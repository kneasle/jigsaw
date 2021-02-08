use itertools::Itertools;
use proj_core::{Bell, Row};
use wasm_bindgen::prelude::*;

fn clone_or_empty(string: &Option<String>) -> String {
    match string {
        Some(x) => x.clone(),
        None => "".to_owned(),
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AnnotatedRow {
    is_lead_end: bool,
    method_str: Option<String>,
    call_str: Option<String>,
    row: Row,
}

#[wasm_bindgen]
impl AnnotatedRow {
    /// Creates an [`AnnotatedRow`] representing a given [`Row`] with no annotations
    pub fn unannotated(row: Row) -> AnnotatedRow {
        AnnotatedRow {
            is_lead_end: false,
            method_str: None,
            call_str: None,
            row,
        }
    }

    /// Returns this [`Row`] without any annotations
    pub fn row(&self) -> Row {
        self.row.clone()
    }

    /// The number of [`Bell`]s in this `Row`
    pub fn len(&self) -> usize {
        self.row.slice().len()
    }

    /// Returns the [`Bell`]s that make up this row, as [`Vec`] of 0-indexed integers
    pub fn bells_indices(&self) -> Vec<usize> {
        self.row.slice().iter().copied().map(Bell::index).collect()
    }

    /// Returns a [`String`] that should be used to signal this `Row` as the start of a 'lead'
    pub fn method_str(&self) -> String {
        clone_or_empty(&self.method_str)
    }

    /// Returns a [`String`] that should annotate this row as a call
    pub fn call_str(&self) -> String {
        clone_or_empty(&self.call_str)
    }

    /// Returns `true` if this `AnnotatedRow` should have a line rendered underneath it
    pub fn is_ruleoff(&self) -> bool {
        self.is_lead_end
    }

    /// Returns the music highlighting layout for this row, with each [`bool`] in the [`Vec`]
    /// deciding whether or not that bell is part of music
    pub fn highlights(&self) -> Vec<usize> {
        /// Helper function which calculates the length of the longest run taken from an iterator
        /// of bells
        fn run_len(iter: impl Iterator<Item = Bell>) -> usize {
            let pairs: itertools::TupleWindows<_, (Bell, Bell)> = iter.tuple_windows();
            pairs
                .take_while(|(x, y)| (x.index() as isize - y.index() as isize).abs() == 1)
                .count()
                + 1
        }
        let mut highlights = vec![0; self.len()];
        // Highlight >=4 bell runs off the front
        let run_len_front = run_len(self.row.iter());
        if run_len_front >= 4 {
            for i in 0..run_len_front {
                highlights[i] = 1;
            }
        }
        // Highlight >=4 bell runs off the front
        let run_len_back = run_len(self.row.iter().rev());
        if run_len_back >= 4 {
            for i in 0..run_len_back {
                highlights[self.len() - 1 - i] = 1;
            }
        }
        // Return the highlights
        highlights
    }

    /// Returns the ranges of the row that should be highlighted.  These are 0-indexed and
    /// act the same way as `..` in Rust, so the result `[0, 4, 5, 10]` would highlight the first
    /// and last 4 bells in a row of Royal.  This is used by the rendering code to avoid lines
    /// between the individual rectangles under each bell.
    pub fn highlight_ranges(&self) -> Vec<usize> {
        let mut last_highlighted = 0;
        let mut ranges = Vec::new();
        for (i, &h) in self.highlights().iter().enumerate() {
            if h != last_highlighted {
                ranges.push(i);
            }
            last_highlighted = h;
        }
        if last_highlighted != 0 {
            ranges.push(self.len());
        }
        ranges
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct Frag {
    rows: Vec<AnnotatedRow>,
    pub x: f32,
    pub y: f32,
}

#[wasm_bindgen]
impl Frag {
    /// Get the row at a given index
    #[inline]
    pub fn get_row(&self, index: usize) -> AnnotatedRow {
        self.rows[index].clone()
    }

    /// The number of rows in this fragment
    #[inline]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Generates an example fragment (in this case, it's https://complib.org/composition/75822)
    pub fn example() -> Frag {
        let mut rows: Vec<_> = include_str!("example-comp")
            .lines()
            .map(|x| Row::parse(x).unwrap())
            .map(AnnotatedRow::unannotated)
            .collect();
        /* ANNOTATIONS */
        // Method names and LE ruleoffs
        let method_names = [
            "Deva",
            "Bristol",
            "Lessness",
            "Yorkshire",
            "Cooktown Orchid",
            "Superlative",
            "Cornwall",
            "Bristol",
        ];
        for i in 0..rows.len() / 32 {
            rows[i * 32].method_str = Some(method_names[i].to_owned());
            rows[i * 32 + 31].is_lead_end = true;
        }
        // Calls
        rows[31].call_str = Some("sB".to_owned());
        rows[63].call_str = Some("sB".to_owned());
        rows[223].call_str = Some("sH".to_owned());
        rows[255].call_str = Some("sH".to_owned());
        // Create the fragment and return
        Frag {
            rows,
            x: -100.0,
            y: -200.0,
        }
    }
}

#[wasm_bindgen]
pub fn reverse(s: String) -> String {
    s.chars().rev().skip(1).collect()
}
